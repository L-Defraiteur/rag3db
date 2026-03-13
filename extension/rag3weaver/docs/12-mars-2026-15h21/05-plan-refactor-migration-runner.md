# Plan : Refactor MigrationRunner → Catalog + auto-drain post-rollback

## Context

Le MigrationRunner accède directement à `conn: Arc<dyn DbConnection>` pour toute la logique DB (lock, record, execute, rollback). Si on veut s'abstraire du backend, c'est le Catalog qui doit posséder cette logique. Le runner devient un orchestrateur pur (scan fichiers, décision d'ordre, dry-run).

Bonus : après rollback, le Catalog peut auto-drain() les entités restaurées.

## Fichiers à modifier

| Fichier | Changement |
|---------|-----------|
| `src/catalog.rs` | Ajouter méthodes `pub(crate)` migration_* |
| `src/dataflow/migrations.rs` | Remplacer `conn` par `&Catalog` / `&mut Catalog` sur chaque méthode |
| `src/dataflow/mod.rs` | Exporter aussi `MigrationDirection`, `DryRunPlan`, `DryRunNode` si besoin |

## Nouvelles méthodes Catalog (pub(crate))

```rust
impl Catalog {
    // ── Migration support ──────────────────────────────────

    /// Ensure _DataflowMigration + _DataflowMigrationLock tables exist.
    pub(crate) async fn migration_initialize(&self) -> Result<(), CatalogError>;

    /// Load applied migrations from DB → HashMap<version, AppliedMigration>.
    pub(crate) async fn migration_load_applied(&self) -> Result<HashMap<u64, AppliedMigration>, CatalogError>;

    /// Acquire migration lock (TTL-based).
    pub(crate) async fn migration_acquire_lock(&self, lock_id: &str) -> Result<(), MigrationError>;

    /// Release migration lock.
    pub(crate) async fn migration_release_lock(&self) -> Result<(), MigrationError>;

    /// Record a migration result (MERGE + SET).
    pub(crate) async fn migration_record(&self, file: &MigrationFile, status: &str, direction: &str, execution_id: &str, duration_ms: u64, error: &str) -> Result<(), MigrationError>;

    /// Update migration status only.
    pub(crate) async fn migration_update_status(&self, version: u64, status: &str) -> Result<(), MigrationError>;

    /// Execute a migration graph with checkpoint.
    /// Sets up services (conn from self), checkpoint store, runtime.
    pub(crate) async fn migration_execute_graph(&self, graph: &mut DataflowGraph, execution_id: &str) -> Result<(), MigrationError>;

    /// Rollback: load checkpoint, undo nodes in reverse order.
    /// Then auto-drain if pending work exists.
    pub(crate) async fn migration_rollback_graph(&mut self, graph: &mut DataflowGraph, checkpoint: &ExecutionCheckpoint) -> Result<(), MigrationError>;
}
```

## Nouveau MigrationRunner (sans conn)

```rust
pub struct MigrationRunner {
    registry: Arc<NodeRegistry>,
    migration_dirs: Vec<PathBuf>,
    lock_id: String,
}

impl MigrationRunner {
    pub fn new(registry: Arc<NodeRegistry>) -> Self;
    pub fn add_dir(&mut self, dir: PathBuf);

    // Méthodes qui prennent le Catalog en paramètre
    pub async fn initialize(&self, catalog: &Catalog) -> Result<(), MigrationError>;
    pub async fn status(&self, catalog: &Catalog) -> Result<Vec<MigrationStatus>, MigrationError>;
    pub async fn pending(&self, catalog: &Catalog) -> Result<Vec<MigrationFile>, MigrationError>;
    pub async fn apply(&self, catalog: &mut Catalog, target: Option<u64>, dry_run: bool, vars: &HashMap<String, String>) -> Result<Vec<MigrationResult>, MigrationError>;
    pub async fn rollback(&self, catalog: &mut Catalog, version: u64) -> Result<MigrationResult, MigrationError>;
    pub async fn check_reversible(&self, vars: &HashMap<String, String>) -> Result<Vec<(MigrationFile, bool)>, MigrationError>;
}
```

Note : `check_reversible()` et `dry_run_migration()` ne touchent pas la DB (parse + validate seulement) → pas besoin de Catalog.

## Détail du refactor

### 1. Catalog — migration_initialize()
Reprend le code de `MigrationRunner::initialize()` tel quel, remplace `self.conn` par `self.conn.clone()` (déjà un Arc).

### 2. Catalog — migration_load_applied()
Reprend `MigrationRunner::load_applied()`. Rend `AppliedMigration` pub(crate) (le déplacer dans catalog.rs ou le laisser dans migrations.rs avec `pub(crate)`).

### 3. Catalog — migration_acquire_lock() / release_lock()
Reprend le code lock/unlock. Le `lock_id` est passé en paramètre (owned par le runner).

### 4. Catalog — migration_record() / migration_update_status()
Reprend tel quel.

### 5. Catalog — migration_execute_graph()
```rust
pub(crate) async fn migration_execute_graph(&self, graph: &mut DataflowGraph, execution_id: &str) -> Result<(), MigrationError> {
    let mut services = ServiceRegistry::new();
    services.register::<Arc<dyn DbConnection>>("conn", Arc::new(self.conn.clone()));
    let checkpoint_store = CypherCheckpointStore::new(self.conn.clone());
    checkpoint_store.initialize().await.map_err(|e| MigrationError::DbError(e))?;
    let runtime = DataflowRuntime::with_services(100, services);
    runtime.execute_with_checkpoint(graph, &checkpoint_store, execution_id)
        .await.map_err(|e| MigrationError::ExecutionError { ... })
}
```

### 6. Catalog — migration_rollback_graph() + auto-drain
```rust
pub(crate) async fn migration_rollback_graph(
    &mut self,
    graph: &mut DataflowGraph,
    checkpoint: &ExecutionCheckpoint,
) -> Result<(), MigrationError> {
    // Undo in reverse topological order (même logique qu'avant)
    let order = graph.topological_sort().map_err(...)?;
    let reversed: Vec<String> = order.into_iter().rev().collect();

    let mut services = ServiceRegistry::new();
    services.register::<Arc<dyn DbConnection>>("conn", Arc::new(self.conn.clone()));
    let services = Arc::new(services);

    for node_name in &reversed {
        // ... même code undo qu'avant ...
    }

    // ── Auto-drain ──
    if self.has_pending() {
        self.drain().await;
    }

    Ok(())
}
```

**Note sur auto-drain** : après undo de DeleteRecordNode, les entités sont restaurées en DB mais sans chunks/embeddings/FTS. Dans les tests e2e_undo, on re-ingest manuellement puis drain. Ici il faut que le rollback détecte les entités restaurées et les enqueue dans PendingWork.

L'undo_context de DeleteRecordNode contient `{ "EntityName": [{ _uuid, field1, ... }, ...] }`. Après le undo, on peut extraire ces UUIDs et les enqueuer comme creates dans le Catalog :

```rust
// Après chaque node undo, si le node est un DeleteRecordNode,
// extraire les entités restaurées et les enqueuer
if node.node_type() == "DeleteRecordNode" {
    if let Some(undo_ctx) = &undo_ctx {
        if let Some(groups) = undo_ctx.as_object() {
            for (entity_name, items) in groups {
                if let Some(arr) = items.as_array() {
                    for item in arr {
                        if let Some(props) = item.as_object() {
                            // Convertir JSON → BTreeMap<String, CypherValue>
                            // Appeler self.create(entity_name, uuid, data)
                        }
                    }
                }
            }
        }
    }
}
```

### 7. MigrationRunner — refactored

Chaque méthode appelle les méthodes Catalog correspondantes. Le runner ne stocke plus `conn`. Exemples :

```rust
pub async fn initialize(&self, catalog: &Catalog) -> Result<(), MigrationError> {
    catalog.migration_initialize().await
        .map_err(|e| MigrationError::DbError(e.to_string()))
}

pub async fn apply(&self, catalog: &mut Catalog, ...) -> Result<Vec<MigrationResult>, MigrationError> {
    let pending = self.pending(catalog).await?;
    // ...
    if !dry_run { catalog.migration_acquire_lock(&self.lock_id).await?; }
    for file in &pending {
        let result = if dry_run {
            self.dry_run_migration(file, vars)?  // pas de DB
        } else {
            self.apply_migration(catalog, file, vars).await?
        };
        // ...
    }
    if !dry_run { catalog.migration_release_lock().await.ok(); }
    Ok(results)
}
```

## Tests unitaires existants

Les 16 tests dans `migrations.rs` se répartissent en :
- **8 tests parsing** (parse_migration_filename, scan_migration_dir) → pas de DB, pas affectés
- **3 tests display** (MigrationState, MigrationDirection, MigrationError) → pas affectés
- **5 tests migration template** (internal_001_*) → utilisent `NodeRegistry` seulement, pas de DB

→ Aucun test ne crée de `MigrationRunner` avec `conn`. Le refactor ne casse rien.

## Vérification

```bash
# Unit tests
cargo test --lib --features "rag3db-native,candle-embedder"

# E2E complète
./run_e2e.sh --summary
```

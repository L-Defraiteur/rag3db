# Doc 06 — Refactor MigrationRunner → Catalog + auto-drain

Date : 12 mars 2026

## 1. Ce qui a été fait

### 1.1 MigrationRunner n'a plus de `conn`

**Avant** : `MigrationRunner` stockait `conn: Arc<dyn DbConnection>` et faisait toute la logique DB directement (lock, record, execute graph, rollback undo).

**Après** : `MigrationRunner` est un orchestrateur pur. Il scanne les fichiers, décide l'ordre, gère le dry-run. Toute interaction DB passe par le Catalog.

```rust
// Avant
pub struct MigrationRunner {
    conn: Arc<dyn DbConnection>,
    registry: Arc<NodeRegistry>,
    migration_dirs: Vec<PathBuf>,
    lock_id: String,
}
pub fn new(conn: Arc<dyn DbConnection>, registry: Arc<NodeRegistry>) -> Self;

// Après
pub struct MigrationRunner {
    registry: Arc<NodeRegistry>,
    migration_dirs: Vec<PathBuf>,
    lock_id: String,
}
pub fn new(registry: Arc<NodeRegistry>) -> Self;
```

Toutes les méthodes publiques prennent maintenant `&Catalog` ou `&mut Catalog` :
- `initialize(&self, catalog: &Catalog)`
- `status(&self, catalog: &Catalog)`
- `pending(&self, catalog: &Catalog)`
- `apply(&self, catalog: &mut Catalog, ...)`
- `rollback(&self, catalog: &mut Catalog, version)`
- `check_reversible(&self, catalog: &Catalog, vars)`

### 1.2 Nouvelles méthodes Catalog `pub(crate)`

9 méthodes ajoutées dans `src/catalog.rs` :

| Méthode | Rôle |
|---------|------|
| `migration_initialize()` | Crée tables _DataflowMigration + _DataflowMigrationLock |
| `migration_load_applied()` | Charge les migrations appliquées → HashMap |
| `migration_acquire_lock(lock_id)` | Lock TTL 10min |
| `migration_release_lock()` | Libère le lock |
| `migration_record(file, status, ...)` | MERGE + SET le résultat d'une migration |
| `migration_update_status(version, status)` | Update status seul |
| `migration_execute_graph(graph, execution_id)` | Setup services + checkpoint + runtime + execute |
| `migration_rollback_graph(graph, checkpoint)` | Undo reverse order + enqueue restored entities + auto-drain |
| `enqueue_restored_entities(undo_ctx)` | Parse undo context de DeleteRecordNode, re-enqueue dans PendingWork |

+ helper `json_to_cypher_value()` pour convertir serde_json → CypherValue.

### 1.3 Auto-drain post-rollback

Après le rollback, `migration_rollback_graph()` :
1. Exécute undo() sur chaque nœud en reverse topological order
2. Si un nœud est un `DeleteRecordNode`, extrait les entités restaurées du undo_context et les enqueue dans `pending.entities`
3. Appelle `self.drain()` si du pending work existe

Les entités restaurées sont re-enqueued avec leur `_uuid` original (l'EntityRef est résolue immédiatement). InsertRecordNode fait MERGE sur `_uuid` → pas de duplication. Le pipeline chunk/embed/FTS tourne normalement.

## 2. Fichiers modifiés

| Fichier | Changement |
|---------|-----------|
| `src/catalog.rs` | +9 méthodes `pub(crate)` migration_* + `enqueue_restored_entities()` + `json_to_cypher_value()` |
| `src/dataflow/migrations.rs` | MigrationRunner sans `conn`, toutes méthodes via Catalog, `AppliedMigration` pub(crate) |

## 3. Tests

- **544 unit tests passent**, 0 régression, 0 warning
- Les 16 tests unitaires de migrations.rs (parsing, scanning, display, template) ne touchent pas la DB → pas affectés
- Aucun code externe n'utilisait `MigrationRunner` (vérifié par grep)

## 4. Bug pré-existant : ambiguïté `dim()` sur BgeM3Embedder

**Fichier** : `src/bge_m3_embedder.rs:373`

`BgeM3Embedder` implémente à la fois `Embedder::dim()` et `DualEmbedder::dim()`. Le test `bge_m3_dense_basic()` appelle `embedder.dim()` sans qualifier le trait → erreur E0034 "multiple applicable items in scope".

Ce bug n'est visible que quand on compile les tests avec `--features bge-m3`. Il est `#[ignore]` donc n'affecte pas les CI runs normaux. Fix trivial : `Embedder::dim(&*embedder)` ou `<BgeM3Embedder as Embedder>::dim(&embedder)`.

**Pas lié à notre refactor** — pré-existant.

## 5. Pourquoi ce refactor

Le Catalog est la seule surface API. Si un jour on veut un backend pgvector/qdrant, c'est le Catalog qui s'adapte — pas le MigrationRunner. Le runner ne connaît plus le concept de connexion DB.

# Doc 11 — Plan Phase 2 : Search direct + SparseCommitNode

Date : 13 mars 2026

Ref : doc 09 (plan migration sparse), doc 10 (rapport Phase 1)

## Contexte

Phase 1 terminée (commit `dbcf494ca`). Le Catalog crée/ouvre les `SparseHandle` via BlobStore. Les 4 call sites `CREATE_SPARSE_VECTOR_INDEX` sont remplacés par `ensure_sparse_handle()`.

Phase 2 = deux tâches indépendantes :
- **#237** — Recherche directe via `handle.search()` (remplace `QUERY_SPARSE_VECTOR_INDEX`)
- **#238** — Noeud dataflow `SparseCommitNode` (commit explicite des handles dirty)

## Tâche #237 — Recherche sparse directe

### Situation actuelle

`search_sparse_cypher()` dans `search.rs` (ligne 2119) :
1. Sérialise `SparseVector` en Cypher string (`[indices]`, `[weights]`)
2. `CALL QUERY_SPARSE_VECTOR_INDEX('{entity}', [...], [...], limit)` → `(node_id, score)`
3. Résout les offsets → UUIDs via `resolve_and_enrich()`

### Cible

Appeler directement `handle.search(query, limit)` → `Vec<(u64, f32)>` (offsets, scores). Même format de sortie, même résolution ensuite.

### Changement de signature

```rust
// Avant
pub async fn search_sparse_cypher(
    conn: &dyn DbConnection,
    entity: &str,
    query_vector: &SparseVector,
    limit: usize,
    return_fields: &[String],
) -> Result<Vec<SearchResult>, CatalogError>

// Après
pub async fn search_sparse(
    handle: &sparse_vector::handle::SparseHandle,
    conn: &dyn DbConnection,       // pour resolve_and_enrich
    entity: &str,
    query_vector: &SparseVector,
    limit: usize,
    return_fields: &[String],
) -> Result<Vec<SearchResult>, CatalogError>
```

### Conversion de type

`rag3weaver::SparseVector` et `sparse_vector::SparseVector` ont la même structure (`indices: Vec<u32>`, `values: Vec<f32>`). Conversion triviale :

```rust
let sv = sparse_vector::index::SparseVector::new(
    query_vector.indices.clone(),
    query_vector.values.clone(),
);
let raw_results = handle.search(&sv, limit);
```

### Callers à adapter (2)

| Caller | Fichier | Ligne | Adaptation |
|--------|---------|-------|-----------|
| `Catalog::search()` | `catalog.rs` | ~2566 | `self.sparse_handle(vector_entity)` → passer le handle |
| `SparseSearchNode::execute()` | `generic_search_nodes.rs` | ~351 | Service `sparse_handles: HashMap<String, Arc<SparseHandle>>` |

### Gestion du cas sans handle

Si le handle n'existe pas (pas de sparse index pour cette entité), retourner `vec![]` — même comportement qu'actuellement quand l'extension n'a pas d'index.

## Tâche #238 — SparseCommitNode

### Pattern

Même architecture que `FlushNode` (FTS flush) :

```
FlushNode                          SparseCommitNode
─────────                          ────────────────
CALL FLUSH_LUCIVY_INDEX(table)     handle.commit()
service: conn (DbConnection)      service: sparse_handles (HashMap)
```

### Structure

```rust
pub struct SparseCommitNode {
    name: String,
    tables: Vec<String>,
    undo_data: Option<serde_json::Value>,
}
```

### Ports

| Port | Type | Direction | Required |
|------|------|-----------|----------|
| `trigger` | Empty | Input | Non |
| `done` | Empty | Output | Non |

### Service requis

`sparse_handles` : `HashMap<String, Arc<SparseHandle>>`

Le Catalog enregistre ce service dans le runtime dataflow au même moment qu'il enregistre `conn`.

### Execute

```rust
async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
    let handles = ctx.service::<HashMap<String, Arc<SparseHandle>>>("sparse_handles")
        .ok_or("SparseCommitNode: 'sparse_handles' service not registered")?;

    let mut committed = 0usize;
    for table in &self.tables {
        if let Some(handle) = handles.get(table) {
            handle.commit().map_err(|e| format!("commit {table}: {e}"))?;
            committed += 1;
        }
    }

    self.undo_data = Some(serde_json::json!(self.tables));
    ctx.log_metric("table_count", self.tables.len());
    ctx.log_metric("committed", committed);
    ctx.set_output("done", PortValue::Empty);
    Ok(())
}
```

### Undo

Re-commit (idempotent, pas d'effet si rien n'a changé).

### Factory

`SparseCommitNodeFactory` — même pattern que `FlushNodeFactory` :
- Config : `"table"` (string) ou `"tables"` (array)
- Enregistré dans `register_builtins()`

## Fichiers à modifier

| Fichier | Changement |
|---------|-----------|
| `src/search.rs` | Renommer `search_sparse_cypher` → `search_sparse`, remplacer Cypher par `handle.search()` |
| `src/catalog.rs` | Passer le sparse handle dans l'appel search |
| `src/dataflow/generic_search_nodes.rs` | Utiliser service `sparse_handles` pour `SparseSearchNode` |
| `src/dataflow/record_nodes.rs` | Ajouter `SparseCommitNode` |
| `src/dataflow/node_factories.rs` | Ajouter `SparseCommitNodeFactory` + register |
| `src/dataflow/mod.rs` | Export `SparseCommitNode` |

## Ordre d'implémentation

Les deux tâches sont indépendantes. Ordre suggéré :

1. **#238 SparseCommitNode** — auto-contenu, pas de changement aux fonctions existantes
2. **#237 search_sparse** — touche plus de fichiers, plus de risque de régression

## Risques

- **Service sparse_handles** : le Catalog doit enregistrer les handles comme service dans le runtime dataflow. Si les handles sont ajoutés après `initialize()` (via `register_entity()`), le service doit être mis à jour.
- **Commit sans insert** : appeler `commit()` sur un handle propre = no-op (pas de panic, pas d'erreur).
- **Type SparseVector** : la conversion est triviale mais nécessite un `clone()` des vecs. Acceptable pour un query vector (petit).

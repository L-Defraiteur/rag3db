# Doc 10 — Rapport de progression : migration sparse vers BlobStore

Date : 13 mars 2026

Ref : doc 08 (BlobStore complet), doc 09 (plan migration sparse)

## Résumé

Phase 1 (fondation) de la migration sparse terminée. Le Catalog crée et gère des `SparseHandle` via BlobStore au lieu d'appeler l'extension C++ `CREATE_SPARSE_VECTOR_INDEX`. Le trait BlobStore est unifié entre lucivy_core et sparse_vector.

## Travail réalisé

### 1. Trait SyncDbConnection

**Fichiers modifiés :**

| Fichier | Changement |
|---------|-----------|
| `rag3weaver/src/connection.rs` | Ajout trait `SyncDbConnection` avec `execute_sync()` et `execute_with_params_sync()` |
| `rag3weaver/src/rag3db_connection.rs` | `impl SyncDbConnection for Rag3dbConnection` (délègue à `query_sync` / `query_with_params_sync`) |
| `rag3weaver/src/lib.rs` | Export `SyncDbConnection` |

**Pourquoi** : `CypherBlobStore::from_connection()` utilisait `tokio::Handle::block_on()` dans un closure, ce qui panic si appelé depuis un runtime tokio (ce qui arrive dans `initialize()` et `drain()`). Le trait sync permet d'appeler directement les méthodes sync de `Rag3dbConnection` sans bridge async.

### 2. Refactor CypherBlobStore

| Fichier | Changement |
|---------|-----------|
| `rag3weaver/src/cypher_blob_store.rs` | `from_connection()` remplacé par `from_sync_connection(Arc<dyn SyncDbConnection>)` — plus de `block_on` |

### 3. Dépendance sparse-vector + unification BlobStore

| Fichier | Changement |
|---------|-----------|
| `rag3weaver/Cargo.toml` | Ajout `sparse-vector = { path = "../sparse_vector/rust" }` |
| `sparse_vector/rust/Cargo.toml` | Ajout `lucivy-core` dependency + `crate-type = ["lib", "staticlib"]` |
| `sparse_vector/rust/src/blob_store.rs` | Remplacé par re-export de `lucivy_core::blob_store::{BlobStore, MemBlobStore}` |
| `sparse_vector/rust/src/lib.rs` | `mod handle` → `pub mod handle` (nécessaire pour l'import depuis rag3weaver) |

**Résultat** : le trait `BlobStore` n'est plus dupliqué. `CypherBlobStore` implémente `lucivy_core::blob_store::BlobStore`, et sparse_vector re-exporte ce même trait.

### 4. SparseHandle dans le Catalog

| Fichier | Changement |
|---------|-----------|
| `rag3weaver/src/catalog.rs` | Ajout champs : `sparse_handles: HashMap<String, Arc<SparseHandle>>`, `cache_base: PathBuf`, `sync_conn: Option<Arc<dyn SyncDbConnection>>` |
| `rag3weaver/src/catalog.rs` | Ajout méthodes : `set_sync_connection()`, `set_cache_base()`, `sparse_handle()`, `ensure_sparse_handle()` |
| `rag3weaver/src/catalog.rs` | 4 call sites `CREATE_SPARSE_VECTOR_INDEX` → `ensure_sparse_handle()` |

**Call sites migrés :**
1. `initialize()` — KB sparse indexes
2. `create_entity_tables()` — simple entity sparse
3. `migrate_entity()` — migration add sparse signal
4. `create_kb_tables()` — KB creation

**`ensure_sparse_handle(table)`** : tente `open_with_store` d'abord (index existant dans BlobStore), sinon `create_with_store`. Stocke le handle dans `sparse_handles`.

### 5. Cache dir avec PID

| Fichier | Changement |
|---------|-----------|
| `sparse_vector/rust/src/handle.rs` | `create_with_store` et `open_with_store` prennent un `cache_base: &Path` supplémentaire |
| `sparse_vector/rust/src/handle.rs` | `make_cache_dir` : layout `{base}/{pid}/{index_name}_{seq}/` — PID isole les process, compteur atomique isole les threads |
| `sparse_vector/rust/src/handle.rs` | Ajout `BLOB_PREFIX = "Sparse_"` — préfixé automatiquement dans les clés BlobStore |

**Layout cache** : `{cache_base}/{pid}/{index_name}_{seq}/sparse.mmap` etc.
L'appelant fournit `cache_base`, la lib gère le reste. Même si deux DB utilisent le même `cache_base`, le PID + seq garantit l'unicité.

## Compteurs de tests

| Crate | Tests | Changement |
|-------|-------|-----------|
| rag3weaver | 543/543 | 0 failed |
| sparse_vector | 34/34 | -3 (tests blob_store locaux supprimés, remplacés par re-export lucivy_core) |

## Ce qui reste (tâches en cours)

### Phase 2 — Recherche (tâche #237)
- Remplacer `QUERY_SPARSE_VECTOR_INDEX` dans `search.rs` par `handle.search()` direct
- Convertir `rag3weaver::SparseVector` → `sparse_vector::index::SparseVector` aux call sites

### Phase 2 — SparseCommitNode (tâche #238)
- Noeud dataflow `SparseCommitNode` qui appelle `handle.commit()` sur les handles dirty
- Même pattern que `FlushNode` pour FTS

### Phase 3 — Insertion (tâche #239)
- EmbedNode appelle `handle.insert(node_id, vector)` après calcul des sparse embeddings
- Supprimer les colonnes `sparse_indices` / `sparse_weights` des chunk tables (single source of truth)

### Phase 4 — Cleanup
- Retirer tous les `CALL CREATE/QUERY_SPARSE_VECTOR_INDEX`
- Retirer la dépendance sur l'extension C++ sparse pour rag3weaver

### Question ouverte — Préfixe FTS
Quand lucivy sera migré vers BlobStore, adopter le même pattern de préfixe (`"Lucivy_"` ou `"FTS_"`) et le même layout cache (`{cache_base}/{pid}/{name}_{seq}/`). À investiguer dans `lucivy_core/src/blob_directory.rs`.

## Architecture actuelle après Phase 1

```
Catalog
  ├── conn: Arc<dyn DbConnection>           (async, pour Cypher DDL/DML)
  ├── sync_conn: Arc<dyn SyncDbConnection>   (sync, pour BlobStore)
  ├── blob_store: Arc<CypherBlobStore>        (_index_blobs table)
  ├── sparse_handles: HashMap<String, Arc<SparseHandle>>
  │     clé = nom de table (ex: "Product_Chunk")
  │     BlobStore key = "Sparse_Product_Chunk/{file}"
  │
  ├── register_entity()
  │     → create_entity_tables() (DDL tables)
  │     → ensure_sparse_handle() si sparse signal
  │
  ├── register_kb()
  │     → create_kb_tables() (DDL tables)
  │     → ensure_sparse_handle() si sparse signal
  │
  └── initialize()
        → ensure_table() pour _index_blobs (via conn async)
        → CypherBlobStore::from_sync_connection()
        → ensure_sparse_handle() pour chaque KB sparse existant
```

## Fichiers non committés (depuis dernier commit 8f5220023)

### rag3weaver
- `src/catalog.rs` (modifié — sparse_handles, sync_conn, cache_base, ensure_sparse_handle)
- `src/connection.rs` (modifié — trait SyncDbConnection)
- `src/cypher_blob_store.rs` (modifié — from_sync_connection)
- `src/rag3db_connection.rs` (modifié — impl SyncDbConnection)
- `src/lib.rs` (modifié — export SyncDbConnection)
- `Cargo.toml` (modifié — dep sparse-vector)
- `Cargo.lock` (modifié)
- `docs/13-mars-2026-18h45/09-plan-migration-sparse-vers-blob-store.md` (nouveau)
- `docs/13-mars-2026-18h45/10-rapport-progression-migration-sparse.md` (ce fichier)

### sparse_vector
- `rust/Cargo.toml` (modifié — dep lucivy-core, crate-type lib+staticlib)
- `rust/src/blob_store.rs` (modifié — re-export lucivy_core)
- `rust/src/handle.rs` (modifié — cache_base param, PID, BLOB_PREFIX)
- `rust/src/lib.rs` (modifié — pub mod handle)

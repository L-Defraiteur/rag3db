# Doc 04 — Knowledge dump : extension sparse_vector

Date : 24 mars 2026

## Structure

```
packages/rag3db/extension/sparse_vector/
├── rust/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── handle.rs           # SparseHandle (create/open/insert/search/commit)
│       ├── index.rs            # SparseIndex + SparseVector (in-memory posting lists)
│       ├── mmap_index.rs       # MmapPostingData (flat binary format, WAND pruning)
│       ├── bridge.rs           # cxx bridge (C++ ↔ Rust)
│       └── blob_store.rs       # Re-export: lucivy_core::blob_store::{BlobStore, MemBlobStore}
├── src/                        # C++ extension code
│   ├── main/                   # sparse_vector_extension.cpp (LOAD EXTENSION)
│   ├── index/                  # sparse_vector_index.cpp (CREATE/QUERY/DROP)
│   └── function/               # CREATE_SPARSE_VECTOR_INDEX, QUERY_SPARSE_VECTOR_INDEX
└── CMakeLists.txt
```

## SparseHandle

Le composant principal. Gère un index sparse avec persistance via BlobStore.

```rust
pub struct SparseHandle {
    inner: Mutex<SparseInner>,
    backend: StorageBackend,  // Filesystem ou Store (BlobStore)
    path: PathBuf,            // Cache local mmap
}

enum StorageBackend {
    Filesystem,                                  // StdFsDirectory, données sur disque
    Store { store: Arc<dyn BlobStore>, index_name: String },  // BlobStore, mmap cache temp
}
```

### API

```rust
// Création
SparseHandle::create_with_store(store, index_name, cache_base) -> Result<SparseHandle>
SparseHandle::open_with_store(store, index_name, cache_base) -> Result<SparseHandle>

// Insertion
handle.insert(node_id: u64, vector: &SparseVector) -> Result<()>

// Recherche (WAND pruning)
handle.search(query: &SparseVector, limit: usize) -> Vec<(u64, f32)>  // (offset, score)

// Persistance
handle.commit_inner() -> Result<()>  // Write mmap + sync to BlobStore

// Info
handle.len() -> usize
handle.is_empty() -> bool
```

### Format mmap (flat binary)

```
[FileHeader]                    16 bytes (magic, version, num_dims, num_vectors)
[DimHeader × num_dims]          16 bytes × N (dim_id, offset, count, max_weight)
[PostingEntry × total_entries]  16 bytes × M (doc_id: u64, weight: f32, padding)
```

- WAND pruning utilise `max_weight` dans DimHeader pour skip les dimensions non-prometteuses
- Posting lists triées par doc_id dans chaque dimension
- Lazy loading : mmap n'est ouvert qu'au premier search (pas au insert)

### BlobStore

Le `SparseHandle` persiste via un `BlobStore` abstrait :
- **CypherBlobStore** : table `_index_blobs` dans rag3db (via SyncDbConnection)
- **PostgresBlobStore** : table `rag3weaver._index_blobs` (BYTEA)
- **MemBlobStore** : in-memory (tests, DBs in-memory)

Fichiers persistés par index :
```
Sparse_{table}/sparse.mmap     # posting lists mmap
Sparse_{table}/vectors.bin     # vecteurs originaux (bincode)
Sparse_{table}/dims.bin        # dim mapping (bincode)
```

### Dirty flag + lazy commit

- `insert()` → `dirty = true`
- `search()` → si `dirty`, flush d'abord (`commit_inner()`)
- Le commit écrit les 3 fichiers + sync vers BlobStore si store-backed
- Après commit : re-mmap pour les lectures

## Migration sparse (complète)

Historique des phases de migration (session 13-15 mars) :

### Phase 1 — SparseHandle + BlobStore foundation
- `SparseHandle::create_with_store` / `open_with_store`
- `CypherBlobStore` pour rag3db, `MemBlobStore` fallback
- `SyncDbConnection` trait (évite le block_on dans le BlobStore sync)

### Phase 2 — Search direct
- `search_sparse()` dans search.rs : `handle.search()` direct au lieu de `QUERY_SPARSE_VECTOR_INDEX`
- `SparseCommitNode` dans dataflow (même pattern que FlushNode)
- Résolution offsets → UUIDs via `OFFSET(id(n))` Cypher

### Phase 3 — Insertion directe
- `EmbedNode` / `KBEmbedNode` : `handle.insert(offset, sv)` au lieu d'écrire dans les colonnes
- Colonnes `sparse_indices` / `sparse_weights` supprimées du DDL
- Les anciennes colonnes restent comme orphelines (ignorées)

## Intégration dans le Catalog

```rust
// Dans Catalog struct :
sparse_handles: HashMap<String, Arc<SparseHandle>>,  // clé = table name (e.g. "kb_Index_Chunk")
blob_store: Option<Arc<dyn BlobStore>>,               // CypherBlobStore ou MemBlobStore

// Dans initialize() :
// 1. blob_store initialisé (CypherBlobStore si sync_conn, sinon MemBlobStore)
// 2. ensure_sparse_handle() pour chaque KB avec sparse signal

// Services registered pour les nodes :
services.register::<HashMap<String, Arc<SparseHandle>>>("sparse_handles", ...);
```

### ensure_sparse_handle()

```rust
fn ensure_sparse_handle(&mut self, table: &str) {
    if self.sparse_handles.contains_key(table) { return; }
    let Some(ref blob_store) = self.blob_store else { return };
    // Try open first (index may exist), fall back to create
    let handle = SparseHandle::open_with_store(blob_store, table, &self.cache_base)
        .or_else(|_| SparseHandle::create_with_store(blob_store, table, &self.cache_base))?;
    self.sparse_handles.insert(table.to_string(), Arc::new(handle));
}
```

### Shutdown

```rust
// Dans Catalog::shutdown() :
for (table, handle) in self.sparse_handles.drain() {
    handle.commit_inner()?;  // flush + persist
}
```

## Offset mechanism

Le sparse index utilise des `u64` node offsets comme identifiants de documents :
- **rag3db** : `OFFSET(id(n))` — offset interne stable (pas de compaction)
- **PostgreSQL** : `_row_id BIGSERIAL` — auto-increment, jamais réutilisé après DELETE

L'insertion sparse fait :
```rust
// Dans EmbedNode :
let cypher = dialect.embed_set_hash_returning_offset(entity_name);
// → RETURN item.uuid, OFFSET(id(n)) AS offset
// → ou RETURNING v.uuid, table._row_id AS offset
for (uuid, offset) in results {
    handle.insert(offset as u64, &sparse_vector)?;
}
```

## WAND pruning

L'algorithme de search :
1. Pour chaque dimension du query vector, ouvrir le posting list
2. Calculer l'upper-bound score de chaque document (somme des max_weight des dimensions matchées)
3. Élaguer les documents dont l'upper-bound < score du K-ème meilleur résultat actuel
4. Pour les documents survivants, calculer le score exact (dot product)
5. Maintenir un heap de taille K

Performance : sub-linéaire en nombre de documents, O(K × dimensions_query) dans le meilleur cas.

## Prochaines étapes sparse

### Segments WORM (doc `lucivy docs/24-mars-20h35/07`)
- Au lieu d'un seul `sparse.mmap`, segments immutables
- Commit = nouveau segment UUID
- Search = fan-out par segment + merge top-K
- Delete = bitset par segment
- Merge background = combiner N segments en 1
- Incremental sync = delta (segments ajoutés/supprimés)

### Sharding (doc `lucivy docs/16-mars/14-questions` + réponses)
- `ShardRouter` de lucivy_core réutilisable directement (u64 term_ids)
- `ShardedSparseHandle` = N `SparseHandle` + router (~150 lignes)
- Heap merge top-K cross-shard (20 lignes)
- Parallélisable via luciole `fan_out_merge`

## Tests

Les tests sparse sont dans les E2E search :
```bash
./run_e2e.sh --test e2e_search phase3 --summary    # sparse search
./run_e2e.sh --test e2e_search phase4 --summary    # signal combos (sparse_only, all_three, etc.)
./run_e2e.sh --test e2e_search phase5 --summary    # dual embedder
./run_e2e.sh --test e2e_search phase6_sparse_mmap   # persistence roundtrip (BlobStore)
```

Tests unitaires sparse_vector :
```bash
cd packages/rag3db/extension/sparse_vector/rust
cargo test
```

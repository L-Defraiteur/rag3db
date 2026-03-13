# Doc 05 — Rapport : implémentation BlobStore + BlobDirectory dans lucivy_core

Date : 13 mars 2026

Ref : doc 04 (findings blob store)

## Résumé

Implémentation du trait `BlobStore` et de `BlobDirectory` dans `lucivy_core/`, permettant à lucivy de persister ses index dans un backend externe (DB, S3, mémoire) tout en gardant les performances mmap natives via un cache local.

## Fichiers créés/modifiés

### Nouveaux fichiers (lucivy_core/src/)

| Fichier | Contenu |
|---------|---------|
| `blob_store.rs` | Trait `BlobStore` (load/save/delete/exists/list) + `MemBlobStore` (impl in-memory pour tests) |
| `blob_directory.rs` | `BlobDirectory<S: BlobStore>` — impl `Directory` avec cache local mmap |

### Fichiers modifiés

| Fichier | Changement |
|---------|-----------|
| `lucivy_core/src/lib.rs` | Ajout `pub mod blob_store; pub mod blob_directory;` |
| `lucivy_core/src/query.rs` | Fix tests `test_filter_clause_contains` et `test_filter_clause_contains_with_fuzzy` — ngram pairs manquants dans le schema de test |

## Architecture BlobDirectory

### Principe : materialise + mmap

```
BlobStore (DB/S3/mémoire)     Cache local (tmpdir)
  source of truth               perf mmap native
       │                              │
       │  open(): list + load         │
       ├─────────────────────────────→│  fichiers matérialisés sur disque
       │                              │  StdFsDirectory dessus
       │                              │
       │  atomic_write/open_write:    │
       │←─────────────────────────────┤  écrit dans cache ET dans store
       │                              │
       │  delete:                     │
       │←─────────────────────────────┤  supprime du cache ET du store
       │                              │
       │  open_read/get_file_handle:  │
       │                              ├→ lecture depuis cache (mmap natif)
       │                              │
       │  drop (dernier ref):         │
       │                              ├→ cleanup cache_dir
```

### Cycle de vie

1. **`BlobDirectory::new(store, index_name)`** :
   - Crée un tmpdir unique : `/tmp/lucivy_blob_cache/{index_name}_{seq}/`
   - `store.list()` + `store.load()` → écrit chaque fichier dans le tmpdir
   - `StdFsDirectory::open(tmpdir)` → mmap-capable

2. **Écriture** (via IndexWriter) :
   - `open_write(path)` → `BlobWriter` buffer en RAM → flush écrit dans cache_dir ET `store.save()`
   - `atomic_write(path, data)` → écrit dans cache (via inner StdFsDirectory) ET `store.save()`
   - WORM respecté : `open_write` refuse si fichier existe déjà dans cache

3. **Lecture** (via IndexReader/Searcher) :
   - `open_read` / `get_file_handle` → délègue à `StdFsDirectory` → lecture fichier local (mmap)

4. **Suppression** (GC après merge) :
   - `delete(path)` → supprime du cache ET `store.delete()`

5. **Drop** :
   - Cleanup cache_dir uniquement quand `Arc::strong_count(cache_dir) == 1`

### Bug trouvé et corrigé : Drop prématuré

Le trait `Directory` de lucivy a une méthode `acquire_lock` par défaut qui clone le directory via `box_clone()`. Le `DirectoryLockGuard` stocke un autre clone. Ces clones partagent le même `cache_dir`.

**Problème initial** : `cache_dir: PathBuf` → le `Drop` du premier clone supprimait le cache, cassant tous les autres clones → `meta.json` FileDoesNotExist.

**Fix** : `cache_dir: Arc<PathBuf>` → `Drop` ne nettoie que quand `strong_count == 1` (dernier clone).

## Trait BlobStore

```rust
pub trait BlobStore: Send + Sync + 'static {
    fn load(&self, index_name: &str, file_name: &str) -> io::Result<Vec<u8>>;
    fn save(&self, index_name: &str, file_name: &str, data: &[u8]) -> io::Result<()>;
    fn delete(&self, index_name: &str, file_name: &str) -> io::Result<()>;
    fn exists(&self, index_name: &str, file_name: &str) -> io::Result<bool>;
    fn list(&self, index_name: &str) -> io::Result<Vec<String>>;
}
```

- `index_name` : identifie l'index (ex: "Product", "Article_Index")
- `file_name` : identifie le fichier dans l'index (ex: "meta.json", "{uuid}.idx")
- `MemBlobStore` : `HashMap<String, HashMap<String, Vec<u8>>>` derrière `Arc<RwLock<...>>`

Implémentations futures prévues :
- `CypherBlobStore` : persistence dans `_index_blobs` table rag3db (dans rag3weaver)
- `PostgresBlobStore` : persistence Postgres bytea
- `S3BlobStore` : S3-compatible object storage

## Fix tests query.rs (pré-existant, pas lié au blob store)

`make_filter_index()` créait un schema sans champs ngram. Les tests `contains` échouaient car `build_contains_query` exige un ngram field pour la recherche trigram.

Fix : enrichi le schema de test avec `name._raw` + `name._ngram` + enregistrement tokenizer ngram. Documenté dans `ld-lucivy/docs/13-mars-2026-16h47/01-fix-filter-contains-tests-ngram-mismatch.md`.

## Tests

### blob_store (3 tests)
- `test_mem_blob_store_roundtrip` — save/load/exists/delete/list
- `test_mem_blob_store_multiple_indexes` — isolation entre index
- `test_mem_blob_store_overwrite` — overwrite existant

### blob_directory (7 tests)
- `test_blob_directory_create_and_search` — create + insert + search, vérifie sync vers store
- `test_blob_directory_create_close_reopen` — close + drop cache → reopen depuis store
- `test_blob_directory_worm_semantics` — write-once, delete, re-write
- `test_blob_directory_search_after_reopen` — search "rust" après close/reopen
- `test_blob_directory_multiple_indexes_isolated` — 2 index sur même store, counts différents
- `test_blob_directory_survives_cache_cleanup` — data survit après suppression cache_dir
- `test_blob_directory_multiple_commits` — 5 batches × 10 docs, multi-segments, close/reopen

### Résultat total lucivy_core : 71/71 passed

## Prochaines étapes

1. **Sparse vector** : ajouter trait `SparseStore` (ou réutiliser `BlobStore`) dans sparse_vector/rust/, refactorer le handle pour abstraire le storage
2. **CypherBlobStore** : impl `BlobStore` dans rag3weaver via requêtes Cypher sur `_index_blobs`
3. **Bridge C++** : permettre de passer un `BlobStore` au create/open via le bridge cxx lucivy_fts
4. **Tests E2E** : persistence roundtrip via CypherBlobStore dans rag3weaver

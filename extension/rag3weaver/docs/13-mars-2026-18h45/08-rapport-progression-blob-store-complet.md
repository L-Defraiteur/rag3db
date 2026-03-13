# Doc 08 — Rapport de progression : BlobStore complet (lucivy + sparse + CypherBlobStore)

Date : 13 mars 2026

Ref : doc 04 (findings), doc 05 (impl BlobDirectory), doc 06 (findings CypherBlobStore), doc 07 (plan CypherBlobStore)

## Résumé

L'abstraction BlobStore est implémentée de bout en bout : du trait générique dans les crates Rust (lucivy_core, sparse_vector) jusqu'au backend rag3db (CypherBlobStore dans rag3weaver). Le pattern "DB stocke, mmap sert" est câblé et testé à chaque niveau.

## Travail réalisé — 3 sessions

### Session 1 : Exploration + BlobDirectory lucivy (doc 04, 05)

**Fichiers créés :**

| Fichier | Contenu | Tests |
|---------|---------|-------|
| `lucivy_core/src/blob_store.rs` | Trait `BlobStore` (load/save/delete/exists/list) + `MemBlobStore` | 3 |
| `lucivy_core/src/blob_directory.rs` | `BlobDirectory<S: BlobStore>` impl `Directory` avec cache tmpdir mmap | 7 |

**Fichiers modifiés :**

| Fichier | Changement |
|---------|-----------|
| `lucivy_core/src/lib.rs` | Ajout `pub mod blob_store; pub mod blob_directory;` |
| `lucivy_core/src/query.rs` | Fix tests contains — ngram pairs manquants dans schema de test |

**Bug trouvé et corrigé** : `cache_dir: PathBuf` → `cache_dir: Arc<PathBuf>`. Le trait `Directory` clone via `box_clone()` dans `acquire_lock()`. Sans Arc, le `Drop` du premier clone supprimait le cache partagé → `meta.json FileDoesNotExist`. Fix : cleanup uniquement quand `Arc::strong_count == 1`.

**Résultat** : 71/71 tests lucivy_core, 1096/1096 tests ld-lucivy complet.

### Session 2 : BlobStore sparse_vector

**Fichiers créés :**

| Fichier | Contenu | Tests |
|---------|---------|-------|
| `sparse_vector/rust/src/blob_store.rs` | Trait `BlobStore` (copie identique) + `MemBlobStore` | 3 |

**Fichiers modifiés :**

| Fichier | Changement |
|---------|-----------|
| `sparse_vector/rust/src/lib.rs` | Ajout `pub mod blob_store;` |
| `sparse_vector/rust/src/handle.rs` | Refactor complet — ajout `StorageBackend` enum, `create_with_store`, `open_with_store`, sync vers BlobStore dans `commit_inner`, `Drop` cleanup tmpdir | +7 tests |

**Design** :
- `StorageBackend::Filesystem` : comportement identique à avant (zéro breaking change)
- `StorageBackend::Store { store, index_name }` : tmpdir cache + BlobStore persistence
- Au commit : écriture locale → read-back → `store.save()` pour les 3 fichiers
- Au drop : `remove_dir_all(cache_dir)` seulement pour les handles store-backed

**Résultat** : 37/37 tests sparse_vector (6 filesystem existants + 7 BlobStore nouveaux + 24 autres).

### Session 3 : CypherValue::Blob + CypherBlobStore

**Étape 1 — Support BLOB natif dans rag3weaver :**

| Fichier | Changement |
|---------|-----------|
| `rag3weaver/src/connection.rs` | Ajout `CypherValue::Blob(Vec<u8>)` (avec `#[serde(skip)]`), `as_blob()`, `From<Vec<u8>>` |
| `rag3weaver/src/rag3db_connection.rs` | Mapping `rag3db::Value::Blob` ↔ `CypherValue::Blob` (2 directions) |
| `rag3weaver/src/dataflow/migration_nodes.rs` | Match exhaustif `CypherValue::Blob` → `"<blob>"` en JSON |
| `rag3weaver/src/search.rs` | Match exhaustif `CypherValue::Blob` → `"<blob>"` en littéral Cypher |

Note : `#[serde(skip)]` sur `Blob` pour éviter un conflit avec `#[serde(untagged)]` — un `[1]` JSON se désérialisait comme `Blob([1])` au lieu de `List([Int(1)])`.

**Étape 2 — CypherBlobStore :**

| Fichier | Contenu | Tests |
|---------|---------|-------|
| `rag3weaver/src/cypher_blob_store.rs` | `CypherBlobStore` struct, `impl BlobStore`, `ensure_table()`, `from_connection()` | 6 |
| `rag3weaver/Cargo.toml` | Ajout dépendance `lucivy-core` (pour le trait BlobStore) |
| `rag3weaver/src/lib.rs` | Ajout `pub mod cypher_blob_store;` |

**Design CypherBlobStore** :
- Closure sync `QueryFn` pour contourner le mismatch sync/async (BlobStore est sync, DbConnection est async)
- `from_connection(Arc<dyn DbConnection>)` : wrappe via `tokio::runtime::Handle::current().block_on()` — safe car `Rag3dbConnection` est sync en interne
- Table `_index_blobs (_key STRING, _data BLOB, PRIMARY KEY(_key))`
- Clé : `"{index_name}/{file_name}"` (ex: `"Product_fts/meta.json"`)
- BLOB natif rag3db — zéro overhead base64
- `ensure_table()` crée la table si absente

**Résultat** : 543/543 tests rag3weaver (dont 6 CypherBlobStore avec mock query function).

## Architecture finale

```
┌─────────────────────────────────────────────────────────┐
│                     rag3weaver                          │
│                                                         │
│  CypherBlobStore ──impl──→ lucivy_core::BlobStore trait │
│       │                                                 │
│       │ Cypher MERGE/MATCH/DELETE sur _index_blobs      │
│       │ avec CypherValue::Blob (BLOB natif)             │
│       ▼                                                 │
│  ┌──────────┐                                           │
│  │  rag3db  │  _index_blobs (_key STRING, _data BLOB)   │
│  └──────────┘                                           │
└─────────────────────────────────────────────────────────┘
        │
        │ store.load() / store.save()
        ▼
┌───────────────────────────────────┐  ┌──────────────────────────────┐
│        lucivy_core                │  │      sparse_vector           │
│                                   │  │                              │
│  BlobDirectory<CypherBlobStore>   │  │  SparseHandle::*_with_store  │
│    │                              │  │    │                         │
│    │ open: store→tmpdir→mmap      │  │    │ open: store→tmpdir→mmap │
│    │ write: tmpdir + store.save   │  │    │ commit: tmpdir + store  │
│    │ read: mmap (zero-copy)       │  │    │ search: mmap (zero-copy)│
│    │ drop: cleanup tmpdir         │  │    │ drop: cleanup tmpdir    │
│    ▼                              │  │    ▼                         │
│  StdFsDirectory (mmap natif)      │  │  MmapPostingData (mmap natif)|
└───────────────────────────────────┘  └──────────────────────────────┘
```

## Compteurs de tests

| Crate | Tests total | Dont BlobStore |
|-------|------------|----------------|
| lucivy_core | 71 | 10 (3 blob_store + 7 blob_directory) |
| sparse_vector | 37 | 10 (3 blob_store + 7 handle blob) |
| rag3weaver | 543 | 6 (cypher_blob_store) |
| **Total** | **651** | **26** |

## Ce qui reste (non implémenté)

### Priorité 1 — Intégration Catalog

1. `Catalog::initialize()` : appeler `ensure_table()` pour créer `_index_blobs`
2. `Catalog` expose `blob_store()` → `Arc<CypherBlobStore>`
3. Migrer les index FTS : au lieu de `CALL CREATE_LUCIVY_INDEX(...)`, utiliser `BlobDirectory + CypherBlobStore` + lucivy_core directement en Rust
4. Migrer les index sparse : au lieu de `CALL CREATE_SPARSE_VECTOR_INDEX(...)`, utiliser `SparseHandle::create_with_store(CypherBlobStore)`
5. Migrer `shutdown()` : plus besoin de `CALL CLOSE_LUCIVY_INDEX(...)` (le Drop de BlobDirectory nettoie le cache)

### Priorité 2 — Tests E2E

1. CypherBlobStore roundtrip avec `Rag3dbConnection::in_memory()` (tests `#[ignore]`)
2. BlobDirectory + CypherBlobStore : create → insert → search → close → reopen → search
3. SparseHandle + CypherBlobStore : create → insert → commit → close → reopen → search
4. Full pipeline : register_entity → ingest → shutdown → reopen → search

### Priorité 3 — Unification trait

Le trait `BlobStore` est copié identiquement dans lucivy_core et sparse_vector. Options :
- sparse_vector dépend de lucivy_core (simple)
- Crate partagé `blob-store` (propre mais nouveau crate)

### Non prioritaire

- Compression zstd des blobs avant stockage (pour les gros index production)
- GC des blobs orphelins (crash entre écriture segment et meta.json)
- CypherBlobStore pour WASM (pas utile — WASM utilise MemoryDirectory)
- Lock distribué pour multi-instance

## Fichiers non committés

Tout le travail BlobStore n'est pas encore committé. Liste des fichiers modifiés/créés :

### lucivy_core (ld-lucivy submodule)
- `src/blob_store.rs` (nouveau)
- `src/blob_directory.rs` (nouveau)
- `src/lib.rs` (modifié)
- `src/query.rs` (modifié — fix ngram tests)

### sparse_vector
- `rust/src/blob_store.rs` (nouveau)
- `rust/src/lib.rs` (modifié)
- `rust/src/handle.rs` (modifié — refactor StorageBackend)

### rag3weaver
- `src/cypher_blob_store.rs` (nouveau)
- `src/connection.rs` (modifié — CypherValue::Blob)
- `src/rag3db_connection.rs` (modifié — mapping Blob)
- `src/dataflow/migration_nodes.rs` (modifié — match exhaustif)
- `src/search.rs` (modifié — match exhaustif)
- `src/lib.rs` (modifié)
- `Cargo.toml` (modifié — dépendance lucivy-core)

### docs (rag3weaver/docs/13-mars-2026-18h45/)
- `04-findings-blob-store-snapshot-mmap.md`
- `05-rapport-blob-store-blob-directory-impl.md`
- `06-findings-cypher-blob-store.md`
- `07-plan-cypher-blob-store.md`
- `08-rapport-progression-blob-store-complet.md` (ce fichier)

### docs (ld-lucivy/docs/)
- `13-mars-2026-16h47/01-fix-filter-contains-tests-ngram-mismatch.md`

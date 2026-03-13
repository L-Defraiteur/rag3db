# Doc 04 — Findings : snapshot, mmap & blob store pour lucivy et sparse

Date : 13 mars 2026

Ref : doc 02 (prio 1 IndexBlobStore), doc 20 (architecture composite mmap+DB)

## Objectif

Avant d'implémenter quoi que ce soit, comprendre les mécanismes internes de lucivy (Tantivy fork) et sparse vector au niveau Rust pour designer la bonne abstraction. L'abstraction doit vivre **côté Rust** (pas dans les extensions C++ rag3db ni dans rag3weaver).

---

## 1. Lucivy (Tantivy fork) — trait `Directory` existant

### 1a. Le trait Directory

Lucivy a **déjà** un trait d'abstraction storage hérité de ses origines : `Directory` (WORM — Write Once Read Many).

**Fichier** : `ld-lucivy/src/directory/directory.rs`

```rust
pub trait Directory: DirectoryClone + fmt::Debug + Send + Sync + 'static {
    // Lecture
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError>;
    fn open_read(&self, path: &Path) -> Result<FileSlice, OpenReadError>;
    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError>;

    // Écriture (WORM : file must not exist)
    fn open_write(&self, path: &Path) -> Result<WritePtr, OpenWriteError>;
    fn atomic_write(&self, path: &Path, data: &[u8]) -> io::Result<()>;

    // Gestion fichiers
    fn delete(&self, path: &Path) -> Result<(), DeleteError>;
    fn exists(&self, path: &Path) -> Result<bool, OpenReadError>;

    // Durabilité & concurrence
    fn sync_directory(&self) -> io::Result<()>;
    fn acquire_lock(&self, lock: &Lock) -> Result<DirectoryLock, LockError>;
    fn watch(&self, watch_callback: WatchCallback) -> Result<WatchHandle>;
}
```

### 1b. Implémentations existantes

| Implémentation | Backend | Usage |
|----------------|---------|-------|
| `MmapDirectory` | Filesystem + mmap LRU cache | Production |
| `RamDirectory` | `HashMap<PathBuf, FileSlice>` | Tests unitaires |
| `StdFsDirectory` | `std::fs` (compatible Emscripten) | WASM |
| `ManagedDirectory` | Wrapper — tracking fichiers pour GC | Décorateur |

### 1c. Mécanisme de commit

```
IndexWriter::commit()
  → flush workers → PreparedCommit
  → SegmentUpdater::schedule_commit()
  → save_metas():
      1. sync_directory()           ← durabilité des segments
      2. atomic_write("meta.json")  ← point de commit atomique
```

**`meta.json`** est le seul point de commit. Contient : `{ segments: [SegmentMeta...], schema, opstamp, payload }`.

### 1d. Fichiers d'un index lucivy

Chaque segment = UUID + 9 composants :

| Composant | Extension | Contenu |
|-----------|-----------|---------|
| Postings | `.idx` | Listes inversées |
| Positions | `.pos` | Positions des termes |
| FastFields | `.fast` | Colonnes (filter fields) |
| FieldNorms | `.fieldnorm` | Normes BM25 |
| Terms | `.term` | Dictionnaire de termes |
| Store | `.store` | Stockage documents row-oriented |
| TempStore | `.store.temp` | Temporaire pendant merge |
| Delete | `.{opstamp}.del` | Bitset documents vivants |
| Offsets | `.offsets` | Offsets pour highlights |

**Fichiers globaux** : `meta.json`, `.managed.json`, `.lock`, `.meta.lock`

**Cycle de vie** : segments créés par les writers, supprimés après merge par le GC (`ManagedDirectory.garbage_collect()`). Le nombre de segments et fichiers est **dynamique**.

### 1e. Conclusion lucivy

**On n'a PAS besoin d'un trait `IndexBlobStore` externe pour lucivy.** Le trait `Directory` existe déjà et fait exactement ce job. La stratégie est d'**implémenter un nouveau `Directory`** qui stocke ses blobs dans un store externe (DB, S3, etc.) au lieu du filesystem.

---

## 2. Sparse vector — aucune abstraction

### 2a. Format de stockage

3 fichiers fixes, réécrits entièrement à chaque commit :

| Fichier | Format | Contenu | Taille typique |
|---------|--------|---------|----------------|
| `sparse.mmap` | Binary flat (`#[repr(C)]`) | FileHeader + DimHeaders + PostingEntries | Gros (N docs × M dims × 16 bytes) |
| `sparse_vectors.bin` | Bincode | `HashMap<u64, SparseVector>` (id → vecteur) | Moyen |
| `sparse_dims.bin` | Bincode | `(HashMap<u32, usize>, Vec<u32>)` dim mapping | Petit |

### 2b. Layout binaire de `sparse.mmap`

```
┌─────────────────────────────────┐
│ FileHeader (16 bytes)           │  magic=0x53505253, version=1, num_dims, num_vectors
├─────────────────────────────────┤
│ DimHeader[0..num_dims] (16B ea) │  offset (u64) + count (u32) + pad (u32)
├─────────────────────────────────┤
│ PostingEntry[] (16B each)       │  record_id (u64) + weight (f32) + max_next_weight (f32)
└─────────────────────────────────┘
```

### 2c. Mécanisme mmap + lazy loading

```
open()
  → mmap sparse.mmap (read-only)
  → charger dims (petit fichier)
  → postings NON chargées en RAM (postings_loaded=false)
  → vectors NON chargées (vectors_loaded=false)

search() [dirty=false]
  → search_mmap() directement sur le mmap (zero-copy, zero-alloc posting iters)

insert/delete()
  → ensure_postings_loaded() : matérialiser posting lists depuis mmap
  → ensure_vectors_loaded() : désérialiser HashMap
  → mutation en RAM
  → dirty=true

commit()
  → réécriture TOTALE des 3 fichiers
  → re-mmap sparse.mmap
  → dirty=false
```

### 2d. Pas de snapshot incrémental

Le commit réécrit **tout**. Pas de notion de segment, pas de merge, pas de GC. Le format est simple et déterministe : 3 fichiers fixes qui représentent l'état complet.

### 2e. Pas d'abstraction storage

Le handle utilise directement `std::fs::write()`, `std::fs::read()`, `memmap2::Mmap::map()`. Aucun trait, aucune indirection.

### 2f. Conclusion sparse

Il faut **ajouter une abstraction storage au niveau Rust** dans le crate sparse_vector. Beaucoup plus simple que lucivy (3 fichiers fixes vs N segments dynamiques).

---

## 3. Comparaison des deux approches

| Aspect | Lucivy (Tantivy) | Sparse vector |
|--------|-----------------|---------------|
| **Abstraction existante** | `Directory` trait (riche, 11 méthodes) | Aucune |
| **Fichiers** | Dynamiques (segments créés/mergés/supprimés) | 3 fichiers fixes |
| **Commit** | Incrémental (nouveaux segments + meta.json atomique) | Full rewrite |
| **Mmap** | Par segment, cache LRU weak-ref | 1 fichier unique, direct |
| **GC** | `ManagedDirectory.garbage_collect()` via `.managed.json` | Aucun (overwrite) |
| **Atomicité** | `atomic_write(meta.json)` | `std::fs::write()` (non atomique) |
| **Taille** | Gros (inverted index complet) | Moyen-gros (postings + vectors) |
| **Stratégie blob** | Implémenter `Directory` custom | Ajouter trait storage simple |

---

## 4. Design proposé : deux traits distincts, pas un IndexBlobStore unifié

La proposition initiale (doc 20) d'un trait `IndexBlobStore` unifié (list/load/save/delete) semble **trop générique** maintenant qu'on connaît les détails :

- Lucivy a besoin de **sémantiques WORM** (write-once, delete, atomic_write) — le trait `Directory` les capture déjà
- Sparse a besoin d'un **simple read/write de blobs** (3 fichiers entiers, pas de WORM, pas de GC)

### 4a. Pour lucivy : `BlobDirectory` (materialise tmpdir + mmap)

**Approche retenue** : à l'ouverture, matérialiser les blobs depuis le `BlobStore` vers un tmpdir local, puis utiliser un vrai `MmapDirectory` (ou `StdFsDirectory`) dessus. Au commit, sync les changements vers le `BlobStore`.

```rust
pub struct BlobDirectory<S: BlobStore> {
    store: Arc<S>,
    index_name: String,
    cache_dir: PathBuf,           // /tmp/lucivy_blob_cache/{index_name}/
    inner: StdFsDirectory,        // directory réel sur cache_dir (mmap-capable)
    watch_router: Arc<RwLock<WatchCallbackList>>,
}
```

**Cycle de vie** :

```
open(store, index_name):
  1. BlobStore.list(index_name) → liste des fichiers
  2. Pour chaque fichier : BlobStore.load() → écrire dans cache_dir/
  3. StdFsDirectory::open(cache_dir)
  4. Prêt — mmap natif, zero-copy reads

open_write(path):  → écrit dans cache_dir (via inner)
atomic_write(path, data):  → écrit dans cache_dir + BlobStore.save() (durabilité)
delete(path):  → supprime de cache_dir + BlobStore.delete()
open_read / get_file_handle:  → lecture depuis cache_dir (mmap natif)

sync_to_store():  // appelé après commit
  1. Lister fichiers dans cache_dir
  2. Diff avec BlobStore.list()
  3. Nouveaux fichiers → BlobStore.save()
  4. Fichiers supprimés → BlobStore.delete()

close / drop:
  → cleanup cache_dir
```

**Avantages** :
- Perf mmap native (OS page cache, zero-copy)
- Zéro changement dans le code lucivy existant (IndexWriter, SegmentUpdater, GC parlent au trait `Directory`)
- Le BlobStore = source of truth (durabilité ACID), le cache_dir = matérialisation runtime

### 4b. Pour sparse : trait `SparseStore`

Ajouter un trait simple dans le crate sparse_vector :

```rust
// Dans sparse_vector/rust/src/
pub trait SparseStore: Send + Sync {
    fn load(&self, file_name: &str) -> Result<Vec<u8>>;
    fn save(&self, file_name: &str, data: &[u8]) -> Result<()>;
    fn exists(&self, file_name: &str) -> Result<bool>;
}
```

3 fichiers = 3 appels save/load. Pas de GC, pas de WORM, pas de lock.

Implémentations :

| Impl | Backend |
|------|---------|
| `FsSparseStore` | `std::fs` (actuel, refactoring) |
| `MemSparseStore` | `HashMap<String, Vec<u8>>` (tests) |
| Blob externe | Via rag3weaver/extension (DB, S3...) |

### 4c. Trait commun `BlobStore` pour les backends

Les deux traits (`BlobDirectory` et `SparseStore`) peuvent partager un même backend :

```rust
// Trait minimal partagé entre lucivy et sparse
pub trait BlobStore: Send + Sync + 'static {
    fn load(&self, index_name: &str, file_name: &str) -> Result<Vec<u8>>;
    fn save(&self, index_name: &str, file_name: &str, data: &[u8]) -> Result<()>;
    fn delete(&self, index_name: &str, file_name: &str) -> Result<()>;
    fn exists(&self, index_name: &str, file_name: &str) -> Result<bool>;
    fn list(&self, index_name: &str) -> Result<Vec<String>>;
}
```

Le `BlobStore` est implémenté une seule fois par backend (DB, S3, filesystem), puis consommé par :
- `BlobDirectory<S: BlobStore>` pour lucivy
- `SparseHandle` avec `store: Box<dyn BlobStore>` pour sparse

### 4d. Table DB unifiée (inchangé par rapport au doc 20)

```sql
_index_blobs(index_name STRING, file_name STRING, data BLOB, PRIMARY KEY(index_name, file_name))
```

Le `CypherBlobStore` (dans rag3weaver, pas dans les crates Rust) implémente `BlobStore` via des requêtes Cypher sur cette table.

---

## 5. Stratégie composite mmap + DB (runtime)

### Pour lucivy :

```
Ouverture:
  1. BlobDirectory charge meta.json depuis le BlobStore
  2. Les segments sont listés (SegmentMeta dans meta.json)
  3. get_file_handle() → load blob → cache en RAM (OwnedBytes)
  4. Pas de mmap possible (données viennent de la DB, pas du FS)
  5. Alternative : matérialiser les blobs en tmpfiles → mmap → perf mmap native

Commit:
  1. save_metas() → atomic_write(meta.json) → store.save()
  2. Nouveaux segments écrits via open_write() → buffer → store.save()
  3. GC → delete segments obsolètes → store.delete()

Option matérialisation tmpfile (si perf mmap critique):
  1. store.load() → write to /tmp/lucivy_cache/{index}/{file}
  2. MmapDirectory sur le tmpdir
  3. sync_to_store() après commit (diff managed_files)
  4. Avantage : perf mmap native, complexité accrue
```

### Pour sparse :

```
Ouverture:
  1. store.load("sparse.mmap") → write tmpfile → mmap (ou Vec<u8> en RAM)
  2. store.load("sparse_dims.bin") → deserialize
  3. Prêt pour search (zero-copy si tmpfile mmap)

Commit:
  1. Réécriture 3 fichiers en RAM
  2. store.save() pour chaque fichier
  3. Re-mmap si tmpfile
```

---

## 6. Questions ouvertes

1. **Performance OwnedBytes vs mmap** : pour lucivy, charger les segments en RAM via `OwnedBytes` (Vec<u8> derrière un Arc) est-il assez rapide pour des index de taille production ? Si non, faut-il matérialiser en tmpfiles.

2. **Atomicité cross-fichiers sparse** : `std::fs::write()` n'est pas atomique. Si le process crash entre l'écriture de `sparse.mmap` et `sparse_dims.bin`, l'index est corrompu. Faut-il un mécanisme de commit atomique (écrire dans des fichiers .tmp puis rename) ?

3. **Taille des blobs** : un index lucivy sur 100k docs peut faire 100+ MB de segments. Stocker ça en BLOB dans rag3db (ou Postgres) a un coût. Faut-il compresser (zstd) avant stockage ?

4. **Sync incrémental lucivy** : à chaque commit, seuls les nouveaux segments changent. Le `BlobDirectory` doit implémenter le diff (quels fichiers sont nouveaux/supprimés) — le `ManagedDirectory` le fait déjà via `.managed.json`. On peut s'en inspirer.

5. **Lock distribué** : si on vise du multi-instance (cloud), `acquire_lock()` doit être distribué (Redis, DB advisory lock, etc.). Pour l'instant single-instance = no-op ou flock suffit.

6. **Où vit le crate BlobStore ?** Options :
   - Nouveau crate `blob-store` partagé entre lucivy et sparse
   - Dans lucivy_core (sparse en dépend)
   - Dans un workspace commun

---

## 7. Ordre d'implémentation suggéré (pas un plan, juste une direction)

```
A. Sparse (plus simple, 3 fichiers, full rewrite)
   1. Trait SparseStore dans sparse_vector/rust/
   2. FsSparseStore (refactoring sans changement)
   3. Tests unitaires
   4. Intégration bridge C++ (passer store au create/open)

B. Lucivy (plus complexe, segments dynamiques)
   1. BlobDirectory dans lucivy_core/
   2. Tests avec RamDirectory → BlobDirectory(MemBlobStore)
   3. Intégration bridge C++ (passer directory au create/open)

C. BlobStore backend DB (dans rag3weaver)
   1. CypherBlobStore : impl BlobStore via Cypher
   2. Table _index_blobs
   3. Tests E2E persistence

D. (Optionnel) Matérialisation tmpfile pour perf mmap
```

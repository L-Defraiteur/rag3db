# Doc 20 — Architecture : Sparse mmap + DB composite storage

Date : 13 mars 2026

Réf : doc 19 (sparse V2 Phase 2 mmap)

## Idée

Combiner mmap (performance search) et DB (persistence ACID) :

- **DB = source of truth** pour la persistence, backup, restore, réplication
- **mmap = cache runtime** matérialisé à l'ouverture, utilisé pour le search

## Flow

### open()

```
1. Lire les blobs depuis table DB (_sparse_meta)
2. Écrire sparse.mmap + sparse_vectors.bin + sparse_dims.bin dans le data dir
3. mmap() le fichier posting → search prêt
```

### commit()

```
1. Écrire sparse.mmap sur disque (comme actuellement)
2. Re-mmap
3. Persister les blobs dans la DB (MERGE dans _sparse_meta)
```

### search()

```
→ mmap inchangé (zero-copy, OS page cache)
```

## Stockage DB

Table `_sparse_meta` (key-value, même pattern que `_catalog_meta`) :

| key | value (blob) |
|-----|-------------|
| `sparse:{index_name}:postings` | contenu brut du sparse.mmap |
| `sparse:{index_name}:vectors` | contenu brut du sparse_vectors.bin |
| `sparse:{index_name}:dims` | contenu brut du sparse_dims.bin |

Le blob est identique bit-pour-bit au fichier — dump direct sur disque puis mmap.

## Trait SparseStorage

```rust
trait SparseStorage {
    /// Charger les données brutes depuis le backend
    fn load_postings_blob(&self, index_name: &str) -> Result<Vec<u8>>;
    fn load_vectors_blob(&self, index_name: &str) -> Result<Vec<u8>>;
    fn load_dims_blob(&self, index_name: &str) -> Result<Vec<u8>>;

    /// Persister les données brutes
    fn save_postings_blob(&self, index_name: &str, data: &[u8]) -> Result<()>;
    fn save_vectors_blob(&self, index_name: &str, data: &[u8]) -> Result<()>;
    fn save_dims_blob(&self, index_name: &str, data: &[u8]) -> Result<()>;
}
```

Deux implémentations :

- `FileStorage` — lit/écrit les fichiers directement (ce qu'on a maintenant)
- `DbStorage` — lit/écrit via Cypher MERGE dans `_sparse_meta`

## Avantages

- **Persistence unifiée** : tout vit dans la DB, pas de fichiers orphelins
- **Backup/restore** : copier la DB suffit, pas besoin de gérer des fichiers annexes
- **Search identique** : mmap inchangé, zéro impact performance
- **Transactions ACID** : commit sparse = commit DB
- **Migration simple** : le format mmap est déjà le format blob

## Coûts

- **Ouverture** : une lecture DB + un write fichier par index (une seule fois au load)
- **Commit** : un write fichier + un MERGE DB (séquentiel, pas sur le hot path)
- **Espace disque** : doublon temporaire (DB + fichier mmap), mais les fichiers mmap sont des caches dérivés

Pour un index de 100k docs, le sparse.mmap fait quelques MB — négligeable.

## Analogies

- SQLite : memory-mapped I/O optionnel depuis le WAL
- DuckDB : buffer pool avec pages mmap'd
- Tantivy : trait `Directory` abstrait le stockage (MmapDirectory, RamDirectory)

Notre approche est la version explicite : DB stocke, mmap sert.

## Prérequis

1. Phase 2 mmap finalisée ✅ (doc 19)
2. Bug lucivy lock résolu (doc 18) — pour les tests E2E persistence
3. Table `_sparse_meta` ou équivalent dans le Catalog rag3weaver

## Proposition : trait unifié `IndexBlobStore` (sparse + FTS + futur)

Le pattern "DB stocke, mmap sert" est identique pour sparse et pour lucivy FTS. Plutôt que deux traits séparés (`SparseStorage` ci-dessus + un hypothétique `FtsStorage`), un seul trait générique suffit :

```rust
/// Backend-agnostic blob persistence for any index type (sparse, FTS, vector, etc.)
///
/// Chaque index est identifié par un `index_name` (ex: "kb_sparse", "kb_fts").
/// Les fichiers sont identifiés par des noms logiques (ex: "postings", "meta.json",
/// "seg_abc.term"). Le trait ne connaît pas la sémantique des fichiers —
/// c'est le consommateur (sparse handle, lucivy handle) qui sait quoi stocker.
trait IndexBlobStore: Send + Sync {
    /// Lister les fichiers stockés pour un index.
    fn list(&self, index_name: &str) -> Result<Vec<String>>;

    /// Charger un fichier blob.
    fn load(&self, index_name: &str, file_name: &str) -> Result<Vec<u8>>;

    /// Sauvegarder des fichiers (atomique — tout ou rien).
    fn save(&self, index_name: &str, files: &[(&str, &[u8])]) -> Result<()>;

    /// Supprimer des fichiers (segments obsolètes après merge, etc.)
    fn delete(&self, index_name: &str, files: &[&str]) -> Result<()>;
}
```

### Implémentations

| Impl | Backend | Usage |
|------|---------|-------|
| `FileBlobStore` | Fichiers sur disque (ce qu'on a) | Embedded, dev, tests |
| `CypherBlobStore` | Table `_index_blobs` via Cypher | rag3db embedded (persistence unifiée) |
| `S3BlobStore` | Object storage | Cloud / production |
| `PostgresBlobStore` | Large objects ou bytea | Déploiement Postgres |

### Utilisation par sparse

```rust
// Remplace SparseStorage — mêmes 3 fichiers, mais via le trait générique
impl SparseHandle {
    fn sync_to_store(&self, store: &dyn IndexBlobStore) -> Result<()> {
        store.save(&self.index_name, &[
            ("postings", &self.mmap_bytes),
            ("vectors", &self.vectors_bytes),
            ("dims", &self.dims_bytes),
        ])
    }

    fn load_from_store(store: &dyn IndexBlobStore, name: &str, dir: &Path) -> Result<Self> {
        for file in store.list(name)? {
            let data = store.load(name, &file)?;
            std::fs::write(dir.join(&file), &data)?;
        }
        // mmap les fichiers depuis dir — search prêt
        Self::open_mmap(dir)
    }
}
```

### Utilisation par lucivy FTS

Lucivy a plus de fichiers (~10-20 par segment), mais le pattern est identique :

```rust
impl LucivyHandle {
    fn sync_to_store(&self, store: &dyn IndexBlobStore) -> Result<()> {
        let managed = self.index.directory().list_managed_files();
        let stored = store.list(&self.index_name)?;

        // Nouveaux fichiers (segments créés depuis le dernier sync)
        let to_save: Vec<_> = managed.iter()
            .filter(|f| !stored.contains(&f.to_string_lossy().to_string()))
            .map(|f| {
                let data = self.index.directory().atomic_read(f)?;
                Ok((f.to_string_lossy().to_string(), data))
            })
            .collect::<Result<_>>()?;

        // Fichiers obsolètes (segments mergés/supprimés)
        let managed_set: HashSet<_> = managed.iter().map(|f| f.to_string_lossy().to_string()).collect();
        let to_delete: Vec<_> = stored.iter()
            .filter(|f| !managed_set.contains(f.as_str()))
            .map(|s| s.as_str())
            .collect();

        if !to_save.is_empty() {
            let refs: Vec<_> = to_save.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
            store.save(&self.index_name, &refs)?;
        }
        if !to_delete.is_empty() {
            store.delete(&self.index_name, &to_delete)?;
        }
        Ok(())
    }
}
```

### Pourquoi un seul trait

- **Sparse** : 3 fichiers fixes (postings, vectors, dims) — simple
- **Lucivy FTS** : N fichiers dynamiques (segments créés/supprimés au merge) — plus de fichiers, mais même API
- **Vector** (futur) : 1-2 fichiers (HNSW graph, vectors) — même pattern
- **Un seul `CypherBlobStore`** à implémenter côté rag3weaver, utilisé par tous les types d'index
- **Un seul `S3BlobStore`** pour le cloud, réutilisé partout

### Table DB unifiée

Au lieu de `_sparse_meta`, une seule table pour tous les index :

```
_index_blobs(index_name STRING, file_name STRING, data BLOB, PRIMARY KEY(index_name, file_name))
```

| index_name | file_name | data |
|------------|-----------|------|
| `kb_sparse` | `postings` | (sparse.mmap bytes) |
| `kb_sparse` | `vectors` | (sparse_vectors.bin bytes) |
| `kb_fts` | `meta.json` | (lucivy meta) |
| `kb_fts` | `seg_abc.term` | (segment data) |
| `kb_fts` | `seg_abc.pos` | (segment data) |

## Priorité

Basse — l'implémentation actuelle (fichiers mmap) fonctionne. Le composite DB+mmap est une évolution pour la mise en production cloud (persistence durable, backup unifié). Le trait `IndexBlobStore` peut être introduit progressivement : d'abord `FileBlobStore` (refactoring sans changement de comportement), puis `CypherBlobStore` quand on en a besoin.

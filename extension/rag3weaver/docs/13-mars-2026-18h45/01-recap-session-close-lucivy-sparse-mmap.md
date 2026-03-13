# Doc 01 — Récap session : CLOSE_LUCIVY_INDEX + sparse mmap finalisé

Date : 13 mars 2026

## Contexte

Suite des sessions 12-mars. Deux objectifs :
1. Résoudre le bug lucivy lock file (doc 18) pour débloquer le test E2E persistence sparse
2. Documenter l'architecture composite mmap+DB pour la production cloud

## Ce qui a été fait

### 1. CLOSE_LUCIVY_INDEX — nouvelle fonction Cypher

**Problème** : `~Database()` dans rag3db engine ne cascade pas la destruction des index d'extensions. Le `rust::Box<LucivyHandle>` n'est jamais droppé → le flock (`IndexWriter`) n'est jamais relâché → impossible de réouvrir la DB dans le même process.

**Root cause confirmée** par l'instance spécialisée lucivy : le problème est côté rag3db engine, pas lucivy. Tous les tests lucivy isolés (drop, cycles open/close) passent.

**Solution** : `CLOSE_LUCIVY_INDEX(table)` — flush les écritures pendantes + libère le writer lock sans supprimer les données.

#### Fichiers modifiés

**Rust (ld-lucivy)** :

| Fichier | Changement |
|---------|-----------|
| `lucivy_core/src/handle.rs` | `writer: Mutex<Option<IndexWriter>>` au lieu de `Mutex<IndexWriter>`. Ajout méthode `close()` (commit + take writer). Nouveau test `test_close_releases_lock`. |
| `lucivy_fts/rust/src/bridge.rs` | Ajout `close_index()` au cxx bridge. Tous les accès writer adaptés pour `Option` (`guard.as_mut().ok_or("index is closed")`). |
| `bindings/wasm/src/lib.rs` | Accès writer adaptés pour `Option`. |

**C++ (lucivy_fts)** :

| Fichier | Changement |
|---------|-----------|
| `close_lucivy_index.h` | **Nouveau** — header pour `CLOSE_LUCIVY_INDEX` / `_CLOSE_LUCIVY_INDEX` |
| `close_lucivy_index.cpp` | **Nouveau** — implémentation (même pattern que flush/drop). No-op si pas d'index. |
| `lucivy_index.h` | Ajout méthode `close()` |
| `lucivy_index.cpp` | Implémentation `close()` : `flushIfDirty()` + `close_index(*handle_)` |
| `lucivy_fts_extension.cpp` | Enregistrement `CloseLucivyFunction` + `InternalCloseLucivyFunction` |
| `CMakeLists.txt` (function/) | Ajout `close_lucivy_index.cpp` |

**rag3weaver** :

| Fichier | Changement |
|---------|-----------|
| `catalog.rs` | Ajout `shutdown()` : itère sur entity_configs + kb_metadata, appelle `CLOSE_LUCIVY_INDEX` pour chaque table |
| `e2e_search.rs` | Test `phase6_sparse_mmap_persistence` : utilise `catalog.shutdown()` au lieu du workaround `remove_lucivy_locks`. Retiré `#[ignore]`. Nettoyage helpers inutilisés. |

#### Tests

- 1096 tests ld-lucivy passent (+ 1 nouveau `test_close_releases_lock` = 1097)
- rag3weaver compile (check --lib + check --tests)
- Le test `test_close_releases_lock` valide : close() libère le lock, réouverture OK même avec le handle original encore en scope

### 2. Doc 20 — Architecture composite mmap+DB + trait unifié IndexBlobStore

Documenté dans `docs/12-mars-2026-15h21/20-architecture-sparse-mmap-db-composite.md`.

**Idée de base** : DB = source of truth (persistence ACID, backup, réplication), mmap = cache runtime matérialisé à l'ouverture.

**Évolution proposée par l'instance lucivy** : au lieu d'un trait `SparseStorage` spécifique, un trait unifié `IndexBlobStore` générique pour tous les types d'index :

```rust
trait IndexBlobStore: Send + Sync {
    fn list(&self, index_name: &str) -> Result<Vec<String>>;
    fn load(&self, index_name: &str, file_name: &str) -> Result<Vec<u8>>;
    fn save(&self, index_name: &str, files: &[(&str, &[u8])]) -> Result<()>;
    fn delete(&self, index_name: &str, files: &[&str]) -> Result<()>;
}
```

- **Sparse** : 3 fichiers fixes (postings, vectors, dims)
- **Lucivy FTS** : N fichiers dynamiques (segments créés/supprimés au merge) — sync incrémental via diff `managed_files` vs `stored`
- **Vector** (futur) : 1-2 fichiers (HNSW graph, vectors)

Implémentations prévues : `FileBlobStore` (actuel), `CypherBlobStore` (DB via `_index_blobs` table), `S3BlobStore` (cloud), `PostgresBlobStore`.

Table DB unifiée : `_index_blobs(index_name STRING, file_name STRING, data BLOB, PRIMARY KEY(index_name, file_name))` — remplace le `_sparse_meta` initial par une table partagée par tous les index.

Priorité basse — introduction progressive : d'abord `FileBlobStore` (refactoring sans changement), puis `CypherBlobStore` quand nécessaire.

### 3. Doc 18 mis à jour

Ajout de la section "Investigation — Root cause confirmée" avec le tableau des tests par couche et la conclusion : bug côté rag3db engine, pas lucivy.

## Différence CLOSE vs DROP vs FLUSH

| Fonction | Commit dirty ? | Libère writer lock ? | Supprime fichiers ? | Supprime du catalog ? |
|----------|---------------|---------------------|--------------------|-----------------------|
| `FLUSH_LUCIVY_INDEX` | ✅ | ❌ | ❌ | ❌ |
| `CLOSE_LUCIVY_INDEX` | ✅ | ✅ | ❌ | ❌ |
| `DROP_LUCIVY_INDEX` | implicite | ✅ (via drop) | ✅ | ✅ |

## Prochaines étapes

1. **Builder l'extension C++ complète** et lancer le test `phase6_sparse_mmap_persistence` E2E
2. **Benchmark** sparse mmap sur 100k+ docs
3. **Trait `IndexBlobStore`** : commencer par `FileBlobStore` (refactoring sans changement de comportement)

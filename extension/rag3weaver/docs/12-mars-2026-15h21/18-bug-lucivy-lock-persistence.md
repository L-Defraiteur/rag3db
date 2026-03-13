# Doc 18 — Bug : Lucivy lock file non relâché à la fermeture DB

Date : 13 mars 2026

Réf : doc 17 (sparse V2 POC), découvert pendant test E2E persistence sparse mmap

## Symptôme

Test E2E : session 1 (create + drain + drop Catalog) → session 2 (reopen) → **crash** :

```
cannot create writer: Failed to acquire Lockfile: LockBusy.
"there is already an IndexWriter working on this Directory"
```

Le lock file `.tantivy-writer.lock` n'est pas relâché entre deux sessions dans le même process.

## Chaîne de drop attendue

```
Catalog (Rust, rag3weaver)
  └→ Box<dyn DbConnection>
       └→ Rag3dbConnection { conn, db }
            └→ conn: Connection dropped first (déclaré avant db)
            └→ db: Box<Database> → UniquePtr<ffi::Database> → destructeur C++
                 └→ ~Database → ~Catalog → ~NodeTable
                      └→ ~LucivyIndex (C++)
                           └→ drop rust::Box<LucivyHandle>
                                └→ drop LucivyHandle
                                     └→ drop Mutex<IndexWriter>
                                          └→ drop _directory_lock: Option<DirectoryLock>
                                               └→ drop ReleaseLockFile { _file, path }
                                                    └→ drop File → OS release lock ✓
```

## Analyse initiale

Pas d'état global (`static`, `Arc` partagé, singleton) trouvé côté extension lucivy_fts C++ ni côté Rust.

Hypothèses initiales :

1. **Le destructeur C++ de `Database` ne cascade pas complètement** jusqu'aux tables/index custom des extensions.
2. **Le `Connection` Rust est transmuted en `'static`** (`std::mem::transmute` dans `Rag3dbConnection::connect()`).
3. **Les extensions chargées via `LOAD EXTENSION` restent en mémoire** (shared lib).
4. **Ordre de destruction des tables dans rag3db**.

## Investigation — Root cause confirmée ✅

Mise à jour : 13 mars 2026

### Tests lucivy isolés : lock release OK

| Couche | Lock release ? | Vérifié par |
|--------|---------------|-------------|
| lucivy `IndexWriter::Drop` | **OK** ✅ | `test_lockfile_released_on_drop_mmap` |
| lucivy `LucivyHandle` drop | **OK** ✅ | `test_handle_close_reopen_lock`, `test_handle_close_reopen_with_merges` |
| lucivy 5 cycles open/close | **OK** ✅ | `test_handle_reopen_cycles` |
| rag3db `~Database()` → `~LucivyIndex` | **NON** ❌ | Workaround `remove_lucivy_locks()` dans test E2E |

### Conclusion

**Le bug est côté rag3db engine, pas lucivy.**

- `Database` Rust = wrapper autour de `UniquePtr<ffi::Database>`, pas de Drop custom
- Quand `UniquePtr` est droppé → appelle `~Database()` C++
- **`~Database()` C++ ne cascade pas la destruction des index d'extensions**
- Donc `rust::Box<LucivyHandle>` n'est jamais droppé → `IndexWriter` jamais droppé → flock jamais relâché

La chaîne de drop attendue (section ci-dessus) ne se réalise pas : `~Database()` ne descend pas jusqu'à `~LucivyIndex()`.

## Impact

- Tout test E2E qui fait close → reopen d'une DB **avec FTS/lucivy** dans le même process échoue
- Les tests `e2e_idempotent_registration` (persistence) marchent car ils n'utilisent pas `drain()` (pas de création d'index lucivy)
- Sparse et vector ne sont pas affectés (pas de lock file)

## Workaround temporaire

Pour les tests E2E persistence qui ont besoin de FTS :
- Supprimer manuellement les `.tantivy-writer.lock` entre les sessions
- Ou ne pas charger l'extension lucivy_fts si BM25 n'est pas nécessaire au test

**Note** : la suppression du lock file ne suffit pas si le flock OS est encore tenu par un file descriptor en mémoire (même process). Fonctionne uniquement si le fichier est recréé.

## Solutions possibles

Côté lucivy_fts (puisqu'on ne contrôle pas rag3db engine) :

1. **`DROP_LUCIVY_INDEX` explicite** dans `Catalog::shutdown()` avant de dropper la connexion — force la destruction du LucivyHandle via Cypher
2. **Hook `on_database_close`** dans l'extension lucivy_fts — si rag3db engine supporte un tel callback
3. **RAII guard côté rag3weaver** — exécute `CALL flush_lucivy_index()` ou similaire avant le drop

Côté rag3db engine (fix propre) :

4. **Corriger `~Database()`** pour cascader la destruction des index d'extensions — c'est le vrai fix mais nécessite de modifier le fork rag3db

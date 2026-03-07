# Doc 08 — Rename lucivy_fts → lucivy_fts

**Date** : 4 mars 2026
**Statut** : À faire
**Contexte** : Le fork ld-lucivy a maintenant un package Python (`lucivy`). On veut renommer le crate Rust FFI de `lucivy-fts` → `lucivy-fts` pour cohérence.

---

## Commits de référence (rollback)

| Repo | Branche | Commit | Message |
|------|---------|--------|---------|
| **ld-lucivy** | `main` | `2b932f2` | feat: add lucivy Python bindings (PyO3 + maturin) |
| **rag3db** | `feature/kb-index-architecture` | `35cea71b5` | feat: simplify filters to single allowed_ids path + update ld-lucivy |

---

## Scope du rename

### Repo ld-lucivy

#### Dossier
- `lucivy_fts/` → `lucivy_fts/`

#### Fichiers à modifier

| Fichier | Changement |
|---------|-----------|
| `Cargo.toml` (workspace) | member `"lucivy_fts/rust"` → `"lucivy_fts/rust"` |
| `lucivy_fts/rust/Cargo.toml` | `name = "lucivy-fts"` → `name = "lucivy-fts"` |
| `lucivy_fts/rust/src/lib.rs` | Commentaire doc `lucivy-fts` → `lucivy-fts` |
| `lucivy_fts/rust/src/bridge.rs` | Commentaire doc `lucivy_fts` → `lucivy_fts` |
| `lucivy_fts/rust/src/tokenizer.rs` | Commentaire doc `lucivy_fts` → `lucivy_fts` |
| `lucivy_fts/rust/build.rs` | `compile("lucivy_fts_cxx")` → `compile("lucivy_fts_cxx")` |
| `lucivy/Cargo.toml` | dep path `"../lucivy_fts/rust"` → `"../lucivy_fts/rust"`, dep name `lucivy-fts` → `lucivy-fts` |
| `lucivy/src/lib.rs` | `use lucivy_fts::` → `use lucivy_fts::` |

### Repo rag3db (extension C++)

#### Dossier
- `extension/lucivy_fts/` → `extension/lucivy_fts/`

#### Fichiers à modifier

| Fichier | Changement |
|---------|-----------|
| **CMakeLists.txt** (root extension) | `lucivy_fts/rust` → `lucivy_fts/rust`, `liblucivy_fts.a` → `liblucivy_fts.a`, targets `lucivy_fts_rust` → `lucivy_fts_rust`, `lucivy_fts_lib` → `lucivy_fts_lib`, `build_extension_lib("lucivy_fts")` → `build_extension_lib("lucivy_fts")` |
| **src/\*\*/CMakeLists.txt** (4 fichiers) | `lucivy_fts_extension_*` → `lucivy_fts_extension_*` |
| **test/CMakeLists.txt** | `lucivy_fts_test` → `lucivy_fts_test`, `rag3db_lucivy_fts_extension` → `rag3db_lucivy_fts_extension` |
| **C++ namespaces** (~15 fichiers) | `lucivy_fts_extension` → `lucivy_fts_extension` |
| **C++ includes** | `main/lucivy_fts_extension.h` → `main/lucivy_fts_extension.h` |
| **C++ filenames** | `lucivy_fts_extension.cpp/.h` → `lucivy_fts_extension.cpp/.h` |
| **test/lucivy_fts_test.cpp** | Toutes les refs `lucivy_fts` dans les paths d'extension LOAD, renommer le fichier → `lucivy_fts_test.cpp` |
| **BUILD_EXTENSIONS cmake** | Les scripts cmake qui passent `-DBUILD_EXTENSIONS="lucivy_fts"` → `"lucivy_fts"` |
| **rag3weaver refs** | Vérifier si `search.rs` / `catalog.rs` référencent `lucivy_fts` (probablement dans les paths d'index : `lucivy_indexes/`) |

#### Paths d'index sur disque
- `lucivy_indexes/<table>/` — c'est le chemin où les index Lucivy sont stockés. Ce nom est dans `lucivy_index.cpp` (`getDatabasePath() + /lucivy_indexes/`). À renommer en `lucivy_indexes/` ? **Attention : breaking pour les données existantes.** Option : garder `lucivy_indexes/` pour compatibilité, ou migrer.

---

## Ordre d'exécution

```
1. ld-lucivy : git mv lucivy_fts/ lucivy_fts/ + sed sur les fichiers
2. ld-lucivy : cargo build --lib (vérifier que ça compile)
3. ld-lucivy : maturin develop dans lucivy/ (vérifier Python)
4. ld-lucivy : commit + push
5. rag3db : git mv extension/lucivy_fts/ extension/lucivy_fts/ + sed sur les fichiers
6. rag3db : update submodule ld-lucivy
7. rag3db : cmake --build (vérifier que l'extension compile)
8. rag3db : run tests (lucivy_fts_test → lucivy_fts_test)
9. rag3db : commit + push
```

---

## Risques

- **Données existantes** : les index sur disque sont dans `lucivy_indexes/`. Renommer casserait la rétro-compat. Décision à prendre : garder l'ancien path ou migrer.
- **Build cache** : `cargo clean` nécessaire après le rename du crate.
- **CMake cache** : reconfiguration cmake nécessaire après le rename des targets.
- **Submodule** : le path du submodule dans rag3db (`extension/lucivy/ld-lucivy`) ne change pas — seul le contenu du submodule change.

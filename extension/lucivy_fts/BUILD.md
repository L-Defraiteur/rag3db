# Build Guide — lucivy_fts extension

## Architecture de build

```
ld-lucivy (Rust lib, workspace root)
  -> lucivy_fts (Rust cxx bridge crate, workspace member)
    -> liblucivy_fts.a (static lib)
      -> liblucivy_fts.rag3db_extension (shared lib, chargee par LOAD EXTENSION)
      -> lucivy_fts_test (binaire de test, charge l'extension dynamiquement)
```

Les 3 couches :

| Couche | Langage | Sortie | Outil |
|--------|---------|--------|-------|
| `ld-lucivy` + `lucivy_fts/rust/` | Rust | `target/release/liblucivy_fts.a` | cargo |
| Extension C++ | C++ | `liblucivy_fts.rag3db_extension` | cmake |
| Tests GTest | C++ | `lucivy_fts_test` | cmake |

## Detection automatique des changements Rust

Le `CMakeLists.txt` utilise `file(GLOB_RECURSE)` + `DEPENDS` pour detecter les modifications des fichiers `.rs` et `Cargo.toml`. Quand un fichier Rust est modifie, cmake relance automatiquement cargo au prochain build.

**Limitation** : les NOUVEAUX fichiers `.rs` ne sont detectes qu'apres re-configuration cmake (`cmake ../..`). Les modifications de fichiers existants sont detectees immediatement.

## Rebuild apres modification Rust

Un seul `cmake --build` suffit — cmake detecte les changements et relance cargo automatiquement :

```bash
cd build/release
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
cmake --build . --target lucivy_fts_test -j$(nproc)
```

Les deux emplacements sources Rust sont surveilles :
- `extension/lucivy/ld-lucivy/src/` — la lib Lucivy (queries, scoring, postings, schema)
- `extension/lucivy/ld-lucivy/lucivy_fts/rust/src/` — le bridge cxx (handle, query routing, bridge)

## Builds de reference

### Tests unitaires Rust (1015 tests)

```bash
cd extension/lucivy/ld-lucivy
cargo test --lib
```

### Tests E2E GTest (11 tests)

```bash
cd build/release

# Premiere config (une seule fois)
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="lucivy_fts" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE

# Build
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
cmake --build . --target lucivy_fts_test -j$(nproc)

# Run
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu \
  ./extension/lucivy_fts/test/lucivy_fts_test
```

### Node.js natif

```bash
cd build/nodejs

cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSIONS="lucivy_fts" \
  -DBUILD_NODEJS=TRUE -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE

cmake --build . --target rag3dbjs -j$(nproc)
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
```

Le `.node` est genere directement dans `tools/nodejs_api/src_js/` (ou le loader l'attend).
L'extension `.rag3db_extension` est chargee dynamiquement via `LOAD EXTENSION`.

### WASM (Emscripten)

```bash
cd build/wasm
source ~/emsdk/emsdk_env.sh

emcmake cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_WASM=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE

emmake cmake --build . -j$(nproc)
```

Le `.js` WASM est genere directement dans `tools/wasm/package/nodejs/rag3db/` (ou le loader l'attend).
Les extensions (lucivy_fts, json, vector, algo) sont statiquement linkees.

Tests browser Playwright :
```bash
cd tools/wasm
npx playwright test
```

Prerequis WASM :
- Emscripten >= 5.0.1
- Rust nightly (`rustup install nightly`)
- `rustup component add rust-src --toolchain nightly`
- Le CMakeLists.txt ajoute automatiquement `-Z build-std` + `+atomics,+bulk-memory` + `-C panic=abort`

## Problemes connus

### `LD_LIBRARY_PATH` et miniconda

Si les tests crashent avec des erreurs de symboles (`GLIBCXX_3.4.32 not found`), c'est que miniconda ou conda injecte un vieux `libstdc++` via `LD_LIBRARY_PATH`.

**Fix** : prefixer les commandes avec `LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu`.

### cargo ne recompile pas apres modification

Cargo detecte les changements par timestamp. Si le fichier a le meme timestamp (copie, git checkout), cargo ne recompile pas.

**Fix** : `touch` le fichier modifie ou `cargo clean -p ld-lucivy`.

### cmake ne detecte pas les nouveaux fichiers Rust

Le `file(GLOB_RECURSE)` qui surveille les `.rs` est evalue au moment de la configuration cmake. Les nouveaux fichiers `.rs` ne sont detectes qu'apres `cmake ../..` (re-configuration).

**Fix** : relancer `cmake ../..` apres avoir ajoute un nouveau fichier `.rs`.

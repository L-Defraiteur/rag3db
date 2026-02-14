# Build Guide — tantivy_fts extension

## Architecture de build

```
ld-tantivy (Rust lib, workspace root)
  -> tantivy_fts (Rust cxx bridge crate, workspace member)
    -> libtantivy_fts.a (static lib)
      -> libtantivy_fts.rag3db_extension (shared lib, chargee par LOAD EXTENSION)
      -> tantivy_fts_test (binaire de test, charge l'extension dynamiquement)
```

Les 3 couches :

| Couche | Langage | Sortie | Outil |
|--------|---------|--------|-------|
| `ld-tantivy` + `tantivy_fts/rust/` | Rust | `target/release/libtantivy_fts.a` | cargo |
| Extension C++ | C++ | `libtantivy_fts.rag3db_extension` | cmake |
| Tests GTest | C++ | `tantivy_fts_test` | cmake |

## Piege principal : cmake ne detecte pas les changements Rust

Le `CMakeLists.txt` utilise `add_custom_command(OUTPUT libtantivy_fts.a ...)` **sans `DEPENDS`** sur les fichiers sources Rust. Consequence :

- **Si la `.a` existe deja, cmake ne relance JAMAIS cargo**, meme si les `.rs` ont change
- **Si la `.a` est plus recente que la `.rag3db_extension`, cmake ne re-linke pas** l'extension dynamique
- Le test binary charge l'extension via `LOAD EXTENSION '...libtantivy_fts.rag3db_extension'`, donc re-linker le test seul ne suffit pas

## Sequence de rebuild apres modification Rust

### 1. Modifier les fichiers `.rs`

Deux emplacements possibles :
- `extension/tantivy/ld-tantivy/src/` — la lib Tantivy (queries, scoring, postings, schema)
- `extension/tantivy/ld-tantivy/tantivy_fts/rust/src/` — le bridge cxx (handle, query routing, bridge)

### 2. Recompiler le Rust (cargo)

```bash
cd extension/tantivy/ld-tantivy

# Touch pour forcer la redetection par cargo
touch src/lib.rs  # si modif dans ld-tantivy/src/
# ou
touch tantivy_fts/rust/src/lib.rs  # si modif dans tantivy_fts/

# Rebuild les deux crates
cargo build --release -p ld-tantivy -p tantivy-fts
```

Verifier que la sortie affiche `Compiling ld-tantivy` et/ou `Compiling tantivy-fts`. Si on voit seulement `Finished` sans `Compiling`, les changements ne sont pas detectes — touch un fichier racine du crate (`lib.rs` ou `Cargo.toml`).

### 3. Re-linker l'extension shared lib (cmake)

```bash
cd build/release
cmake --build . --target rag3db_tantivy_fts_extension -j$(nproc)
```

Verifier que la sortie affiche `Linking CXX shared library ...libtantivy_fts.rag3db_extension`. Si cmake dit tout est "Built" sans linking, forcer :

```bash
# Forcer le re-link en touchant un source C++
touch ../../extension/tantivy_fts/src/tantivy_fts_extension.cpp
cmake --build . --target rag3db_tantivy_fts_extension -j$(nproc)
```

### 4. Rebuild le test (si modifie)

```bash
cmake --build . --target tantivy_fts_test -j$(nproc)
```

Note : le test charge l'extension dynamiquement, donc c'est l'etape 3 (re-link extension) qui compte pour les changements Rust.

### Commande tout-en-un

```bash
# Depuis la racine rag3db/
cd extension/tantivy/ld-tantivy && \
  cargo build --release -p ld-tantivy -p tantivy-fts && \
  cd ../../../build/release && \
  cmake --build . --target rag3db_tantivy_fts_extension -j$(nproc) && \
  cmake --build . --target tantivy_fts_test -j$(nproc)
```

## Builds de reference

### Tests unitaires Rust (1015 tests)

```bash
cd extension/tantivy/ld-tantivy
cargo test --lib
```

### Tests E2E GTest (10 tests)

```bash
cd build/release

# Premiere config (une seule fois)
cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="tantivy_fts" -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE

# Build
cmake --build . --target rag3db_tantivy_fts_extension -j$(nproc)
cmake --build . --target tantivy_fts_test -j$(nproc)

# Run
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu \
  ./extension/tantivy_fts/test/tantivy_fts_test
```

### Node.js natif

```bash
cd build/nodejs

cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSIONS="tantivy_fts" \
  -DBUILD_NODEJS=TRUE -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE

cmake --build . --target rag3dbjs -j$(nproc)
cmake --build . --target rag3db_tantivy_fts_extension -j$(nproc)
```

### WASM (Emscripten)

```bash
cd build/wasm
source ~/emsdk/emsdk_env.sh

emcmake cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_WASM=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE

emmake cmake --build . -j$(nproc)
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

**Fix** : `touch` le fichier modifie ou `cargo clean -p ld-tantivy`.

### cmake ne re-linke pas l'extension

Le `add_custom_command` pour la lib Rust n'a pas de `DEPENDS` sur les sources `.rs`. Si la `.a` existe, cmake ne relance pas cargo et ne re-linke pas l'extension.

**Fix** : rebuild manuellement avec cargo (etape 2 ci-dessus), puis forcer le re-link (etape 3).

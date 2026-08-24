# Build Guide — rag3db

## Prerequis

- **CMake** >= 3.15
- **GCC/G++** >= 11 (ou Clang >= 15)
- **Rust** stable + nightly (pour WASM)
  ```bash
  rustup install stable nightly
  rustup component add rust-src --toolchain nightly
  ```
- **Emscripten** >= 5.0.1 (uniquement pour WASM)
- **Node.js** >= 18 (pour les tests Playwright et le build Node.js)

## Structure de build

```
rag3db/
├── build/release/     <- natif (tests E2E, extension .rag3db_extension)
├── build/nodejs/      <- Node.js natif (rag3dbjs.node + extension)
├── build/wasm/        <- WASM browser (rag3db_wasm.js)
│
├── extension/sparse_vector/rust/         <- Rust : crate sparse-vector (cxx bridge)
├── extension/sparse_vector/              <- C++ : extension rag3db
│   └── build/libsparse_vector.rag3db_extension  <- shared lib (LOAD EXTENSION)
│
├── extension/rag3weaver/                 <- Rust : le produit (FTS lucivy v3 inclus,
│                                            pas d'extension C++ pour le FTS)
```

## IMPORTANT : Rust et cmake sont deconnectes

cmake ne detecte **pas** les changements dans les fichiers `.rs`. Si vous modifiez du Rust, il faut reconstruire manuellement :

```bash
# 1. Recompiler le Rust
cd extension/sparse_vector/rust
cargo build --release

# 2. Re-linker l'extension
cd build/release  # (ou build/nodejs, build/wasm)
cmake --build . --target rag3db_sparse_vector_extension -j$(nproc)
```


---

## 1. Tests unitaires Rust

```bash
cd extension/rag3weaver
cargo test --lib                              # orchestrateur (sans embedder)
cargo test --lib --features burn-embedder     # + embedders burn
```

## 2. Build natif + tests E2E

```bash
mkdir -p build/release && cd build/release

# Config (une seule fois)
cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="vector;sparse_vector;geo" \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE

cmake --build . -j$(nproc)

# Tests E2E rag3weaver (voir extension/rag3weaver/run_e2e.sh)
cd ../../extension/rag3weaver && ./run_e2e.sh
```

## 3. Build Node.js natif

```bash
mkdir -p build/nodejs && cd build/nodejs

cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSIONS="vector;sparse_vector" \
  -DBUILD_NODEJS=TRUE -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE

cmake --build . --target rag3dbjs -j$(nproc)
```

Sortie : `tools/nodejs_api/build/rag3dbjs.node` + `extension/*/build/*.rag3db_extension`

## 4. Build WASM (browser)

```bash
mkdir -p build/wasm && cd build/wasm
source ~/emsdk/emsdk_env.sh

emcmake cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_WASM=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE

emmake cmake --build . -j$(nproc)
```

Sortie : `tools/wasm/build/rag3db/rag3db_wasm.js` (~17MB, single file)

Extensions liees statiquement : json, vector, sparse_vector, algo (le FTS est dans rag3weaver).

### Tests Playwright (browser IDBFS)

```bash
cd tools/wasm
npm install
npx playwright test    # 2 tests, ~10s
```

## 5. Tout reconstruire apres modif Rust (natif)

Commande complete depuis la racine rag3db :

```bash
cd extension/sparse_vector/rust && cargo build --release && \
  cd ../../../build/release && \
  cmake --build . --target rag3db_sparse_vector_extension -j$(nproc)
# Le FTS (lucivy v3) est compilé par cargo dans rag3weaver : rien à re-linker côté cmake.
```

---

## Problemes courants

| Symptome | Cause | Fix |
|----------|-------|-----|
| Les changements Rust ne sont pas pris en compte | cmake ne relance pas cargo | `cargo build --release` manuellement, puis re-linker l'extension |
| `cargo build` dit `Finished` sans `Compiling` | cargo n'a pas detecte le changement | `touch src/lib.rs` puis rebuilder |
| `GLIBCXX_3.4.32 not found` au runtime | miniconda injecte un vieux libstdc++ | `LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu` |
| Tests passent mais avec vieux scores | Extension `.rag3db_extension` pas re-linkee | `cmake --build . --target rag3db_sparse_vector_extension` |
| WASM crash au demarrage | Mauvaise version emscripten ou manque nightly | Verifier `emcc --version` >= 5.0.1, `rustup run nightly rustc --version` |

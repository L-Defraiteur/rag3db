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
├── extension/lucivy/ld-lucivy/          <- Rust : lib Lucivy (cargo workspace)
│   └── lucivy_fts/rust/                  <- Rust : crate FFI cxx bridge
│       └── target/release/liblucivy_fts.a  <- sortie cargo
│
├── extension/lucivy_fts/                 <- C++ : extension rag3db
│   ├── build/liblucivy_fts.rag3db_extension  <- shared lib (LOAD EXTENSION)
│   └── test/lucivy_fts_test             <- binaire test GTest
```

## IMPORTANT : Rust et cmake sont deconnectes

cmake ne detecte **pas** les changements dans les fichiers `.rs`. Si vous modifiez du Rust, il faut reconstruire manuellement :

```bash
# 1. Recompiler le Rust
cd extension/lucivy/ld-lucivy
cargo build --release -p ld-lucivy -p lucivy-fts

# 2. Re-linker l'extension
cd build/release  # (ou build/nodejs, build/wasm)
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
```

Voir [`extension/lucivy_fts/BUILD.md`](extension/lucivy_fts/BUILD.md) pour le detail complet.

---

## 1. Tests unitaires Rust

```bash
cd extension/lucivy/ld-lucivy
cargo test --lib    # 1015 tests
```

## 2. Build natif + tests E2E

```bash
mkdir -p build/release && cd build/release

# Config (une seule fois)
cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSION_TESTS=TRUE \
  -DBUILD_EXTENSIONS="lucivy_fts" \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE

# Build extension + tests
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
cmake --build . --target lucivy_fts_test -j$(nproc)

# Run (10 tests)
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu \
  ./extension/lucivy_fts/test/lucivy_fts_test
```

## 3. Build Node.js natif

```bash
mkdir -p build/nodejs && cd build/nodejs

cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSIONS="lucivy_fts" \
  -DBUILD_NODEJS=TRUE -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE

cmake --build . --target rag3dbjs -j$(nproc)
cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc)
```

Sortie : `tools/nodejs_api/build/rag3dbjs.node` + `extension/lucivy_fts/build/liblucivy_fts.rag3db_extension`

## 4. Build WASM (browser)

```bash
mkdir -p build/wasm && cd build/wasm
source ~/emsdk/emsdk_env.sh

emcmake cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_WASM=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE

emmake cmake --build . -j$(nproc)
```

Sortie : `tools/wasm/build/rag3db/rag3db_wasm.js` (~17MB, single file)

Extensions liees statiquement : lucivy_fts, json, vector, algo.

### Tests Playwright (browser IDBFS)

```bash
cd tools/wasm
npm install
npx playwright test    # 2 tests, ~10s
```

## 5. Tout reconstruire apres modif Rust (natif)

Commande complete depuis la racine rag3db :

```bash
cd extension/lucivy/ld-lucivy && \
  cargo build --release -p ld-lucivy -p lucivy-fts && \
  cd ../../../build/release && \
  cmake --build . --target rag3db_lucivy_fts_extension -j$(nproc) && \
  cmake --build . --target lucivy_fts_test -j$(nproc) && \
  LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu \
    ./extension/lucivy_fts/test/lucivy_fts_test
```

---

## Problemes courants

| Symptome | Cause | Fix |
|----------|-------|-----|
| Les changements Rust ne sont pas pris en compte | cmake ne relance pas cargo | `cargo build --release` manuellement, puis re-linker l'extension |
| `cargo build` dit `Finished` sans `Compiling` | cargo n'a pas detecte le changement | `touch src/lib.rs` puis rebuilder |
| `GLIBCXX_3.4.32 not found` au runtime | miniconda injecte un vieux libstdc++ | `LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu` |
| Tests passent mais avec vieux scores | Extension `.rag3db_extension` pas re-linkee | `cmake --build . --target rag3db_lucivy_fts_extension` |
| WASM crash au demarrage | Mauvaise version emscripten ou manque nightly | Verifier `emcc --version` >= 5.0.1, `rustup run nightly rustc --version` |

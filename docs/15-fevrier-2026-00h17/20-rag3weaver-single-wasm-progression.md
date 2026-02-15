# Rag3Weaver — Single WASM : Progression (15 février 2026)

Date : 15 février 2026
Statut : en cours (Phase 2 partiellement faite)

---

## Ce qui a été fait

### Phase 1 : Compilation WASM emscripten — FAIT

1. **`src/wasm_ffi.rs`** — créé (~500 lignes)
   - Déclarations `extern "C"` pour ~50 fonctions du C API rag3db (`rag3db.h`)
   - Types `#[repr(C)]` : CDatabase, CConnection, CPreparedStatement, CQueryResult, CFlatTuple, CValue, CLogicalType, CSystemConfig, CInternalId
   - Constantes type_id : NODE, REL, BOOL, INT8-64, UINT8-64, FLOAT, DOUBLE, STRING, LIST, ARRAY, STRUCT, MAP
   - `WasmDbConnection` : implémente `DbConnection` via le C API (query_sync, query_with_params_sync, bind_param, read_query_result)
   - Conversion `value_to_cypher()` récursive : gère tous les types scalaires + NODE + REL + LIST + STRUCT + MAP (miroir exact de `rag3db_connection.rs`)
   - Drop impl : détruit connection puis database
   - **Entry points `extern "C"`** pour l'embind C++ :
     - `rag3weaver_version()` → `*const c_char` ("0.1.0")
     - `rag3weaver_catalog_new(config_json, db_path)` → `*mut WeaverContext` (parse config, crée WasmDbConnection, MockEmbedder, Catalog, initialize)
     - `rag3weaver_catalog_destroy(ctx)` — libère le WeaverContext
     - `rag3weaver_create(ctx, entity_type, fields_json)` → JSON `{"uuid":"..."}` ou `{"error":"..."}`
     - `rag3weaver_drain(ctx)` → JSON `{"processed":N,"failed":N,"persisted":N}`
     - `rag3weaver_count(ctx, entity_type)` → JSON `{"count":N}`
   - `WeaverContext` contient un `Catalog` (qui possède le WasmDbConnection + MockEmbedder)
   - `pollster::block_on()` utilisé dans les entry points pour driver les futures async (résolution immédiate)

2. **`src/lib.rs`** — modifié
   - `#[cfg(feature = "wasm-emscripten")] pub mod wasm_ffi;`
   - `#[cfg(feature = "wasm-emscripten")] pub use wasm_ffi::WasmDbConnection;`

3. **`Cargo.toml`** — modifié
   - Feature `wasm-emscripten = ["dep:pollster"]`
   - Dépendance `pollster = { version = "0.4", optional = true }`

4. **Vérification** : 3 targets compilent
   - `cargo test` → 271 tests OK
   - `cargo check --target wasm32-unknown-unknown --no-default-features` → OK
   - `cargo check --target wasm32-unknown-emscripten --features wasm-emscripten --no-default-features` → OK

### Phase 2 : Intégration build — EN COURS

5. **`extension/rag3weaver/CMakeLists.txt`** — créé
   - Guard `if(NOT EMSCRIPTEN) return() endif()`
   - Cargo build pour `wasm32-unknown-emscripten` avec nightly + build-std + atomics
   - `--no-default-features --features wasm-emscripten` (pas de candle-embedder)
   - Produit `librag3weaver.a` en static lib importée (`rag3weaver_lib`)
   - Détection de changements via `GLOB_RECURSE` sur `src/*.rs` + `Cargo.toml`

---

## Ce qui reste à faire

### Phase 2 (suite)

6. **`tools/wasm/src_cpp/weaver_bindings.cpp`** — À CRÉER
   - Fichier C++ embind qui wraps les `extern "C"` rag3weaver_* en classe JS `Weaver`
   - Pattern :
     ```cpp
     extern "C" {
         void* rag3weaver_catalog_new(const char* config, const char* path);
         const char* rag3weaver_create(void* ctx, const char* type, const char* fields);
         const char* rag3weaver_drain(void* ctx);
         const char* rag3weaver_count(void* ctx, const char* type);
         void rag3weaver_catalog_destroy(void* ctx);
     }

     class Weaver {
         void* ctx_;
     public:
         Weaver(std::string config, std::string path);
         ~Weaver();
         std::string create(std::string type, std::string fields);
         std::string drain();
         std::string count(std::string type);
     };

     EMSCRIPTEN_BINDINGS(rag3weaver_wasm) {
         class_<Weaver>("Weaver")
             .constructor<std::string, std::string>()
             .function("create", &Weaver::create)
             .function("drain", &Weaver::drain)
             .function("count", &Weaver::count);
     }
     ```

7. **`tools/wasm/CMakeLists.txt`** — À MODIFIER
   - Ajouter `add_subdirectory(${PROJECT_SOURCE_DIR}/extension/rag3weaver ${CMAKE_BINARY_DIR}/rag3weaver)`
   - Ajouter `src_cpp/weaver_bindings.cpp` aux sources de `rag3db_wasm`
   - Ajouter `target_link_libraries(rag3db_wasm PRIVATE rag3weaver_lib)`

8. **Test build WASM** — À FAIRE
   ```bash
   cd packages/rag3db/build/wasm
   source ~/emsdk/emsdk_env.sh
   emcmake cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_WASM=TRUE \
     -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE
   emmake cmake --build . -j$(nproc)
   ```

### Phase 3 : Test Playwright

9. **`tools/wasm/test/browser/rag3weaver.spec.js`** — À CRÉER
   - Test Playwright qui :
     - Charge rag3db_wasm.js dans un worker
     - Crée un `new Module.Weaver(configJson, "")` (in-memory)
     - Appelle `weaver.create("Document", '{"title":"test","body":"hello"}')` × N
     - Appelle `weaver.drain()`
     - Appelle `weaver.count("Document")` et vérifie `{"count":N}`
     - Optionnel : test avec relations

---

## Architecture rappel (doc 19)

```
JS → embind Weaver class → extern "C" rag3weaver_* (Rust .a)
                                   ↓
                            WasmDbConnection
                                   ↓
                            extern "C" rag3db_* (C API, déjà dans le binaire)
```

Tout dans un seul `rag3db_wasm.js` / `.wasm`.

## Bugs corrigés pendant l'implémentation

- `entity_ref.uuid` → `entity_ref.uuid()` (c'est une méthode, pas un champ, retourne `Result<String, RefError>`)
- `result.pending` → `result.persisted` (FlushResult n'a pas de champ `pending`)

## Fichiers créés/modifiés (cette session)

| Fichier | Action |
|---------|--------|
| `src/wasm_ffi.rs` | Créé — C API FFI + WasmDbConnection + entry points |
| `src/lib.rs` | Modifié — `mod wasm_ffi` + re-export WasmDbConnection |
| `Cargo.toml` | Modifié — feature wasm-emscripten + dep pollster |
| `CMakeLists.txt` (extension/rag3weaver/) | Créé — cargo build WASM + static lib |
| `docs/.../19-rag3weaver-single-wasm-plan.md` | Créé (session précédente dans ce contexte) |

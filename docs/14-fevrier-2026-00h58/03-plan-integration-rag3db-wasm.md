# Plan d'intégration rag3db + tantivy_fts — WASM & Node.js

## Contexte

On avait `kuzu-wasm-exp` qui compile kuzu v0.11.3 en WASM avec les extensions fts, json, vector, algo statiquement linkées. L'extension fts de kuzu ne supporte pas le fuzzy search. On a développé rag3db (fork kuzu v0.11.2.2) avec une extension `tantivy_fts` qui supporte fuzzy, phrase, stems, filter fields, highlights, etc.

**Objectif** : Compiler rag3db en WASM et en Node.js natif, avec tantivy_fts embarqué.

---

## Découverte majeure : tout est déjà en place dans rag3db

En explorant rag3db, on a découvert que **tout le nécessaire existe déjà** :

### 1. `tools/wasm/` — Bindings WASM Embind complets

```
tools/wasm/
├── CMakeLists.txt          # Build executable WASM
├── src_cpp/main.cpp        # Bindings Embind (Database, Connection, QueryResult)
├── src_js/                 # Wrappers JS (database.js, connection.js, query_result.js)
│   ├── index.js
│   ├── sync/               # API synchrone
│   └── ...
├── test/                   # Tests JS (mocha)
├── build.mjs               # Build script
└── package.json
```

**Le main.cpp** expose directement les classes rag3db via Embind :
```cpp
EMSCRIPTEN_BINDINGS(rag3db_wasm) {
    class_<Database>("Database").constructor<std::string, SystemConfig>();
    class_<Connection>("Connection")
        .function("query", &connectionQueryWrapper, allow_raw_pointers())
        .function("prepare", &connectionPrepareWrapper, allow_raw_pointers())
        .function("execute", &connectionExecuteWrapper, allow_raw_pointers());
    class_<QueryResult>("QueryResult")
        .function("getNext", &queryResultGetNext)
        .function("getAsJsArrayOfObjects", &queryResultGetAsEmscriptenArrayOfObjects)
        // ... toutes les méthodes
    ;
}
```

C'est **plus simple et direct** que kuzu-wasm-exp (qui wrappait dans des classes Web* intermédiaires).

### 2. `CMakeLists.txt` principal — Config WASM complète

```cmake
if(${BUILD_WASM})
    # Pthreads
    add_compile_options(-pthread)
    add_link_options(-pthread)
    add_link_options(-sPTHREAD_POOL_SIZE=8)

    # Exceptions
    add_compile_options(-s DISABLE_EXCEPTION_CATCHING=0)
    add_compile_options(-fexceptions)

    # Embind + BigInt
    add_link_options(-lembind)
    add_link_options(-sWASM_BIGINT)

    # Memory
    add_link_options(-sALLOW_MEMORY_GROWTH=1)
    add_link_options(-sMAXIMUM_MEMORY=4GB)
    add_link_options(-sSTACK_SIZE=4MB)

    # Module
    add_link_options(-sMODULARIZE=1)
    add_link_options(-sEXPORT_NAME=rag3db)
    add_link_options(-sEXPORTED_RUNTIME_METHODS=FS,wasmMemory)

    # Filesystem
    add_link_options(-lidbfs.js)
    add_link_options(-lworkerfs.js)
endif()
```

### 3. `extension_config.cmake` — tantivy_fts déjà listé pour WASM

```cmake
if(${BUILD_WASM})
    add_static_link_extension(fts)
    add_static_link_extension(json)
    add_static_link_extension(vector)
    add_static_link_extension(algo)
    add_static_link_extension(tantivy_fts)    # ← DÉJÀ LÀ
endif()
```

### 4. `tantivy_fts/CMakeLists.txt` — Support Emscripten

```cmake
if(EMSCRIPTEN)
    set(CARGO_TARGET "--target" "wasm32-unknown-emscripten")
    set(CARGO_TARGET_DIR ${RUST_WORKSPACE_DIR}/target/wasm32-unknown-emscripten/release)
    set(CARGO_ENV EMCC_CFLAGS=-pthread)
endif()
```

### 5. Auto-enregistrement statique (vérifié)

CMake génère automatiquement `generated_extension_loader.cpp` :
```cpp
tantivy_fts_extension::TantivyFtsExtension extension{};
extension.load(context);
```
Le naming correspond exactement au header `tantivy_fts_extension.h`. `autoLoadLinkedExtensions()` est appelé au démarrage de la DB.

---

## Différences entre tools/wasm/ et kuzu-wasm-exp

| Aspect | tools/wasm/ (rag3db) | kuzu-wasm-exp |
|--------|---------------------|---------------|
| Bindings | Classes rag3db directes | Classes Web* intermédiaires |
| Config WASM | Dans CMakeLists.txt parent | Dans son propre CMakeLists.txt |
| Namespaces | `rag3db::main`, `rag3db::common` | `kuzu::main`, `kuzu::common` |
| Module name | `rag3db_wasm` → `rag3db` | `kuzu_wasm` |
| Extensions | tantivy_fts déjà listé | Pas de tantivy_fts |
| Build script | `build.mjs` (basique) | `build_wasm.sh` (2 passes ESM+CJS) |

---

## Node.js addon (`tools/nodejs_api/`)

Déjà existant et fonctionnel :
- NAPI v6 + cmake-js
- `rag3dbjs.node` linke statiquement contre `librag3db.a`
- `RTLD_GLOBAL` sur Linux pour le dynamic loading d'extensions
- API complète : Database, Connection, PreparedStatement, QueryResult
- Sync + async pour chaque opération

Pour le Node.js natif, tantivy_fts ne sera PAS statiquement linké par défaut (le static linking auto est pour WASM/Android/Swift). Deux options :
- **Option A** : `LOAD EXTENSION '/path/to/tantivy_fts.rag3db_extension'` au runtime
- **Option B** : Dé-commenter `set(EXTENSION_STATIC_LINK_LIST tantivy_fts)` dans `extension_config.cmake`

---

## Problème des threads Rust en WASM

- rayon **compile** sur `wasm32-unknown-emscripten` mais **tombe en fallback single-thread**
- `std::thread` de Rust ne s'appuie pas encore sur les pthreads d'Emscripten par défaut
- Solution : recompiler la stdlib Rust avec `-Z build-std` + `+atomics,+bulk-memory` (nightly)
- Statut : expérimental, peut nécessiter du debug

---

## Plan d'action révisé

### Etape 1 : Build WASM (tout est déjà là)

```bash
cd packages/rag3db

# Installer emscripten si pas fait
# source emsdk/emsdk_env.sh  (ou via le path)

# Build WASM
emcmake cmake -B build/wasm \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_WASM=TRUE \
  -DBUILD_SHELL=FALSE \
  -DBUILD_TESTS=FALSE

emmake make -C build/wasm -j$(nproc)
```

Ceci devrait :
1. Compiler rag3db en WASM
2. Statiquement linker fts, json, vector, algo, tantivy_fts
3. Compiler tantivy_fts Rust vers `wasm32-unknown-emscripten`
4. Produire `build/wasm/tools/wasm/build/rag3db/rag3db_wasm.wasm`

### Etape 2 : Adapter tantivy_fts pour threads WASM (optionnel)

Si on veut des threads Rust réels :
```cmake
# Dans tantivy_fts/CMakeLists.txt
if(EMSCRIPTEN)
    set(CARGO_TOOLCHAIN "+nightly")
    set(CARGO_EXTRA_FLAGS "-Z" "build-std=std,panic_abort")
    list(APPEND CARGO_ENV
        "RUSTFLAGS=-C target-feature=+atomics,+bulk-memory,+mutable-globals")
endif()
```

### Etape 3 : Node.js natif

```bash
cd packages/rag3db/build/release
cmake ../.. -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_EXTENSIONS="tantivy_fts" \
  -DBUILD_NODEJS=TRUE
cmake --build . -j$(nproc)
```

### Etape 4 : Tests

- WASM : tester via Node.js (`node --experimental-wasm-threads`)
- Node.js natif : tester via mocha (`npm test` dans tools/nodejs_api/)

---

## Risques

1. **Compilation Rust WASM** : Le cargo build vers `wasm32-unknown-emscripten` peut rencontrer des problèmes de linking avec les libs C/C++ d'emscripten.
2. **`-Z build-std`** : Expérimental, peut ne pas fonctionner du premier coup.
3. **Taille WASM** : tantivy ajoute ~5-10MB (marginal).
4. **rayon threads** : Fallback single-thread si `-Z build-std` ne fonctionne pas.

## Résumé

| Aspect | WASM | Node.js Natif |
|--------|------|--------------|
| Effort | Build direct (~30min si ça compile) | Build direct (~30min) |
| Infrastructure | Tout existe déjà | Tout existe déjà |
| Threads Rust | Single-thread par défaut, multi avec `-Z build-std` | Full natif |
| Fichiers à modifier | 0 (ou 1 si `-Z build-std`) | 0 (ou 1 si static link) |

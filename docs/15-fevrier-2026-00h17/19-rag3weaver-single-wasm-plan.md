# Rag3Weaver — Plan : Single WASM Module (15 février 2026)

Date : 15 février 2026
Statut : plan avant implémentation

---

## Contexte

Doc 18 posait la question : un seul module WASM (rag3weaver + rag3db ensemble) ou deux modules séparés ?

Décision : **un seul module WASM**. rag3weaver est compilé en static lib pour `wasm32-unknown-emscripten` et linké dans le même binaire emscripten que rag3db + lucivy_fts.

Le pattern existe déjà : lucivy_fts est une lib Rust statique linkée dans le WASM. On fait pareil pour rag3weaver, mais dans l'autre direction (Rust appelle C++ au lieu de C++ appelle Rust).

## Architecture cible

```
┌──────────────────────────────────────────┐
│            JavaScript (browser)          │
│  const w = new Module.Weaver(config)     │
│  w.create("Document", {title: "..."})    │
│  w.drain()                               │
│  w.search("kb1", "query")               │
└────────────────────┬─────────────────────┘
                     │ embind
┌────────────────────┴─────────────────────┐
│          C++ embind wrapper              │
│  class Weaver { rag3weaver_ctx* ctx; }   │
│  calls extern "C" rag3weaver functions   │
└────────────┬────────────┬────────────────┘
             │            │
     ┌───────┴──┐   ┌─────┴──────┐
     │ rag3db   │   │ rag3weaver │
     │ C++ core │   │ Rust .a    │
     │ (existant)│  │ (nouveau)  │
     └───────┬──┘   └─────┬──────┘
             │            │
             │  extern "C" (rag3db.h)
             │◄───────────┘
             │  rag3weaver appelle rag3db
             │  via le C API existant
             │
     ┌───────┴──────────┐
     │ lucivy_fts .a   │
     │ (existant)       │
     └──────────────────┘

Résultat : un seul rag3db_wasm.js / .wasm
```

## FFI rag3weaver → rag3db (C API)

rag3weaver n'utilise **pas** le crate Rust `rag3db` (tools/rust_api/) en WASM — ce crate a un build.rs qui lance cmake, ça créerait un double-build. À la place, on déclare directement les fonctions C nécessaires.

### Fonctions C nécessaires (sous-ensemble de rag3db.h)

Basé sur ce que `Rag3dbConnection` fait en natif, on a besoin de ~15 fonctions :

```rust
// Dans rag3weaver, derrière feature "wasm-emscripten"
extern "C" {
    // Database
    fn rag3db_database_init(
        path: *const c_char,
        buffer_pool_size: u64,
        max_num_threads: u64,
        enable_compression: bool,
        read_only: bool,
        max_db_size: u64,
        auto_checkpoint: bool,
        checkpoint_threshold: u64,
    ) -> rag3db_database;
    fn rag3db_database_destroy(db: *mut rag3db_database);

    // Connection
    fn rag3db_connection_init(db: *mut rag3db_database) -> rag3db_connection;
    fn rag3db_connection_destroy(conn: *mut rag3db_connection);
    fn rag3db_connection_query(
        conn: *mut rag3db_connection,
        query: *const c_char,
    ) -> *mut rag3db_query_result;

    // Prepared statements (pour les paramètres)
    fn rag3db_connection_prepare(
        conn: *mut rag3db_connection,
        query: *const c_char,
    ) -> *mut rag3db_prepared_statement;
    fn rag3db_connection_execute(
        conn: *mut rag3db_connection,
        stmt: *mut rag3db_prepared_statement,
    ) -> *mut rag3db_query_result;
    fn rag3db_prepared_statement_bind_string(
        stmt: *mut rag3db_prepared_statement,
        name: *const c_char,
        value: *const c_char,
    );
    fn rag3db_prepared_statement_bind_int64(
        stmt: *mut rag3db_prepared_statement,
        name: *const c_char,
        value: i64,
    );
    fn rag3db_prepared_statement_bind_double(
        stmt: *mut rag3db_prepared_statement,
        name: *const c_char,
        value: f64,
    );
    fn rag3db_prepared_statement_bind_bool(
        stmt: *mut rag3db_prepared_statement,
        name: *const c_char,
        value: bool,
    );

    // Query result
    fn rag3db_query_result_destroy(result: *mut rag3db_query_result);
    fn rag3db_query_result_is_success(result: *mut rag3db_query_result) -> bool;
    fn rag3db_query_result_get_error_message(result: *mut rag3db_query_result) -> *const c_char;
    fn rag3db_query_result_get_num_columns(result: *mut rag3db_query_result) -> u64;
    fn rag3db_query_result_get_column_name(result: *mut rag3db_query_result, idx: u64) -> *const c_char;
    fn rag3db_query_result_has_next(result: *mut rag3db_query_result) -> bool;
    fn rag3db_query_result_get_next(result: *mut rag3db_query_result) -> *mut rag3db_flat_tuple;

    // Flat tuple (row) → values
    fn rag3db_flat_tuple_get_value(tuple: *mut rag3db_flat_tuple, idx: u64) -> rag3db_value;
    fn rag3db_value_get_data_type(value: rag3db_value) -> rag3db_logical_type;
    fn rag3db_value_get_int64(value: rag3db_value) -> i64;
    fn rag3db_value_get_double(value: rag3db_value) -> f64;
    fn rag3db_value_get_string(value: rag3db_value) -> *const c_char;
    fn rag3db_value_get_bool(value: rag3db_value) -> bool;
    fn rag3db_value_is_null(value: rag3db_value) -> bool;
    fn rag3db_value_destroy(value: rag3db_value);
}
```

### WasmDbConnection

Implémente `DbConnection` en utilisant ces fonctions C :

```rust
pub struct WasmDbConnection {
    conn: *mut rag3db_connection,
}

// SAFETY: WASM est single-threaded
unsafe impl Send for WasmDbConnection {}
unsafe impl Sync for WasmDbConnection {}

#[async_trait]
impl DbConnection for WasmDbConnection {
    async fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        // Appel C sync, wrappé dans async (résout immédiatement)
        let c_str = CString::new(cypher).unwrap();
        let result = unsafe { rag3db_connection_query(self.conn, c_str.as_ptr()) };
        // ... convertir en QueryResult
    }
}
```

Le `async` ici est cosmétique — le future résout immédiatement car l'appel C est synchrone. Mais le trait `DbConnection` reste inchangé, donc tout le code rag3weaver compile sans modification.

## Fonctions rag3weaver exposées à JS

rag3weaver expose des `extern "C"` que le C++ embind appellera :

```rust
// src/wasm_ffi.rs (derrière feature "wasm-emscripten")

#[no_mangle]
pub extern "C" fn rag3weaver_catalog_new(
    config_json: *const c_char,
    db: *mut rag3db_database,
) -> *mut WeaverContext { ... }

#[no_mangle]
pub extern "C" fn rag3weaver_create(
    ctx: *mut WeaverContext,
    entity_type: *const c_char,
    fields_json: *const c_char,
) -> *const c_char { ... }  // retourne UUID

#[no_mangle]
pub extern "C" fn rag3weaver_drain(
    ctx: *mut WeaverContext,
) -> *const c_char { ... }  // retourne stats JSON

#[no_mangle]
pub extern "C" fn rag3weaver_search(
    ctx: *mut WeaverContext,
    kb_name: *const c_char,
    query: *const c_char,
    options_json: *const c_char,
) -> *const c_char { ... }  // retourne résultats JSON

#[no_mangle]
pub extern "C" fn rag3weaver_destroy(ctx: *mut WeaverContext) { ... }
```

Le `WeaverContext` contient un `Catalog` + `WasmDbConnection` + un mini-executor (`pollster::block_on`) pour driver les async calls.

## Wrapper C++ embind

Fichier à ajouter/modifier : `tools/wasm/src_cpp/main.cpp` (ou un nouveau fichier `weaver_bindings.cpp`).

```cpp
#include <emscripten/bind.h>

// Fonctions extern "C" de rag3weaver
extern "C" {
    void* rag3weaver_catalog_new(const char* config_json, void* db);
    const char* rag3weaver_create(void* ctx, const char* entity_type, const char* fields_json);
    const char* rag3weaver_drain(void* ctx);
    const char* rag3weaver_search(void* ctx, const char* kb, const char* query, const char* opts);
    void rag3weaver_destroy(void* ctx);
}

class Weaver {
    void* ctx_;
public:
    Weaver(const std::string& config, Database& db) {
        ctx_ = rag3weaver_catalog_new(config.c_str(), &db);
    }
    ~Weaver() { rag3weaver_destroy(ctx_); }

    std::string create(const std::string& type, const std::string& fields) {
        return rag3weaver_create(ctx_, type.c_str(), fields.c_str());
    }
    std::string drain() { return rag3weaver_drain(ctx_); }
    std::string search(const std::string& kb, const std::string& q, const std::string& opts) {
        return rag3weaver_search(ctx_, kb.c_str(), q.c_str(), opts.c_str());
    }
};

EMSCRIPTEN_BINDINGS(rag3weaver_wasm) {
    class_<Weaver>("Weaver")
        .constructor<std::string, Database&>()
        .function("create", &Weaver::create)
        .function("drain", &Weaver::drain)
        .function("search", &Weaver::search);
}
```

## Build : intégration CMake

En suivant le pattern lucivy_fts, on ajoute dans le build WASM :

```cmake
# extension/rag3weaver/CMakeLists.txt (nouveau)

set(RUST_WORKSPACE_DIR ${CMAKE_CURRENT_SOURCE_DIR})

if(EMSCRIPTEN)
    set(CARGO_TARGET "--target" "wasm32-unknown-emscripten")
    set(CARGO_TARGET_DIR ${RUST_WORKSPACE_DIR}/target/wasm32-unknown-emscripten/release)
    set(CARGO_ENV
        "EMCC_CFLAGS=-pthread -fexceptions -sDISABLE_EXCEPTION_CATCHING=0"
        "RUSTFLAGS=-C target-feature=+atomics,+bulk-memory,+mutable-globals -C panic=abort")
    set(CARGO_TOOLCHAIN "+nightly")
    set(CARGO_EXTRA_FLAGS "-Z" "build-std=std,panic_abort")
    set(CARGO_FEATURES "--features" "wasm-emscripten")
else()
    # natif : pas besoin de ce CMakeLists, on utilise cargo directement
    return()
endif()

set(RAG3WEAVER_STATIC_LIB ${CARGO_TARGET_DIR}/librag3weaver.a)

add_custom_command(
    OUTPUT ${RAG3WEAVER_STATIC_LIB}
    COMMAND ${CMAKE_COMMAND} -E env ${CARGO_ENV}
            cargo ${CARGO_TOOLCHAIN} build --release ${CARGO_TARGET}
                  ${CARGO_EXTRA_FLAGS} ${CARGO_FEATURES}
                  --no-default-features  # pas de candle-embedder en WASM
    WORKING_DIRECTORY ${RUST_WORKSPACE_DIR}
    COMMENT "Building rag3weaver for WASM"
)

add_custom_target(rag3weaver_wasm DEPENDS ${RAG3WEAVER_STATIC_LIB})

add_library(rag3weaver_lib STATIC IMPORTED GLOBAL)
set_target_properties(rag3weaver_lib PROPERTIES
    IMPORTED_LOCATION ${RAG3WEAVER_STATIC_LIB})
```

Et dans `tools/wasm/CMakeLists.txt` (ou le target principal) :

```cmake
target_link_libraries(rag3db_wasm PRIVATE rag3weaver_lib)
```

## Feature flags Cargo

```toml
[features]
default = ["candle-embedder"]
candle-embedder = [...]
rag3db-native = ["dep:rag3db"]
wasm-emscripten = ["dep:pollster"]  # NOUVEAU — active wasm_ffi.rs + WasmDbConnection

[dependencies]
pollster = { version = "0.4", optional = true }  # mini executor block_on
```

## Async → sync : pas de changement

Le code rag3weaver reste async. Le gap est comblé à deux endroits :

1. **WasmDbConnection** : wraps appels C sync dans `async move { ... }` → future immédiat
2. **Points d'entrée FFI** : `pollster::block_on(catalog.create(...))` → drive le future

```rust
// Dans wasm_ffi.rs
#[no_mangle]
pub extern "C" fn rag3weaver_drain(ctx: *mut WeaverContext) -> *const c_char {
    let ctx = unsafe { &mut *ctx };
    let result = pollster::block_on(ctx.catalog.drain());
    //           ^^^^^^^^^^^^^^ tout résout immédiatement car :
    //           - DbConnection = appels C sync wrappés en async
    //           - EventBus = overflow=true, jamais suspendu
    //           - Embedder = fourni par le contexte WASM
    // → block_on ne bloque jamais réellement
    // ...
}
```

## Embedder en WASM

L'embedder est un sujet orthogonal. Options :

1. **Pas d'embedder** : mode FTS-only (BM25), pas de recherche sémantique
2. **Callback via FFI** : rag3weaver expose un hook, le C++ appelle une API JS (fetch vers un serveur d'embedding)
3. **Modèle local WASM** : transformers.js / ONNX dans le même worker — performance dépend du modèle

Pour la première itération, on peut utiliser un `MockEmbedder` ou un `CallbackEmbedder` bridgé via FFI.

## Comparaison avec l'approche deux modules (doc 18)

| | Un seul WASM (ce doc) | Deux WASM (doc 18 approche B) |
|--|----------------------|-------------------------------|
| Modules | 1 (emscripten) | 2 (emscripten + wasm-pack) |
| Communication | FFI directe (pointeurs C) | JS bridge (sérialisation) |
| Overhead | Zéro (même mémoire) | Copie de données entre modules |
| Complexité build | CMake + cargo emscripten | wasm-pack séparé + intégration JS |
| Taille | Identique (même code linké) | Légèrement plus gros (double runtime) |
| Async | block_on sur futures immédiats | Vrai async (deux event loops) |
| Batching | Inchangé (queue/drain) | Inchangé (queue/drain) |

## Dépendances WASM à vérifier

Ces crates doivent compiler pour `wasm32-unknown-emscripten` :

| Crate | Risque | Note |
|-------|--------|------|
| serde, serde_json | Aucun | Universel |
| async-trait | Aucun | Macro proc, pas de runtime |
| thiserror | Aucun | Macro proc |
| blake3 | Faible | A des impls WASM |
| text-splitter | Moyen | Dépend de ahash → getrandom |
| async-broadcast | Faible | No-std compatible |
| tokio (sync only) | **À vérifier** | `tokio::sync::RwLock` utilisé dans Catalog — peut-être remplacer par `std::sync::RwLock` en WASM |
| pollster | Aucun | Conçu pour ça |
| getrandom | Moyen | Backend emscripten ≠ wasm_js, à configurer |

**Point d'attention tokio** : rag3weaver utilise `tokio::sync::RwLock` dans Catalog. Pour WASM emscripten (single-thread), on pourrait utiliser `std::sync::RwLock` derrière un cfg. Ou garder tokio::sync si ça compile pour emscripten.

## Plan d'implémentation

### Phase 1 : Compilation WASM emscripten

1. Ajouter feature `wasm-emscripten` dans Cargo.toml
2. Créer `src/wasm_ffi.rs` avec les déclarations `extern "C"` rag3db
3. Implémenter `WasmDbConnection`
4. Implémenter les fonctions `extern "C"` rag3weaver_*
5. Vérifier : `cargo check --target wasm32-unknown-emscripten --features wasm-emscripten --no-default-features`

### Phase 2 : Intégration build

6. Créer `extension/rag3weaver/CMakeLists.txt` pour le build WASM
7. Ajouter le wrapper C++ embind (`weaver_bindings.cpp`)
8. Linker dans le target WASM principal
9. Build : `emmake cmake --build . -j$(nproc)`

### Phase 3 : Test Playwright

10. Ajouter un test Playwright (`rag3weaver.spec.js`) qui :
    - Crée une DB in-memory via `new Module.Database()`
    - Crée un `new Module.Weaver(config, db)`
    - Insère des documents, drain, vérifie les counts
    - Optionnel : test FTS si lucivy_fts est chargé

---

## Fichiers à créer/modifier

| Fichier | Action |
|---------|--------|
| `extension/rag3weaver/src/wasm_ffi.rs` | Créer — extern "C" rag3db + WasmDbConnection + fonctions rag3weaver_* |
| `extension/rag3weaver/src/lib.rs` | Modifier — `#[cfg(feature = "wasm-emscripten")] mod wasm_ffi;` |
| `extension/rag3weaver/Cargo.toml` | Modifier — feature wasm-emscripten + dep pollster |
| `extension/rag3weaver/CMakeLists.txt` | Créer — build WASM cargo + static lib |
| `tools/wasm/src_cpp/weaver_bindings.cpp` | Créer — embind wrapper C++ |
| `tools/wasm/CMakeLists.txt` | Modifier — linker rag3weaver_lib |
| `tools/wasm/test/browser/rag3weaver.spec.js` | Créer — test Playwright |

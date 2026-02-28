# 05 — Search Async FFI : En cours d'implémentation

## État actuel

**Drain parallelism (task #50) : TERMINÉ** — voir 04-plan-drain-parallel.md + code committé.

**Search async FFI (task #51) : EN COURS** — S1 commencé, reste S1 à finir + S2-S5.

## Ce qui est FAIT dans cette session

### Drain parallelism (B1-B4) — tout vert
- `queue.rs` : `Box<dyn Processor>` → `Arc<dyn Processor>`, méthodes `pub`, `run_processor()` feature-gated, 3 helpers
- `catalog.rs` : `drain_parallel()` feature-gated `wasm-emscripten`
- `wasm_ffi.rs` : `pool` → `Arc<rayon::ThreadPool>`, drain sync+async utilisent `drain_parallel()`
- 271 tests Rust ✅, build WASM ✅, 6 tests Playwright ✅

### Search async S1 — PARTIELLEMENT FAIT
- `search.rs` : ajout `use serde::Serialize;` en haut du fichier — **FAIT**
- Reste à ajouter `#[derive(Serialize)]` + `#[serde(rename_all = "camelCase")]` sur les structs/enums — **PAS ENCORE FAIT**

## Plan complet search async (copie du plan approuvé)

### Fichiers à modifier

| Fichier | Changement |
|---------|-----------|
| `extension/rag3weaver/src/search.rs` | `#[derive(Serialize)]` + `#[serde(rename_all = "camelCase")]` aux types réponse |
| `extension/rag3weaver/src/wasm_ffi.rs` | `rag3weaver_search_async()` + `parse_search_options()` |
| `tools/wasm/src_cpp/weaver_bindings.cpp` | `PendingDrain` → `PendingAsync` générique, `searchAsyncStart`, `asyncPoll/asyncResult` |
| `tools/wasm/test/browser/weaver_worker.js` | `pollAsync()` helper, test search async |
| `tools/wasm/test/browser/rag3weaver.spec.js` | Assertions search |

### S1. search.rs — Ajouter Serialize (PARTIELLEMENT FAIT)

`use serde::Serialize;` ← FAIT

Reste : ajouter derives sur ces types (lignes approximatives du fichier search.rs) :

```rust
// Ligne ~18 — Consistency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Consistency { Immediate, Eventual, Strict }

// Ligne ~52 — SearchType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchType { Hybrid, Semantic, BM25Only }

// Ligne ~118 — SearchResult
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult { pub uuid, pub score, pub entity, pub data, pub chunk }

// Ligne ~128 — ChunkInfo
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkInfo { pub uuid, pub text, pub index, pub score }

// Ligne ~137 — SearchMeta
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMeta { pub query, pub kb, pub search_type, pub consistency, pub partial, ... }

// Ligne ~152 — SearchResponse
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse { pub results, pub meta }
```

NE PAS toucher : SearchOptions, HybridStrategy, BM25Mode (input, parsé manuellement côté FFI).

### S2. wasm_ffi.rs — `rag3weaver_search_async()`

Réutilise `AsyncCallback` existant (ligne ~1057).

```rust
fn parse_search_options(json: &str) -> crate::search::SearchOptions {
    let mut opts = crate::search::SearchOptions::default();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
        if let Some(n) = v.get("limit").and_then(|v| v.as_u64()) { opts.limit = n as usize; }
        if let Some(n) = v.get("offset").and_then(|v| v.as_u64()) { opts.offset = n as usize; }
        if let Some(s) = v.get("consistency").and_then(|v| v.as_str()) {
            opts.consistency = match s {
                "strict" => crate::search::Consistency::Strict,
                "immediate" => crate::search::Consistency::Immediate,
                _ => crate::search::Consistency::Eventual,
            };
        }
        if let Some(n) = v.get("fuzzyDistance").and_then(|v| v.as_u64()) { opts.fuzzy_distance = n as u8; }
        if let Some(s) = v.get("bm25Mode").and_then(|v| v.as_str()) {
            if s == "regex" { opts.bm25_mode = crate::search::BM25Mode::Regex; }
        }
        if let Some(f) = v.get("keywordWeight").and_then(|v| v.as_f64()) { opts.keyword_weight = Some(f); }
    }
    opts
}

#[no_mangle]
pub extern "C" fn rag3weaver_search_async(
    ctx: *const WeaverContext,
    kb_name: *const c_char,
    query: *const c_char,
    options_json: *const c_char,
    callback: AsyncCallback,
    user_data: usize,
) {
    // 1. Null checks, parse C strings
    // 2. Parse options via parse_search_options()
    // 3. Clone ctx.catalog, ctx.pool
    // 4. ctx.pool.spawn(move || {
    //      let mut cat = catalog.lock().unwrap();
    //      let result = futures::executor::block_on(cat.search(&kb, &query, opts));
    //      match result {
    //          Ok(response) => serde_json::to_string(&response),
    //          Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    //      }
    //      callback(return_string_to_c(json), user_data);
    //    });
}
```

**Note** : search() est async, on utilise `block_on()` (pas `drain_parallel`). Le drain de consistance interne à search() utilise `self.queue.drain().await` (séquentiel, OK).

### S3. weaver_bindings.cpp — PendingAsync générique

```cpp
// Renommer PendingDrain → PendingAsync, drain_callback → async_callback
struct PendingAsync { std::string result; std::atomic<bool> done{false}; };
static void async_callback(const char* result_json, uintptr_t user_data) { ... }

// Extern C ajouté :
void rag3weaver_search_async(const void* ctx, const char* kb_name,
    const char* query, const char* options_json,
    async_callback_t callback, uintptr_t user_data);

// Weaver class :
uintptr_t searchAsyncStart(std::string kb, std::string query, std::string optionsJson) {
    auto* pa = new PendingAsync();
    rag3weaver_search_async(ctx_, kb.c_str(), query.c_str(), optionsJson.c_str(),
        async_callback, reinterpret_cast<uintptr_t>(pa));
    return reinterpret_cast<uintptr_t>(pa);
}

// Méthodes generiques (remplacent drainAsyncPoll/drainAsyncResult) :
static bool asyncPoll(uintptr_t handle) { ... }
static std::string asyncResult(uintptr_t handle) { ... }

// Embind :
.function("searchAsyncStart", &Weaver::searchAsyncStart)
.class_function("asyncPoll", &Weaver::asyncPoll)       // remplace drainAsyncPoll
.class_function("asyncResult", &Weaver::asyncResult)    // remplace drainAsyncResult
```

**BREAKING** : `Module.Weaver.drainAsyncPoll(h)` → `Module.Weaver.asyncPoll(h)` dans le JS.

### S4-S5. Tests JS

```javascript
// Helper factorisé (remplace la boucle inline du drain async)
function pollAsync(handle) {
    return new Promise((resolve, reject) => {
        let polls = 0;
        const poll = () => {
            polls++;
            if (Module.Weaver.asyncPoll(handle)) {
                resolve(JSON.parse(Module.Weaver.asyncResult(handle)));
            } else if (polls > 30000) { reject(new Error("timeout")); }
            else { setTimeout(poll, 1); }
        };
        poll();
    });
}

// Test 9: search async
const searchHandle = weaver.searchAsyncStart("main", "test query",
    JSON.stringify({ limit: 5, consistency: "immediate" }));
const searchRes = await pollAsync(searchHandle);
// Attend: { results: [], meta: { query: "test query", kb: "main", ... } }
```

MockEmbedder → zéro vectors → 0 résultats. On vérifie : pas de crash, JSON parseable, meta.query OK.

## Vérification

```bash
cargo test --lib                    # 271 tests
./build_wasm.sh --clean             # Rust WASM lib
emmake cmake --build . --target rag3db_wasm  # Binary
npx playwright test                 # 7+ tests
```

## Contexte technique important

- `CypherValue` a déjà `Serialize, Deserialize` (connection.rs)
- `SearchOptions.filters: HashMap<String, FilterValue>` — FilterValue n'a PAS Serialize/Deserialize, mais on ne touche pas SearchOptions
- `search()` prend `&mut self` (modifie queue pour consistency + embedding_cache)
- En WASM, `catalog` est `Arc<Mutex<Catalog>>` — `.lock()` donne `&mut`
- `AsyncCallback` type déjà défini ligne ~1057 de wasm_ffi.rs : `extern "C" fn(*const c_char, usize)`
- `return_string_to_c()` est thread-local — safe dans rayon::spawn

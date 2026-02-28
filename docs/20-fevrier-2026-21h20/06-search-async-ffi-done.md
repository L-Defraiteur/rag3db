# 06 — Search Async FFI : TERMINÉ

## Résumé

Expose `Catalog::search()` comme FFI async en WASM via le pattern rayon::spawn + callback (même pattern que drain_async). Test E2E complet : JS → C++ embind → Rust FFI → rayon pool → search → serde JSON → callback → JS.

## Fichiers modifiés

### search.rs — Serialize sur types réponse
- `use serde::Serialize;` (déjà présent)
- `#[derive(Serialize)]` + `#[serde(rename_all = "camelCase")]` ajouté sur :
  - `Consistency` (enum)
  - `SearchType` (enum)
  - `SearchResult` (struct)
  - `ChunkInfo` (struct)
  - `SearchMeta` (struct)
  - `SearchResponse` (struct)
- **NON touché** : SearchOptions, HybridStrategy, BM25Mode (input, parsé manuellement)

### wasm_ffi.rs — FFI search async + fixes

#### `parse_search_options(json) -> SearchOptions`
- Parse JSON manuellement (limit, offset, consistency, fuzzyDistance, bm25Mode, keywordWeight)
- Default pour champs non fournis

#### `rag3weaver_search_async(ctx, kb_name, query, options_json, callback, user_data)`
- Null checks, parse C strings
- Clone catalog Arc, spawn sur rayon pool
- `futures::executor::block_on(cat.search(&kb, &q, opts))`
- Sérialise via `serde_json::to_string(&response)` ou JSON erreur
- `callback(return_string_to_c(json), user_data)`

#### Fix : `config.max_num_threads = 2` dans `WasmDbConnection::new()`
- **Problème** : `rag3db_default_system_config()` retourne `maxNumThreads = hardware_concurrency()` (ex: 12-16 sur machine moderne). Avec le rayon pool (4 threads), total = 16-20 threads, dépassant PTHREAD_POOL_SIZE=16 → thread pool exhausted → deadlock.
- **Fix** : forcer `config.max_num_threads = 2` avant `rag3db_database_init()`. Le DB access est sérialisé par Mutex côté Rust, 2 threads DB suffisent. Budget final : 2 (DB) + 4 (rayon) = 6 long-lived sur 16 slots.

#### Fix : binding natif des List via C API
- **Problème** : `bind_param` sérialisait `CypherValue::List` en JSON string → `array_cosine_similarity($embedding)` recevait un String au lieu de `DOUBLE[]` → query error.
- **Fix** : ajout de `cypher_to_c_value()` récursif qui crée des valeurs C natives :
  - `rag3db_value_create_double/int64/bool/string/null` pour les scalaires
  - `rag3db_value_create_list(num_elements, elements**, out**)` pour les listes
  - `rag3db_prepared_statement_bind_value(stmt, name, value)` pour binder
- Map reste en JSON string (pas de C API simple pour maps dynamiques)
- Nouveaux extern C déclarés : `create_bool`, `create_int64`, `create_double`, `create_string`, `create_list`

### weaver_bindings.cpp — PendingAsync générique

- `PendingDrain` → `PendingAsync` (renommé)
- `drain_callback` → `async_callback` (renommé)
- `drainAsyncPoll/drainAsyncResult` → `asyncPoll/asyncResult` (statiques, génériques)
- Ajout `searchAsyncStart(kb, query, optionsJson)` sur classe Weaver
- Ajout extern C : `rag3weaver_search_async`
- Embind : `.function("searchAsyncStart", ...)`, `.class_function("asyncPoll", ...)`, `.class_function("asyncResult", ...)`
- **Breaking JS** : `Module.Weaver.drainAsyncPoll(h)` → `Module.Weaver.asyncPoll(h)` (idem Result)

### weaver_worker.js — Tests

- `pollAsync(Module, handle)` : helper factorisé, retourne `{result, polls}`
- Drain async (test 6) migré vers `pollAsync()` + `asyncPoll/asyncResult`
- **Test 9** : search async
  - Config enrichi : `titleFor: "main"` sur title, `knowledgeBases: { main: {} }`
  - `weaver.searchAsyncStart("main", "test query", JSON.stringify({limit:5, consistency:"immediate"}))`
  - Vérifie : pas de crash, JSON parseable, `meta.query === "test query"`, `meta.kb === "main"`, `results` est array

### rag3weaver.spec.js — Assertions

- Test 9 : `results.search.error` undefined, `results.search.results` array, `meta.query`, `meta.kb`

## Vérification

```
cargo test --lib               → 271 passed
./build_wasm.sh --clean        → OK
emmake cmake --build . --target rag3db_wasm  → OK
npx playwright test            → 6 passed (18.9s)
```

## Architecture threads WASM (budget)

```
PTHREAD_POOL_SIZE = 16 (emscripten, pré-alloué au démarrage)

Long-lived (jamais libérés) :
  rag3db QueryProcessor : 2 threads (forcé, était hardware_concurrency())
  rayon weaver pool     : 4 threads
  Total long-lived      : 6

Disponibles pour threads temporaires : 10
  - futures::executor::block_on (search, drain interne)
  - std::thread::spawn (tests threading)
  - rayon global pool si tantivy l'utilise
```

## Prochaines étapes possibles

- **Explore async** : même pattern pour `search_with_explore()`
- **Embedder réel** : remplacer MockEmbedder par un vrai embedder (ONNX WASM ?)
- **Map binding natif** : `rag3db_value_create_map` pour les filtres search
- **PTHREAD_POOL_SIZE dynamique** : adapter selon `navigator.hardwareConcurrency`

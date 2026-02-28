# 01 — État des lieux (20 février 2026)

## Contexte

Reprise du projet rag3weaver après une semaine de pause. Ce document résume l'état de l'implémentation et les prochaines étapes.

---

## Historique des sessions (docs 15 février)

| Doc | Contenu | Statut |
|-----|---------|--------|
| 19 | Plan : single WASM module (rag3weaver + rag3db en un binaire) | FAIT |
| 20 | Phase 1+2 : Rust WASM FFI + intégration build CMake | FAIT |
| 21 | Phase 2 terminée : binaire 17 MB, classe JS `Weaver` via embind | FAIT |
| 22 | 11 concessions/limites identifiées + solutions proposées | Référence |
| 23 | Validation threading WASM (rayon OK, tokio runtime KO) | FAIT |
| 24 | Architecture async : `rayon::spawn` + Promise JS | FAIT |
| 25 | Implémentation async : drain, Mutex, batch embed, pool dédié | FAIT |
| 26 | Design : opaque handles + drain parallelism + resolutions | À IMPLÉMENTER |

---

## Ce qui fonctionne (validé)

### Rust / WASM
- **Compilation WASM** : `wasm32-unknown-emscripten`, nightly, `-Z build-std`, atomics
- **Thread-safety** : `Mutex<CConnection>`, `Arc<Mutex<Catalog>>`, pool rayon dédié (4 threads)
- **drain() async** : `rayon::spawn` + pattern start/poll/result côté C++/JS
- **Batch embedding** : collect all texts → 1 appel embed → store vectors
- **pollster supprimé** → `futures::executor::block_on`
- **Memory leak corrigé** : `return_string_to_c` → thread-local `RefCell<CString>`

### Build
- **Binaire unique** : `rag3db_wasm.js` (17 MB) avec rag3weaver linké statiquement
- **Extensions WASM** : json, vector, algo, tantivy_fts (fts retirée)

### Tests
- **271 tests Rust** (cargo test --lib) : tous verts
- **6 tests Playwright** : threading (A/B/C), weaver (create/drain/count), IDBFS persistence (x2)
- **Test async drain Playwright** : drainAsyncStart/Poll/Result, 8-9 polls, 2 entités traitées

### API JS actuelle
```js
const weaver = new Module.Weaver(configJson, "");     // in-memory
weaver.create("Document", fieldsJson)                  // → JSON {"uuid":""}
weaver.drain()                                         // → JSON {"processed":N, "failed":N, "persisted":N}
weaver.drainAsyncStart()                               // → handle (uintptr_t)
Module.Weaver.drainAsyncPoll(handle)                   // → bool
Module.Weaver.drainAsyncResult(handle)                 // → JSON result
weaver.count("Document")                               // → JSON {"count":N}
Module.Weaver.version()                                // → "0.1.0"
```

---

## Points résolus du doc 22

| # | Problème | Solution | Statut |
|---|----------|----------|--------|
| 1 | Memory leak `return_string_to_c` | Thread-local RefCell | FAIT |
| 3 | Clé JSON "pending" → "persisted" | Correction nom | FAIT |
| 6 | EmbedProcessor ne batch pas | Refactor 3 phases | FAIT |
| 7 | `unsafe Send+Sync` WasmDbConnection | `Mutex<CConnection>` | FAIT |

---

## Ce qui reste à faire

### Priorité 1 : Opaque Handle (doc 26 — Part A)

**Problème** : `create()` retourne `{"uuid":""}` (vide car Pending). Inutile.

**Solution** : `create()` retourne `i64` handle (index dans `Vec<EntityRef>`).

Étapes :
- A1. Ajouter `refs: Mutex<Vec<EntityRef>>` dans WeaverContext
- A2. `rag3weaver_create()` → retourne `i64` au lieu de `*const c_char`
- A3. Nouveau `rag3weaver_link(from_handle, to_handle, rel_type)` → `i64`
- A4. Nouveau `rag3weaver_get_uuid(handle)` → JSON `{"uuid":"..."}` ou `{"error":"pending"}`
- A5. C++ embind : `create()` retourne int, + `link()`, `getUuid()`
- A6. `drain()` JSON inclut `resolved: [{handle, entity, uuid}]`

### Priorité 2 : Drain Parallelism (doc 26 — Part B)

**Objectif** : `rayon::join(inserts, embeds)` puis links séquentiels.

Étapes :
- B1. `Box<dyn Processor>` → `Arc<dyn Processor>`
- B2. Helpers queue : `take_pending_by_type()`, `return_processed()`, `get_processor()`
- B3. `Catalog::drain_parallel()` (feature-gated `wasm-emscripten`)
- B4. Utiliser dans wasm_ffi (remplacer `block_on(drain())`)

### Priorité 3+ : Différé

| Item | Doc | Notes |
|------|-----|-------|
| Embed callback FFI (Transformers.js) | 22 #5, 24 #2.4 | À rediscuter |
| search() async FFI | 22 #10 | Même pattern que drain async |
| Wrapper JS/TS typé | 22 #8 | Au-dessus de l'embind |
| `rag3weaver_last_error()` | 22 #4 | Thread-local error string |
| Cypher query brut | 22 #9 | `Catalog::execute()` ou séparation |

---

## JS wrapper cible (WeaverRef + Promises)

Design validé le 15 février — drain retourne les résolutions, JS dispatche les Promises :

```js
class WeaverRef {
    constructor(handle, entity) {
        this.handle = handle;
        this.entity = entity;
        this.uuid = new Promise(resolve => { this._resolve = resolve; });
    }
}

// Après drain :
for (const r of result.resolved) {
    const ref = byHandle.get(r.handle);
    if (ref) ref._resolve(r.uuid);
}
```

---

## Repos et branches

| Repo | Branche | Dernier commit |
|------|---------|----------------|
| rag3db | master | `67a8712` feat: rag3weaver — full WASM integration |
| ld-tantivy | main | `30d5c96` feat: add ngram contains scoring utils |

## Commandes build

```bash
# Tests Rust
cd extension/rag3weaver && cargo test --lib

# Build WASM
cd build_wasm && source ~/emsdk/emsdk_env.sh
emcmake cmake ../.. -DCMAKE_BUILD_TYPE=Release -DBUILD_WASM=TRUE \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_BENCHMARK=FALSE
emmake cmake --build . -j$(nproc)

# Tests Playwright
cd tools/wasm && npx playwright test
```

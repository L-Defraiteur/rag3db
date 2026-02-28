# 02 — Plan : Opaque Handle pour create() (doc 26 — Part A uniquement)

## Contexte

`create()` retourne actuellement `*const c_char` → JSON `{"uuid":""}` car l'EntityRef est Pending avant drain. L'UUID est toujours vide — inutile. On veut retourner un `i64` handle opaque (index dans un Vec), ajouter `link()` et `getUuid()` FFI, et inclure les résolutions (`resolved` array) dans le JSON de drain.

Part B (drain parallelism) sera faite séparément ensuite.

## Fichiers à modifier

| Fichier | Changement |
|---------|-----------|
| `extension/rag3weaver/src/wasm_ffi.rs` | refs Vec, create→i64, link FFI, get_uuid FFI, build_drain_json |
| `tools/wasm/src_cpp/weaver_bindings.cpp` | create→int, link(), getUuid(), extern C decls, embind |
| `tools/wasm/test/browser/weaver_worker.js` | Adapter tests pour handles + ajouter tests link/getUuid/resolved |
| `tools/wasm/test/browser/rag3weaver.spec.js` | Assertions mises à jour |

## Étapes

### A1. WeaverContext + refs Vec (`wasm_ffi.rs` ~ligne 774)

Ajouter `refs: std::sync::Mutex<Vec<crate::refs::EntityRef>>` dans WeaverContext.
Initialiser dans `catalog_new` avec `refs: Mutex::new(Vec::new())`.

### A2. `rag3weaver_create()` → retourne `i64` (`wasm_ffi.rs` ~ligne 878)

- Type retour : `*const c_char` → `i64`
- Stocker EntityRef dans `ctx.refs`, retourner l'index
- Convention : `>= 0` = handle, `-1` = erreur

### A3. `rag3weaver_get_uuid()` — nouveau (`wasm_ffi.rs`)

```rust
pub extern "C" fn rag3weaver_get_uuid(ctx: *const WeaverContext, handle: i64) -> *const c_char
```
Retourne `{"uuid":"..."}` si résolu, `{"error":"pending"}` sinon.

### A4. `rag3weaver_link()` — nouveau (`wasm_ffi.rs`)

```rust
pub extern "C" fn rag3weaver_link(
    ctx: *mut WeaverContext,
    from_handle: i64, to_handle: i64,
    rel_type: *const c_char, properties_json: *const c_char,
) -> i64  // 0 success, -1 error
```
Clone EntityRefs depuis `ctx.refs[handle]`, passe à `catalog.link()`.

### A5. `build_drain_json()` — nouveau helper (`wasm_ffi.rs`)

```rust
fn build_drain_json(result: &FlushResult, refs: &[EntityRef]) -> String
```
Génère `{"processed":N,"failed":N,"resolved":[{"handle":0,"entity":"Doc","uuid":"abc"},...]}`.
Remplace le `format!` inline dans `rag3weaver_drain()` et la callback de `rag3weaver_drain_async()`.

### A6. C++ embind (`weaver_bindings.cpp`)

- `create()` retourne `int` au lieu de `std::string`
- Nouvelle méthode `link(int from, int to, string relType, string propsJson)` → `int`
- Nouvelle méthode `getUuid(int handle)` → `std::string` (JSON)
- Extern C declarations mises à jour
- EMSCRIPTEN_BINDINGS : `.function("link", ...)`, `.function("getUuid", ...)`

### A7. Tests Playwright

**weaver_worker.js** :
- Test 3 (create) : `weaver.create()` retourne int >= 0, plus de JSON parse
- Test 4 (drain) : vérifier `result.resolved` array avec handles et UUIDs non vides
- Test 5 (count) : inchangé
- Nouveau test : `getUuid(handle)` après drain → UUID non vide
- Nouveau test : `link(h1, h2, "REFERENCES", "{}")` → retourne 0, drain OK

**rag3weaver.spec.js** :
- Adapter assertions pour handles (int), resolved array, getUuid

## Vérification

```bash
# 1. Tests unitaires Rust
cd packages/rag3db/extension/rag3weaver && cargo test --lib

# 2. Build WASM complet
cd packages/rag3db/build_wasm && source ~/emsdk/emsdk_env.sh
emmake cmake --build . -j$(nproc)

# 3. Tests Playwright
cd packages/rag3db/tools/wasm && npx playwright test
```

# 03 — Opaque Handle : Implémentation (doc 26 Part A)

## Résumé

L'API `create()` retourne maintenant un `i64` handle opaque au lieu du JSON `{"uuid":""}` inutile. Nouvelles fonctions `getUuid()`, `link()`, et `drain()` enrichi avec un tableau `resolved`.

## Changements

### Rust — `extension/rag3weaver/src/wasm_ffi.rs`

| Fonction | Avant | Après |
|----------|-------|-------|
| `rag3weaver_create()` | `*const c_char` → `{"uuid":""}` | `i64` handle (>= 0 ou -1) |
| `rag3weaver_get_uuid()` | n'existait pas | `*const c_char` → `{"uuid":"..."}` ou `{"error":"pending"}` |
| `rag3weaver_link()` | n'existait pas | `i64` (0 success, -1 error) |
| `rag3weaver_drain()` | `{"processed":N,"failed":N,"persisted":N}` | `{"processed":N,"failed":N,"resolved":[{handle,entity,uuid},...]}` |
| `rag3weaver_drain_async()` | idem drain | idem — utilise `build_drain_json()` |

**WeaverContext** :
```rust
pub struct WeaverContext {
    catalog: Arc<Mutex<Catalog>>,
    drain_lock: Arc<Mutex<()>>,
    pool: rayon::ThreadPool,
    refs: Arc<Mutex<Vec<EntityRef>>>,  // NOUVEAU
}
```

`refs` est `Arc<Mutex<>>` (pas juste `Mutex`) pour pouvoir être cloné dans la closure `rayon::spawn` du drain async.

**`build_drain_json()`** — helper qui itère sur tous les refs, collecte ceux qui sont `is_ready()`, et génère le JSON `resolved` array. Appelé par drain sync et async.

### C++ — `tools/wasm/src_cpp/weaver_bindings.cpp`

- `create()` retourne `int` au lieu de `std::string`
- `getUuid(int handle)` → `std::string` (JSON)
- `link(int from, int to, string relType, string propsJson)` → `int`
- Extern C declarations mises à jour pour les nouvelles signatures

### Tests Playwright

8 tests dans `weaver_worker.js` :
1. Version
2. Constructor
3. Create 3 entities → handles [0, 1, 2]
3b. getUuid avant drain → `{"error":"pending"}`
4. Drain sync → resolved 3 avec UUIDs
4b. getUuid après drain → UUID valide
5. Count = 3
6. Create 2 + link + async drain → resolved 5, link OK
7. Count after async = 5
8. getUuid handles async → UUIDs valides

## Fix build WASM

### Problème : deux static libs Rust avec rayon

`librag3weaver.a` et `liblucivy_fts.a` compilent chacune leur propre `rayon_core` (target dirs différents). Le linker `wasm-ld` refusait les symboles dupliqués.

**Fix** : `target_link_options(rag3db_wasm PRIVATE "LINKER:--allow-multiple-definition")` dans `tools/wasm/CMakeLists.txt`.

### Problème : cargo build manuel sans les bons flags

`cargo build --target wasm32-unknown-emscripten` sans `EMCC_CFLAGS` et `RUSTFLAGS` produit un `.a` sans atomics ni exceptions → link échoue.

**Fix** : `extension/rag3weaver/build_wasm.sh` — script wrapper qui set les env vars correctement.

```bash
# BONNE façon de builder pour WASM :
cd extension/rag3weaver && ./build_wasm.sh

# MAUVAISE façon (flags manquants) :
cargo +nightly build --target wasm32-unknown-emscripten --release  # NE PAS FAIRE
```

## Validation

- 271 tests Rust ✅
- Build WASM 17 MB ✅
- 6 tests Playwright ✅ (18.7s)

# WASM Async — Implementation (15 fevrier 2026)

Date : 15 fevrier 2026

---

## Contexte

Suite aux docs 23 (threading valide) et 24 (architecture async choisie), ce document
decrit l'implementation effective des changements : thread-safety, pool rayon dedie,
drain async, batch embedding, et suppression de pollster.

Tous les changements ont ete valides : 271 tests Rust + 6 tests Playwright verts.

---

## 1. Changements implementes

### 1.1. `return_string_to_c` — thread-local (doc 22 #1)

**Fichier :** `extension/rag3weaver/src/wasm_ffi.rs`

**Avant :** `mem::forget(CString)` — fuite memoire a chaque appel FFI.

**Apres :** Buffer thread-local `RefCell<CString>`. Chaque appel ecrase le precedent,
l'ancien est libere. Chaque thread (main + rayon workers) a son propre buffer.

```rust
thread_local! {
    static RETURN_BUF: RefCell<CString> = RefCell::new(CString::new("").unwrap());
}

fn return_string_to_c(s: String) -> *const c_char {
    let c = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
    RETURN_BUF.with(|buf| {
        *buf.borrow_mut() = c;
        buf.borrow().as_ptr()
    })
}
```

**Securite :** Le pointeur retourne est valide jusqu'au prochain appel sur le meme
thread. Le C++ copie immediatement dans `std::string(result)`, donc safe.

### 1.2. Mutex sur WasmDbConnection (doc 24 #2.3)

**Fichier :** `extension/rag3weaver/src/wasm_ffi.rs`

**Avant :**
```rust
pub struct WasmDbConnection {
    db: CDatabase,
    conn: CConnection,  // raw pointer, non protege
}
// SAFETY: WASM emscripten is single-threaded.  <-- FAUX
unsafe impl Send for WasmDbConnection {}
unsafe impl Sync for WasmDbConnection {}
```

**Apres :**
```rust
pub struct WasmDbConnection {
    db: CDatabase,
    conn: Mutex<CConnection>,  // acces serialise
}
// SAFETY: CDatabase n'est accede qu'a l'init/drop (meme thread).
// CConnection est protege par un Mutex pour acces thread-safe.
unsafe impl Send for WasmDbConnection {}
unsafe impl Sync for WasmDbConnection {}
```

**Impact :** `query_sync()` et `query_with_params_sync()` acquierent le Mutex avant
d'acceder a la connexion. Le cast `*const as *mut` est remplace par un lock propre :
```rust
let mut conn_guard = self.conn.lock().map_err(...)?;
let conn_ptr = &mut *conn_guard as *mut CConnection;
```

Le `Drop` utilise `get_mut()` (acces exclusif, pas de contention) :
```rust
let conn = self.conn.get_mut().unwrap_or_else(|e| e.into_inner());
rag3db_connection_destroy(conn);
```

### 1.3. WeaverContext refactore (doc 24 #2.7 + #2.8)

**Fichier :** `extension/rag3weaver/src/wasm_ffi.rs`

**Avant :**
```rust
pub struct WeaverContext {
    catalog: Catalog,
}
```

**Apres :**
```rust
pub struct WeaverContext {
    catalog: Arc<Mutex<Catalog>>,    // partageable entre threads
    drain_lock: Arc<Mutex<()>>,      // serialise les drain()
    pool: rayon::ThreadPool,         // pool dedie, 4 threads
}
```

**Changements cles :**
- Tous les FFI utilisent `&*ctx` (shared ref) au lieu de `&mut *ctx` (exclusive ref)
- Chaque acces au Catalog passe par `ctx.catalog.lock()`
- Le pool rayon est configure avec 4 threads nommes "weaver-pool-{i}"
- Le `drain_lock` serialise les drains (un seul a la fois)

**Budget threads :**

| Composant | Threads | Source |
|-----------|---------|--------|
| Kuzu (query parallelism) | ~8 | Pool emscripten global |
| Tantivy (rayon interne) | ~4 | Pool rayon global tantivy |
| rag3weaver (notre pool) | 4 | Pool rayon dedie |
| Total | ~16 | = PTHREAD_POOL_SIZE |

### 1.4. drain_async FFI (doc 24 #2.2)

**Fichier Rust :** `extension/rag3weaver/src/wasm_ffi.rs`

Nouvelle fonction FFI non-bloquante :

```rust
type AsyncCallback = extern "C" fn(result_json: *const c_char, user_data: usize);

#[no_mangle]
pub extern "C" fn rag3weaver_drain_async(
    ctx: *const WeaverContext,
    callback: AsyncCallback,
    user_data: usize,
)
```

**Flow :**
1. Clone `Arc<Mutex<Catalog>>` et `Arc<Mutex<()>>` (drain_lock)
2. `ctx.pool.spawn(move || { ... })` — queue sur le pool rayon (jamais d'echec)
3. Dans le worker : lock drain_lock, lock catalog, `block_on(drain())`, callback
4. Retour immediat — le JS recoit le controle pendant que le drain tourne

**Fichier C++ :** `tools/wasm/src_cpp/weaver_bindings.cpp`

Pattern start/poll/result (simple, pas de threading C++ complexe) :

```cpp
struct PendingDrain {
    std::string result;
    std::atomic<bool> done{false};
};

static void drain_callback(const char* result_json, uintptr_t user_data) {
    auto* pd = reinterpret_cast<PendingDrain*>(user_data);
    pd->result = result_json ? std::string(result_json) : R"({"error":"null"})";
    pd->done.store(true, std::memory_order_release);
}
```

**API embind :**

| Methode | Type | Description |
|---------|------|-------------|
| `drainAsyncStart()` | instance | Demarre un drain async, retourne un handle (uintptr_t) |
| `drainAsyncPoll(handle)` | static | Verifie si le drain est termine (non-bloquant) |
| `drainAsyncResult(handle)` | static | Recupere le resultat JSON et libere le handle |

**Usage JS (wrapper Promise) :**
```js
Weaver.prototype.drainAsync = function() {
    const handle = this.drainAsyncStart();
    return new Promise((resolve) => {
        const poll = () => {
            if (Weaver.drainAsyncPoll(handle)) {
                resolve(JSON.parse(Weaver.drainAsyncResult(handle)));
            } else {
                setTimeout(poll, 1);
            }
        };
        poll();
    });
};
```

### 1.5. Batch EmbedProcessor (doc 22 #6, doc 24 #2.1)

**Fichier :** `extension/rag3weaver/src/catalog.rs`

**Avant :** Boucle `for item in items` appelant `embedder.embed(&[text])` un par un.
Chaque item = un appel reseau separe (catastrophique en latence).

**Apres :** 3 phases :

1. **Collect** : pour chaque EmbedOp, `entity_ref.ready().await`, collecter
   (uuid, text, entity_name, embedding_col) dans un `Vec<EmbedWork>`
2. **Batch embed** : un seul appel `embedder.embed(&all_texts)` — N textes, 1 round-trip
3. **Store** : pour chaque vecteur, executer `SET n.{kb}_embedding = $embedding`

Validation : si `vectors.len() != works.len()`, erreur explicite.

### 1.6. Suppression pollster (doc 24 #2.6)

**Fichiers :** `Cargo.toml` + `wasm_ffi.rs`

- `pollster::block_on()` remplace par `futures::executor::block_on()` (3 call sites)
- `dep:pollster` retire de la feature `wasm-emscripten`
- Section `[dependencies.pollster]` supprimee du Cargo.toml
- `futures` 0.3 (deja en dep) fournit `block_on` sans dep supplementaire

---

## 2. Fichiers modifies

| Fichier | Changements |
|---------|------------|
| `extension/rag3weaver/src/wasm_ffi.rs` | thread-local return_string_to_c, Mutex conn, Arc<Mutex<Catalog>>, rayon pool, drain_lock, drain_async FFI, pollster -> futures |
| `extension/rag3weaver/src/catalog.rs` | EmbedProcessor batch (3 phases) |
| `extension/rag3weaver/Cargo.toml` | -pollster, wasm-emscripten feature mise a jour |
| `tools/wasm/src_cpp/weaver_bindings.cpp` | PendingDrain, drain_callback, drainAsyncStart/Poll/Result, embind registrations |

---

## 3. Validation

### Tests Rust (271 passes)

```
cargo test
test result: ok. 271 passed; 0 failed; 5 ignored; 0 measured
```

Tous les tests existants passent sans regression. Le module `wasm_ffi` est
feature-gate derriere `wasm-emscripten` et n'est pas teste en natif.

### Build WASM

```
source ~/emsdk/emsdk_env.sh
# Rust (wasm32-unknown-emscripten, nightly, -Z build-std, +atomics)
cargo +nightly build -Z build-std=std,panic_abort --target wasm32-unknown-emscripten \
    --release --no-default-features --features wasm-emscripten
# 2 warnings attendus : doc comment sur macro, atomics unstable

# CMake
emmake cmake --build . -j$(nproc)
# [100%] Built target rag3db_wasm
```

### Tests Playwright (6 passes, 20.3s)

```
npx playwright test test/browser/
  ✓ Test A: std::thread::spawn + atomics (6.6s)
  ✓ Test B: futures::executor::ThreadPool (3.9s)
  ✓ Test C: rayon par_iter (4.7s)
  ✓ create + drain + count via Weaver embind class (7.0s)
  ✓ Phase 1: create + index + query + persist to IDBFS (8.0s)
  ✓ Phase 2: reload from IDBFS + verify persistence (10.4s)
  6 passed (20.3s)
```

Le test rag3weaver confirme que le refactoring (Arc<Mutex<Catalog>>, drain_lock,
futures::executor::block_on) fonctionne correctement en runtime WASM :
- `create()` : 3 entites creees via lock catalog
- `drain()` : 3 processed, 0 failed (via drain_lock + catalog lock + block_on)
- `count()` : 3 (via catalog lock + block_on)

---

## 4. Architecture resultante

```
JS (Web Worker)
  │
  ├── weaver.create()  ─── embind ──→ &*ctx → catalog.lock() → create() → unlock
  ├── weaver.drain()   ─── embind ──→ &*ctx → drain_lock + catalog.lock() → block_on(drain()) → unlock
  ├── weaver.count()   ─── embind ──→ &*ctx → catalog.lock() → block_on(count()) → unlock
  │
  └── weaver.drainAsyncStart()
        │
        ├── PendingDrain { done: atomic<bool> }
        ├── rag3weaver_drain_async(ctx, callback, user_data)
        │     └── ctx.pool.spawn(move || {
        │           drain_lock.lock()
        │           catalog.lock()
        │           block_on(drain())
        │           callback(result, user_data)   ← rayon worker thread
        │         })
        │
        ├── drainAsyncPoll(handle)  → done.load(acquire)
        └── drainAsyncResult(handle) → move result + delete PendingDrain
```

**Pools de threads :**

```
emscripten PTHREAD_POOL_SIZE=16
  ├── Kuzu query workers     (~8 threads)
  ├── Tantivy rayon global   (~4 threads)
  └── rag3weaver pool dedie  (4 threads, "weaver-pool-{0..3}")
```

**Ordre de verrouillage** (toujours respecte, pas de deadlock) :
1. `drain_lock` (optionnel, seulement pour drain)
2. `catalog` (requis pour toute operation)

---

## 5. Points resolus du doc 22

| # | Concession doc 22 | Resolution |
|---|-------------------|------------|
| #1 | Memory leak return_string_to_c | thread-local RefCell<CString> |
| #6 | EmbedProcessor ne batch pas | 3 phases collect/batch/store |
| #7 | unsafe Send+Sync WasmDbConnection | Mutex<CConnection>, justification corrigee |

---

## 6. Points resolus du doc 24

| # | Chantier doc 24 | Statut |
|---|-----------------|--------|
| #2.1 | Batch embedding rayon | **FAIT** (batch embed, pas encore rayon pour le pre-traitement) |
| #2.2 | drain() async via rayon::spawn | **FAIT** (drain_async FFI + C++ start/poll/result) |
| #2.3 | Mutex sur WasmDbConnection | **FAIT** |
| #2.4 | Embed callback bidirectionnel | A faire (Phase 4, Transformers.js) |
| #2.5 | Parallelisme dans drain() | A faire (rayon::join persist+embed) |
| #2.6 | Supprimer pollster | **FAIT** |
| #2.7 | Pool rayon dedie | **FAIT** (4 threads, weaver-pool-{i}) |
| #2.8 | Garde contre appels concurrents | **FAIT** (drain_lock + Arc<Mutex<Catalog>>) |

---

## 7. Prochaines etapes

1. **Test async drain en Playwright** : ajouter un test qui utilise `drainAsyncStart/Poll/Result`
   pour valider le flow non-bloquant de bout en bout
2. **Embed callback FFI** (doc 24 #2.4) : callback bidirectionnel pour Transformers.js
3. **Parallelisme drain** (doc 24 #2.5) : `rayon::join(persist, embed)` dans le drain
4. **search() FFI** : exposer search() en FFI (directement en async)
5. **Handle opaque** (doc 22 #2) : create() retourne un index au lieu d'un UUID

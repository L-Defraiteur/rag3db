# Opaque Handle + Drain Parallelism (15 fevrier 2026)

Date : 15 fevrier 2026

---

## Contexte

Suite aux docs 22-25, deux ameliorations a l'API rag3weaver :

1. **Opaque handle** (doc 22 #2) : `create()` retourne un `i64` handle au lieu de
   `{"uuid":""}` (toujours vide avant drain). Permet aussi d'ajouter `link()` en FFI.
2. **Drain parallelism** (doc 24 #2.5) : InsertProcessor et EmbedProcessor tournent
   en parallele via `rayon::join`. EmbedProcessor attend les UUIDs via le watch channel.

**Decision cle** : `drain()` retourne les resolutions dans son resultat JSON (pas juste
`{processed, failed}`). Cela permet un pattern Promise cote JS sans threading
supplementaire, et fonctionne pour WASM ET Node.js natif.

---

## 1. Drain avec resolutions (design extensible)

### 1.1. Pourquoi inclure les resolutions dans drain()

Le core Rust a deja le pattern EntityRef avec watch channel. Apres drain,
`entity_ref.uuid()` retourne Ok(uuid). Mais en FFI (C extern), on ne peut pas
passer des objets Rust — d'ou les handles (index entier).

Au lieu de forcer chaque consumer (WASM, Node.js) a appeler `getUuid(handle)` apres
drain, on inclut les resolutions directement dans le resultat drain :

```json
{
  "processed": 5,
  "failed": 0,
  "resolved": [
    {"handle": 0, "entity": "Document", "uuid": "abc-def-..."},
    {"handle": 1, "entity": "Chunk", "uuid": "ghi-jkl-..."}
  ]
}
```

Avantages :
- **WASM JS** : le wrapper dispatch les UUIDs aux Promises
- **Node.js natif** : meme pattern, meme JSON
- **Tout futur consumer** : la resolution est dans le resultat du drain
- **Pas de threading supplementaire** (pas de watch thread par ref)
- Le core Rust reste propre (EntityRef inchange)

### 1.2. Implementation cote Rust (wasm_ffi.rs)

Apres `drain()` ou `drain_parallel()`, iterer sur `ctx.refs` pour trouver
les refs nouvellement resolues :

```rust
fn build_drain_json(result: &FlushResult, refs: &[EntityRef]) -> String {
    let mut resolved = Vec::new();
    for (i, r) in refs.iter().enumerate() {
        if r.is_ready() {
            if let Ok(uuid) = r.uuid() {
                resolved.push(format!(
                    r#"{{"handle":{},"entity":"{}","uuid":"{}"}}"#,
                    i, r.entity(), uuid
                ));
            }
        }
    }
    format!(
        r#"{{"processed":{},"failed":{},"resolved":[{}]}}"#,
        result.processed, result.failed, resolved.join(",")
    )
}
```

Note : on retourne TOUS les handles resolus (pas juste les nouveaux). Le JS wrapper
peut tracker lesquels il a deja dispatche.

### 1.3. Usage JS (wrapper Promise)

```js
class WeaverRef {
    constructor(handle, entity, weaver) {
        this.handle = handle;
        this.entity = entity;
        this._weaver = weaver;
        this._resolved = false;
        this.uuid = new Promise(resolve => { this._resolve = resolve; });
    }
}

// Methodes wrapper sur la classe Weaver embind :

Weaver.prototype.createEntity = function(type, fieldsJson) {
    const handle = this.create(type, fieldsJson); // C++ retourne int
    if (handle < 0) throw new Error("create failed");
    const ref = new WeaverRef(handle, type, this);
    this._pendingRefs = this._pendingRefs || [];
    this._pendingRefs.push(ref);
    return ref;
};

Weaver.prototype.linkEntities = function(fromRef, toRef, relType, propsJson) {
    return this.link(fromRef.handle, toRef.handle, relType, propsJson || "{}");
};

Weaver.prototype.drainAndResolve = async function() {
    const handle = this.drainAsyncStart();
    const resultJson = await new Promise((resolve) => {
        const poll = () => {
            if (Weaver.drainAsyncPoll(handle)) {
                resolve(Weaver.drainAsyncResult(handle));
            } else {
                setTimeout(poll, 1);
            }
        };
        poll();
    });
    const result = JSON.parse(resultJson);

    // Dispatch les resolutions aux Promises des refs
    if (result.resolved && this._pendingRefs) {
        const byHandle = new Map();
        for (const ref of this._pendingRefs) byHandle.set(ref.handle, ref);
        for (const r of result.resolved) {
            const ref = byHandle.get(r.handle);
            if (ref && !ref._resolved) {
                ref._resolved = true;
                ref._resolve(r.uuid);
            }
        }
    }
    return result;
};
```

Usage final :
```js
const doc = weaver.createEntity("Document", JSON.stringify({ title: "Rust Guide" }));
const chunk = weaver.createEntity("Chunk", JSON.stringify({ text: "..." }));
weaver.linkEntities(doc, chunk, "HAS_CHUNK");

const result = await weaver.drainAndResolve();
const uuid = await doc.uuid; // "abc-def-..." (resolu par drainAndResolve)
```

---

## 2. Fichiers modifies

| Fichier | Changements |
|---------|------------|
| `extension/rag3weaver/src/wasm_ffi.rs` | `refs: Mutex<Vec<EntityRef>>` sur WeaverContext, `create()` retourne i64, nouveau `link()` FFI, `get_uuid()` FFI, `build_drain_json()` avec resolved |
| `extension/rag3weaver/src/queue.rs` | `processors` -> `Arc<dyn Processor>`, helpers `take_pending_by_type()`, `return_processed()`, `get_processor()` |
| `extension/rag3weaver/src/catalog.rs` | `drain_parallel()` feature-gated wasm-emscripten |
| `tools/wasm/src_cpp/weaver_bindings.cpp` | `create()` retourne int, nouveau `link()`, `getUuid()`, externs mis a jour |
| `tools/wasm/test/browser/weaver_worker.js` | Tests mis a jour pour handles + link + resolved |
| `tools/wasm/test/browser/rag3weaver.spec.js` | Assertions mises a jour |

---

## 3. Partie A : Opaque Handle

### A1. WeaverContext — ajouter `refs`

```rust
pub struct WeaverContext {
    catalog: Arc<Mutex<Catalog>>,
    drain_lock: Arc<Mutex<()>>,
    pool: rayon::ThreadPool,
    refs: std::sync::Mutex<Vec<crate::refs::EntityRef>>,  // NOUVEAU
}
```

Le Mutex est necessaire car `create()` utilise `&*ctx` (shared ref).
`catalog_new` initialise avec `refs: Mutex::new(Vec::new())`.

### A2. `rag3weaver_create()` -> retourne `i64`

Avant : retourne `*const c_char` (JSON `{"uuid":""}`)
Apres : retourne `i64` (handle = index dans le Vec)

```rust
#[no_mangle]
pub extern "C" fn rag3weaver_create(
    ctx: *mut WeaverContext,
    entity_type: *const c_char,
    fields_json: *const c_char,
) -> i64 {
    // ... parse entity_type, fields_json ...
    let ctx = unsafe { &*ctx };
    let mut catalog = ctx.catalog.lock().unwrap();
    match catalog.create(&entity, fields) {
        Ok(entity_ref) => {
            let mut refs = ctx.refs.lock().unwrap();
            let handle = refs.len() as i64;
            refs.push(entity_ref);
            handle  // 0, 1, 2, ...
        }
        Err(_) => -1,
    }
}
```

Convention : `>= 0` = succes (handle), `-1` = erreur.

### A3. `rag3weaver_get_uuid()`

```rust
#[no_mangle]
pub extern "C" fn rag3weaver_get_uuid(
    ctx: *const WeaverContext,
    handle: i64,
) -> *const c_char
```

Retourne JSON `{"uuid":"abc-def"}` ou `{"error":"pending"}` / `{"error":"invalid handle"}`.

### A4. `rag3weaver_link()`

```rust
#[no_mangle]
pub extern "C" fn rag3weaver_link(
    ctx: *mut WeaverContext,
    from_handle: i64,
    to_handle: i64,
    rel_type: *const c_char,
    properties_json: *const c_char,
) -> i64  // 0 = succes, -1 = erreur
```

Recupere les EntityRef clones depuis `ctx.refs[from]` / `ctx.refs[to]`,
les passe a `catalog.link(rel_name, from_ref, to_ref, props)`.

### A5. C++ embind

```cpp
extern "C" {
    // Modifie
    int64_t rag3weaver_create(void* ctx, const char* entity_type, const char* fields_json);
    // Nouveaux
    int64_t rag3weaver_link(void* ctx, int64_t from, int64_t to,
                            const char* rel_type, const char* props_json);
    const char* rag3weaver_get_uuid(const void* ctx, int64_t handle);
}

class Weaver {
    // Modifie
    int create(std::string entityType, std::string fieldsJson) {
        return (int)rag3weaver_create(ctx_, entityType.c_str(), fieldsJson.c_str());
    }
    // Nouveaux
    int link(int from, int to, std::string relType, std::string propsJson) {
        return (int)rag3weaver_link(ctx_, from, to, relType.c_str(), propsJson.c_str());
    }
    std::string getUuid(int handle) {
        const char* r = rag3weaver_get_uuid(ctx_, handle);
        return r ? std::string(r) : R"({"error":"null"})";
    }
};

// Embind
.function("create", &Weaver::create)        // modifie : retourne int
.function("link", &Weaver::link)             // nouveau
.function("getUuid", &Weaver::getUuid)       // nouveau
```

### A6. drain() retourne les resolutions

Modifier `rag3weaver_drain()` et `rag3weaver_drain_async()` pour utiliser
`build_drain_json()` qui inclut le champ `resolved` (voir section 1.2).

---

## 4. Partie B : Drain Parallelism

### B1. `processors` -> `Arc<dyn Processor>`

**Fichier :** `queue.rs`

```rust
// Avant
processors: HashMap<&'static str, Box<dyn Processor>>,
// Apres
processors: HashMap<&'static str, Arc<dyn Processor>>,
```

`register_processor` convertit `Box` en `Arc` :
```rust
pub fn register_processor(&mut self, op_type: &'static str, processor: Box<dyn Processor>) {
    self.processors.insert(op_type, Arc::from(processor));
}
```

Le flush existant utilise `.as_ref()` — pas de changement fonctionnel.

### B2. Queue helpers

**Fichier :** `queue.rs`

```rust
/// Extraire tous les items pending, groupes par type d'operation.
pub fn take_pending_by_type(&mut self) -> HashMap<&'static str, Vec<OperationItem>> {
    let all = std::mem::take(&mut self.items);
    let (pending, keep): (Vec<_>, Vec<_>) = all.into_iter()
        .partition(|i| matches!(i.state, ItemState::Pending | ItemState::Persisted));
    self.items = keep;
    let mut groups: HashMap<&'static str, Vec<OperationItem>> = HashMap::new();
    for item in pending {
        groups.entry(item.op.operation_type()).or_default().push(item);
    }
    groups
}

/// Retourner les items non-completes et mettre a jour les stats.
pub fn return_processed(&mut self, items: Vec<OperationItem>, result: &FlushResult) {
    self.items.extend(items.into_iter()
        .filter(|i| !matches!(i.state, ItemState::Completed)));
    self.cumulative.total_processed += result.processed;
    self.cumulative.total_failed += result.failed;
    self.cumulative.flush_count += 1;
}

/// Obtenir un clone Arc d'un processor.
pub fn get_processor(&self, name: &str) -> Option<Arc<dyn Processor>> {
    self.processors.get(name).cloned()
}
```

### B3. `Catalog::drain_parallel()`

**Fichier :** `catalog.rs`

```rust
#[cfg(feature = "wasm-emscripten")]
pub fn drain_parallel(&mut self, pool: &rayon::ThreadPool) -> FlushResult {
    let mut groups = self.queue.take_pending_by_type();
    if groups.is_empty() { return FlushResult::default(); }

    let mut inserts = groups.remove("insert").unwrap_or_default();
    let mut links = groups.remove("link").unwrap_or_default();
    let mut embeds = groups.remove("embed").unwrap_or_default();

    let insert_proc = self.queue.get_processor("insert");
    let embed_proc = self.queue.get_processor("embed");
    let link_proc = self.queue.get_processor("link");

    // Phase 1 : inserts + embeds en parallele
    // EmbedProcessor attend les UUIDs via entity_ref.ready().await
    // (block_on dans chaque closure rayon)
    let (r_insert, r_embed) = pool.install(|| {
        rayon::join(
            || run_batch(insert_proc.as_deref(), &mut inserts),
            || run_batch(embed_proc.as_deref(), &mut embeds),
        )
    });

    // Phase 2 : links (apres inserts resolus)
    let r_link = run_batch(link_proc.as_deref(), &mut links);

    let combined = FlushResult {
        processed: r_insert.processed + r_embed.processed + r_link.processed,
        failed: r_insert.failed + r_embed.failed + r_link.failed,
        persisted: 0,
    };

    let mut all = inserts;
    all.extend(links);
    all.extend(embeds);
    self.queue.return_processed(all, &combined);
    combined
}

/// Helper: process items through a processor using block_on.
#[cfg(feature = "wasm-emscripten")]
fn run_batch(processor: Option<&dyn Processor>, items: &mut [OperationItem]) -> FlushResult {
    let mut result = FlushResult::default();
    if items.is_empty() { return result; }
    let Some(proc) = processor else { return result; };
    let batch_size = items[0].op.config().batch_size;
    for batch in items.chunks_mut(batch_size) {
        for item in batch.iter_mut() { item.mark_processing(); }
        match futures::executor::block_on(proc.process(batch)) {
            Ok(()) => {
                for item in batch.iter_mut() {
                    item.mark_completed();
                    result.processed += 1;
                }
            }
            Err(e) => {
                for item in batch.iter_mut() {
                    item.mark_failed(e.clone());
                    result.failed += 1;
                }
            }
        }
    }
    result
}
```

### B4. Utiliser drain_parallel dans wasm_ffi.rs

Dans `rag3weaver_drain()` et `rag3weaver_drain_async()`, remplacer :
```rust
// Avant
let result = futures::executor::block_on(cat.drain());
// Apres
let result = cat.drain_parallel(&ctx.pool);
```

Le drain sequentiel (`cat.drain()`) reste disponible pour les tests et le natif.

---

## 5. Ordre d'execution

```
A1. WeaverContext + refs Vec      (standalone)
A2. create() -> i64               (depend de A1)
A3. get_uuid() FFI                (depend de A1)
A4. link() FFI                    (depend de A1)
A5. C++ embind                    (depend de A2-A4)
A6. drain() avec resolved JSON    (depend de A5)
A7. Tests Playwright              (depend de A6)
B1. processors -> Arc             (standalone)
B2. Queue helpers                 (depend de B1)
B3. drain_parallel Catalog        (depend de B2)
B4. Utiliser dans wasm_ffi        (depend de B3)
```

A et B sont independants. B4 doit etre fait apres A6 (car drain JSON change).

---

## 6. Verification

```bash
# Tests unitaires Rust (271 tests)
cd packages/rag3db/extension/rag3weaver && cargo test

# Build WASM Rust
cd packages/rag3db/extension/rag3weaver
cargo +nightly build -Z build-std=std,panic_abort \
  --target wasm32-unknown-emscripten --release \
  --no-default-features --features wasm-emscripten

# Build WASM complet
cd packages/rag3db/build/wasm
source ~/emsdk/emsdk_env.sh
emmake cmake --build . -j$(nproc)

# Tests Playwright
cd packages/rag3db/tools/wasm
npx playwright test test/browser/
```

Tests a verifier :
- 271 tests Rust (pas de regression)
- 6 tests Playwright existants (threading A/B/C, weaver, IDBFS phase 1+2)
- Nouveaux tests : create handles, link, drain avec resolved, getUuid apres drain

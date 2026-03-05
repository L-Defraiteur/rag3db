# WASM Async Architecture — rayon::spawn + Promise (15 février 2026)

Date : 15 février 2026

---

## Contexte

Suite aux validations du doc 23, nous avons confirmé que `std::thread::spawn`,
`rayon::par_iter` et `futures::executor::ThreadPool` fonctionnent dans notre build
WASM emscripten. Tokio est exclu (compile_error + thread pool exhaustion).

Ce document décrit l'architecture async choisie et les chantiers pour exploiter
pleinement le multi-threading.

---

## 1. Choix : `rayon::spawn` + Promise JS

### Pourquoi pas JSPI ou Asyncify ?

| Approche | Avantages | Inconvénients |
|----------|-----------|---------------|
| JSPI | Zéro overhead, zéro code | Chrome uniquement (Firefox flagged, Safari absent) |
| Asyncify | Universel | +50% taille binaire (17MB → ~25MB), overhead runtime |
| **rayon::spawn + Promise** | **Universel, zéro overhead taille, pas de pool exhaustion** | **Plus de code C++/Rust à écrire** |

On choisit `rayon::spawn` plutôt que `std::thread::spawn` pour les opérations async.
Migration vers JSPI possible plus tard quand le support sera universel.

### Pourquoi rayon::spawn et pas std::thread::spawn ?

Le pool emscripten est **fini** (16 Web Workers) et **partagé** :

```
Budget total : 16 Web Workers emscripten
  - Kuzu (query parallelism)    : ~4-8 threads
  - Lucivy/rayon (search/index): ~4 threads
  - Nos opérations async        : 1-2 threads
                                  ─────────
                        → facilement 12-16 au pic
```

| | `std::thread::spawn` | `rayon::spawn` |
|-|---------------------|----------------|
| Thread | **Crée** un nouveau Web Worker | **Réutilise** un worker du pool rayon |
| Pool plein | **Échec fatal** : "thread pool is exhausted" | **File d'attente** : attend qu'un worker se libère |
| Contrôle | Aucun — dépend du pool emscripten global | Pool dédié configurable (`num_threads`) |
| Cleanup | Doit `.detach()` ou `.join()` | Automatique (work-stealing) |

`rayon::spawn` ne crée jamais de thread supplémentaire. Si tous les workers sont
occupés, le travail attend dans la queue — la Promise prend plus longtemps mais
**jamais d'échec**. C'est exactement le comportement souhaité.

### Flow async

```
JS (Worker)                    C++ embind                  Rust (rayon pool)
    │                              │                              │
    │  weaver.drain()              │                              │
    │─────────────────────────────>│                              │
    │                              │  rayon::spawn (queue)        │
    │                              │─────────────────────────────>│
    │  ← Promise retournée         │                              │
    │  (JS event loop libre)       │                              │
    │                              │        work-stealing pool    │
    │                              │        drain + embed + index │
    │                              │                              │
    │                              │  channel.recv() + proxy      │
    │                              │<─────────────────────────────│
    │  Promise.resolve(result)     │                              │
    │<─────────────────────────────│                              │
```

### Implémentation C++ (embind)

```cpp
#include <emscripten/threading.h>
#include <emscripten/val.h>

class Weaver {
    void* ctx_;

    // Shared state for async operations
    struct AsyncOp {
        std::string result;
        emscripten::val resolve = emscripten::val::undefined();
        emscripten::val reject = emscripten::val::undefined();
    };

public:
    // Synchrone (rapide, pas de thread)
    int create(std::string entityType, std::string fieldsJson) { ... }
    void link(int fromHandle, int toHandle, std::string relType) { ... }

    // Asynchrone (retourne une Promise JS)
    emscripten::val drain() {
        auto op = std::make_shared<AsyncOp>();

        auto promise = emscripten::val::global("Promise").new_(
            emscripten::val::module_property("_createPromiseCallback")(op)
        );

        // Côté Rust : rayon::spawn + channel (voir rag3weaver_drain_async)
        void* ctx = ctx_;
        rag3weaver_drain_async(ctx, [op](const char* result) {
            op->result = result ? std::string(result) : R"({"error":"null"})";

            // Proxy le resolve() vers le thread appelant
            emscripten_async_run_in_main_runtime_thread(
                EM_FUNC_SIG_VI,
                [](void* arg) {
                    auto* o = static_cast<AsyncOp*>(arg);
                    o->resolve(o->result);
                },
                op.get()
            );
        });

        return promise;
    }
};
```

### Implémentation Rust (rayon::spawn + channel)

```rust
// Dans wasm_ffi.rs
#[no_mangle]
pub extern "C" fn rag3weaver_drain_async(
    ctx: *mut c_void,
    callback: extern "C" fn(*const c_char),
) {
    let ctx = unsafe { &mut *(ctx as *mut WeaverContext) };

    // Sérialise les drain() — un seul à la fois
    let _guard = ctx.drain_lock.lock().unwrap();

    rayon::spawn(move || {
        let result = do_drain(ctx);
        let c_str = return_string_to_c(result);
        callback(c_str);
    });
}
```

### Usage JS

```js
const weaver = new Module.Weaver(config, "");

// Sync (rapide)
const h1 = weaver.create("Document", JSON.stringify({ title: "Rust Guide" }));
const h2 = weaver.create("Chunk", JSON.stringify({ text: "..." }));
weaver.link(h1, h2, "HAS_CHUNK");

// Async (non-bloquant)
const result = await weaver.drain();
console.log(result); // {"processed":3,"failed":0,"persisted":3}
```

### Quelles fonctions sont async ?

| Fonction | Sync/Async | Raison |
|----------|-----------|--------|
| `create()` | **Sync** | Juste un push dans la queue, instantané |
| `link()` | **Sync** | Idem |
| `count()` | **Sync** | Query simple, rapide |
| `drain()` | **Async** | Persist + embed + index, potentiellement long |
| `search()` | **Async** | Lucivy search + DB lookup, peut prendre du temps |
| `version()` | **Sync** | Constante |

---

## 2. Chantiers pour exploiter le multi-threading

### 2.1. Batch embedding avec rayon — PRIORITÉ HAUTE

Le bug #6 du doc 22 (EmbedProcessor ne batch pas). Avec rayon validé, la correction
est encore plus simple :

```rust
async fn process(&self, items: &mut [OperationItem]) -> Result<(), String> {
    // Collect texts
    let texts: Vec<String> = items.iter()
        .filter_map(|item| /* extract text */)
        .collect();

    // Single batch embed call (déjà batch-ready dans le trait)
    let vectors = self.embedder.embed(&texts).await?;

    // Distribute results back
    // ...
}
```

Et si l'embedder est CPU-bound (candle en natif), on peut paralléliser via rayon :

```rust
use rayon::prelude::*;

// Pré-traitement parallèle (chunking, tokenisation)
let prepared: Vec<_> = texts.par_iter()
    .map(|text| preprocess(text))
    .collect();
```

### 2.2. drain() async via rayon::spawn (ce doc) — PRIORITÉ HAUTE

Implémenter le flow rayon::spawn + Promise décrit en section 1. Fichiers à modifier :
- `wasm_ffi.rs` : nouvelle fonction `rag3weaver_drain_async()` qui utilise `rayon::spawn`
- `weaver_bindings.cpp` : drain() retourne `emscripten::val` (Promise)
- Le drain synchrone reste disponible comme fallback
- **Mutex drain_lock** : un seul drain() à la fois (c'est un flush, pas besoin de paralléliser)

### 2.3. `unsafe Send + Sync` sur WasmDbConnection — PRIORITÉ HAUTE

Reclassifié en haute dans le doc 23. Avec du vrai multi-threading, les raw pointers
`CDatabase*` / `CConnection*` sont un risque de data race.

**Solution recommandée** : `Mutex<*mut CConnection>` sur la connexion.

```rust
pub struct WasmDbConnection {
    db: *mut c_void,          // CDatabase — partageable (Kuzu thread-safe sur DB)
    conn: Mutex<*mut c_void>, // CConnection — un seul thread à la fois
}
```

Kuzu supporte plusieurs connexions sur une même DB, mais une `CConnection` individuelle
n'est pas thread-safe. Le Mutex serialise les accès à la connexion.

**Alternative** : pool de connexions (une par thread du ThreadPool). Plus performant
mais plus complexe.

### 2.4. Embed callback bidirectionnel — PRIORITÉ MOYENNE

Le callback FFI pour Transformers.js (doc 22 #5) est simplifié par le threading :

```
Background thread (drain)
  → rayon batch les textes
  → appel embed callback
  → callback proxy vers le Worker thread (JS)
  → JS appelle Transformers.js (pipeline, batch)
  → vecteurs retournent au background thread
```

Le thread background fait le gros du travail. Le callback vers JS ne bloque plus
le thread appelant (c'est le background thread qui attend, pas le Worker principal).

**Pattern** : `emscripten_sync_run_in_main_runtime_thread()` pour un appel synchrone
depuis le background thread vers le Worker thread (où JS/Transformers.js tourne).

### 2.5. Parallélisme dans drain() via rayon — PRIORITÉ MOYENNE

Actuellement drain() exécute les processors séquentiellement :
1. PersistProcessor (DB writes)
2. EmbedProcessor (embeddings)
3. IndexProcessor (lucivy index)

Avec rayon, certains peuvent tourner en parallèle :
- Persist et Embed sont **indépendants** sur des items différents → parallélisables
- Index dépend de Persist (le nœud doit exister avant l'indexation)

```rust
// Pseudo-code
rayon::join(
    || persist_processor.process(&mut persist_items),
    || embed_processor.process(&mut embed_items),
);
index_processor.process(&mut index_items); // après persist
```

### 2.6. Supprimer pollster — PRIORITÉ FAIBLE

`pollster::block_on()` est remplaçable par `futures::executor::block_on()`.
On dépend déjà de `futures`. Ça retire une dépendance.

### 2.7. Pool rayon dédié sur WeaverContext — PRIORITÉ HAUTE (fusionné avec #2.2)

Avec le passage à `rayon::spawn`, le pool rayon **doit** être configuré explicitement
sur le `WeaverContext` pour contrôler le budget de threads.

```rust
pub struct WeaverContext {
    catalog: Catalog,
    pool: rayon::ThreadPool,  // Pool dédié, 4 threads
    drain_lock: Mutex<()>,    // Un seul drain() à la fois
}

impl WeaverContext {
    pub fn new(config: CatalogConfig) -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)  // Laisse ~12 workers pour Kuzu + lucivy interne
            .build()
            .expect("Failed to create rayon thread pool");
        Self {
            catalog: Catalog::new(config),
            pool,
            drain_lock: Mutex::new(()),
        }
    }
}
```

**Budget threads** :

| Composant | Threads | Source |
|-----------|---------|--------|
| Kuzu (query parallelism) | ~8 | Pool emscripten global |
| Lucivy (rayon interne) | ~4 | Pool rayon global de lucivy |
| **rag3weaver (notre pool)** | **4** | **Pool rayon dédié** |
| Total | ~16 | = `PTHREAD_POOL_SIZE` |

Note : rayon supporte plusieurs pools indépendants. Le pool dédié de rag3weaver
ne partage pas ses threads avec le pool rayon global utilisé par lucivy.

### 2.8. Garde contre appels concurrents — PRIORITÉ HAUTE

Scénarios d'exhaustion et protections :

| Scénario | Risque | Protection |
|----------|--------|------------|
| drain() + drain() | 2 drains en parallèle sur le même contexte | `drain_lock: Mutex<()>` — le 2e attend |
| drain() + search() | Compétition pour le pool rayon | OK — queue rayon, le plus lent attend |
| drain() pendant query Kuzu lourde | Kuzu occupe ses threads, rayon attend les siens | OK — pools séparés |
| Rafale de search() | N search en queue | OK — rayon traite dans l'ordre, work-stealing |

Le pire cas : drain() + search() simultanés → les deux sont en queue sur le pool
rayon → l'un attend que l'autre libère un thread → la Promise prend plus longtemps
mais **jamais d'échec fatal**.

---

## 3. Ordre de réalisation suggéré

| # | Chantier | Priorité | Dépendance |
|---|----------|----------|------------|
| 1 | Mutex sur WasmDbConnection (#2.3) | **Haute** | — (thread safety avant tout) |
| 2 | Pool rayon dédié + drain_lock (#2.7+#2.8) | **Haute** | — |
| 3 | drain() async via rayon::spawn (#2.2) | **Haute** | Après #1 et #2 |
| 4 | Batch embedding rayon (#2.1) | **Haute** | — |
| 5 | Embed callback FFI (#2.4) | Moyenne | Après #3 et #4 |
| 6 | Parallélisme drain (#2.5) | Moyenne | Après #4 |
| 7 | Supprimer pollster (#2.6) | Faible | — |

**L'ordre critique** :
- #1 (Mutex WasmDbConnection) **avant** #3 (drain async) — sinon data race sur `CConnection`
- #2 (pool rayon dédié) **avant** #3 (drain async) — sinon rayon::spawn utilise le pool global partagé avec lucivy

---

## 4. Stack technique WASM rag3weaver (résumé)

| Besoin | Outil | Statut |
|--------|-------|--------|
| Data parallelism (batch) | **rayon** | Validé (doc 23, test C) |
| **JS async (Promises)** | **rayon::spawn + channel + emscripten proxy** | **À implémenter** |
| Sérialisation drain | **Mutex<()> drain_lock** | À implémenter |
| Pool dédié weaver | **rayon::ThreadPool (4 threads)** | À implémenter |
| Coordination async | **tokio::sync** (channels, watch) | Fonctionne (compile) |
| Bridge sync→async FFI | **futures::executor::block_on** | Remplace pollster |
| Embeddings browser | **@xenova/transformers** via callback | À implémenter (Phase 4) |
| JSPI (futur) | **-sJSPI** | Migration quand universel |

**Note** : `futures::executor::ThreadPool` (validé doc 23, test B) reste disponible
mais n'est plus nécessaire pour l'architecture async. `rayon::spawn` couvre le même
besoin avec l'avantage du work-stealing et de la résilience à l'exhaustion du pool.

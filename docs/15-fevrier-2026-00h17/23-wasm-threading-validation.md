# WASM Threading — Validation et findings (15 février 2026)

Date : 15 février 2026

---

## Contexte

Lors de la revue des concessions du doc 22, la question du single-threading en WASM
a été soulevée. Une recherche initiale (basée sur des rapports de bugs de 2021-2022)
suggérait que `std::thread::spawn` ne fonctionnait pas sur `wasm32-unknown-emscripten`.

**C'est faux pour notre build.** L'analyse de la configuration réelle, l'historique
du projet (extension lucivy nécessitant plus de threads que prévu), et les tests
de validation ci-dessous confirment que le multi-threading fonctionne pleinement.

---

## 1. Configuration multi-threading confirmée

### C++ (CMakeLists.txt racine, lignes 199-203)

```cmake
if(NOT __SINGLE_THREADED__)
    add_compile_options(-pthread)
    add_link_options(-pthread)
    add_link_options(-sPTHREAD_POOL_SIZE=16)
endif()
```

16 Web Workers pré-alloués. Kuzu (moteur de requêtes rag3db) les utilise pour le
parallélisme de ses requêtes.

### Rust — lucivy_fts (extension/lucivy_fts/CMakeLists.txt)

```cmake
EMCC_CFLAGS=-pthread -fexceptions -sDISABLE_EXCEPTION_CATCHING=0
RUSTFLAGS=-C target-feature=+atomics,+bulk-memory,+mutable-globals -C panic=abort
```

Avec `+nightly` et `-Z build-std=std,panic_abort`.

### Rust — rag3weaver (extension/rag3weaver/CMakeLists.txt)

Configuration identique à lucivy_fts.

### Headers navigateur (serve.js)

```js
res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
```

COOP/COEP requis pour SharedArrayBuffer.

### Récapitulatif

| Composant | Threading | Mécanisme |
|-----------|-----------|-----------|
| rag3db (Kuzu C++) | Multi-thread | emscripten pthreads, 16 workers |
| lucivy_fts (Rust) | Multi-thread | rayon + atomics réelles via `-Z build-std` |
| rag3weaver (Rust) | **Multi-thread (validé)** | `std::thread::spawn` + `futures::executor::ThreadPool` |
| Navigateur | SharedArrayBuffer | COOP/COEP headers |

---

## 2. Pourquoi la recherche initiale était fausse

Les rapports de bugs cités datent de 2021-2022 :

- **`singlethread: true` dans le target spec** : contourné par `-C target-feature=+atomics`
  qui force LLVM à générer des atomiques réelles, et `-Z build-std` qui recompile std
  avec ces features.
- **Bug TLS `R_WASM_MEMORY_ADDR_TLS_SLEB`** : résolu dans les versions récentes de
  wasm-ld (emscripten 5.0.1 utilise une version à jour).
- **`Instant::now()` cassé sur emscripten** : corrigé dans les nightly récents.

La preuve empirique : l'extension lucivy_fts utilise rayon pour le parallélisme, et
lors des tests WASM, il a fallu **augmenter** `PTHREAD_POOL_SIZE` parce que lucivy
créait plus de threads que le pool ne le permettait.

---

## 3. Tests de validation — 3 PASSÉS, 1 ÉCHEC (tokio)

Quatre fonctions FFI de test ajoutées dans `wasm_ffi.rs`, exposées via embind, et
exécutées dans un test Playwright (`threading.spec.js`).

### Test A — `std::thread::spawn` + atomics — PASSÉ

4 threads Rust, chacun incrémentant un compteur atomique 1000 fois.

```rust
#[no_mangle]
pub extern "C" fn rag3weaver_test_threads() -> *const c_char {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    let num_threads: u64 = 4;
    let iterations: u64 = 1000;
    let counter = Arc::new(AtomicU64::new(0));

    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let c = counter.clone();
            std::thread::spawn(move || {
                for _ in 0..iterations {
                    c.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();

    for h in handles { h.join().unwrap(); }

    let total = counter.load(Ordering::SeqCst);
    // ...
}
```

**Résultat** :
```json
{"ok": true, "total": 4000, "expected": 4000, "threads": 4, "joinErrors": 0}
```

**Conclusions** :
- `std::thread::spawn` fonctionne sur `wasm32-unknown-emscripten`
- Les atomiques sont réelles (pas de shims) : 4 threads × 1000 = exactement 4000
- `join()` fonctionne sans erreur
- Les threads utilisent le pool de Web Workers pré-alloués par emscripten

### Test B — `futures::executor::ThreadPool` — PASSÉ

Pool de 2 worker threads, 8 tâches async, chacune incrémentant un compteur atomique.

```rust
#[no_mangle]
pub extern "C" fn rag3weaver_test_async_pool() -> *const c_char {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    let pool = futures::executor::ThreadPool::builder()
        .pool_size(2)
        .create().unwrap();

    let num_tasks: u64 = 8;
    let counter = Arc::new(AtomicU64::new(0));

    let handles: Vec<_> = (0..num_tasks)
        .map(|_| {
            let c = counter.clone();
            futures::task::SpawnExt::spawn_with_handle(&pool, async move {
                c.fetch_add(1, Ordering::Relaxed);
            }).unwrap()
        })
        .collect();

    for h in handles { futures::executor::block_on(h); }

    let total = counter.load(Ordering::SeqCst);
    // ...
}
```

**Résultat** :
```json
{"ok": true, "total": 8, "expected": 8, "poolSize": 2}
```

**Conclusions** :
- `futures::executor::ThreadPool` fonctionne — runtime async multi-thread sans tokio/mio
- `spawn_with_handle` + `block_on` pour attendre les résultats : fonctionnel
- Pas de dépendance mio, pas de problème de compilation

### Test C — rayon `par_iter` — PASSÉ

Somme parallèle de 1..=1000 via `par_iter().sum()` + `par_chunks(250)`.

```rust
#[no_mangle]
pub extern "C" fn rag3weaver_test_rayon() -> *const c_char {
    use rayon::prelude::*;

    let data: Vec<u64> = (1..=1000).collect();
    let total: u64 = data.par_iter().sum();
    let expected: u64 = (1000 * 1001) / 2;

    let chunk_results: Vec<u64> = data
        .par_chunks(250)
        .map(|chunk| chunk.iter().sum::<u64>())
        .collect();
    // ...
}
```

**Résultat** :
```json
{"ok": true, "total": 500500, "expected": 500500, "numChunks": 4}
```

**Conclusions** :
- Rayon fonctionne dans rag3weaver (pas seulement dans lucivy)
- `par_iter()`, `par_chunks()`, work-stealing : tout opérationnel
- Idéal pour le batch embedding (paralléliser les appels embed)

### Test D — tokio `current_thread` + `enable_all()` — ÉCHEC (thread pool exhaustion)

tokio `Builder::new_current_thread().enable_all()` crée des threads internes
(timer driver, signal handler) qui consomment des workers du pool emscripten.
Avec 16 workers partagés entre Kuzu, lucivy (rayon), et les tests A/B/C, le
pool est épuisé et tokio deadlock.

```
Tried to spawn a new thread, but the thread pool is exhausted.
This might result in a deadlock unless some threads eventually exit or
the code explicitly breaks out to the event loop.
```

**Note** : tokio bloque aussi `rt-multi-thread` sur WASM via un `compile_error!`
explicite. C'est une décision délibérée de l'équipe tokio, pas un bug.

**Conséquence** : tokio n'est pas viable comme runtime async en WASM. On garde
`tokio::sync` (channels, watch) qui compile et fonctionne, mais le runtime
async doit être `futures::executor::ThreadPool` ou `pollster::block_on()`.

### Sortie Playwright complète

```
  ✓ Test A: std::thread::spawn + atomics (3.3s)
  ✓ Test B: futures::executor::ThreadPool (2.6s)
  ✓ Test C: rayon par_iter (2.6s)
  ✗ Test D: tokio current_thread — RETIRÉ (deadlock thread pool)
  3 passed (6.5s)
```

---

## 4. Implications pour le doc 22

### #7 (`unsafe impl Send + Sync` sur WasmDbConnection) — RECLASSIFIÉ EN HAUTE

Le WASM étant confirmé multi-threadé, les raw pointers `CDatabase*` / `CConnection*`
dans `WasmDbConnection` peuvent être accédés depuis plusieurs threads. Le
`unsafe impl Send + Sync` n'est pas une formalité — c'est un vrai risque de data race.

**Actions possibles** :
- Protéger les accès par un `Mutex<*mut CDatabase>` / `Mutex<*mut CConnection>`
- Ou créer une connexion par thread (Kuzu supporte plusieurs connexions sur une DB)
- Ou documenter et enforcer que `WasmDbConnection` n'est utilisé que sur un seul thread

### #5 (Embeddings callback FFI) — SIMPLIFIÉ

Le callback bidirectionnel peut être exécuté sur un thread séparé via `ThreadPool`.
`drain()` pourrait lancer l'embedding sur un worker thread, libérant le thread appelant.

### `pollster::block_on()` — REMPLAÇABLE

Avec `futures::executor::ThreadPool` validé, on peut remplacer `pollster::block_on()`
par un `ThreadPool` partagé sur le `WeaverContext`. Les opérations async (drain, search)
tourneraient sur le pool, et le thread FFI attendrait via `block_on(handle)`.

Plus ambitieux : exposer des API non-bloquantes au JS (drain retourne immédiatement,
callback quand c'est fini).

---

## 5. Impact sur l'architecture

| Capacité | Statut | Conséquence |
|----------|--------|-------------|
| `std::thread::spawn` | **Validé** | Parallélisme explicite disponible |
| `futures::executor::ThreadPool` | **Validé** | Runtime async multi-thread sans tokio |
| Rayon `par_iter` | **Validé** | Data parallelism, work-stealing, batch processing |
| Atomiques réelles | **Validé** | Arc, Mutex, AtomicU64 fonctionnent |
| Rayon (lucivy) | **Déjà actif** | Search/indexing parallèles en WASM |
| tokio `rt-multi-thread` | **Bloqué** | `compile_error!` explicite dans tokio sur WASM |
| tokio `current_thread` | **Bloqué** | `enable_all()` épuise le pool de threads emscripten |
| tokio `sync` | **Fonctionne** | Channels, watch, oneshot — compilent et fonctionnent |

### Architecture WASM recommandée pour rag3weaver

```
JS (main thread ou Worker)
  │
  ├── weaver.create() ─── FFI ──→ Rust (sync, rapide, pas de thread)
  ├── weaver.link()   ─── FFI ──→ Rust (sync, rapide)
  │
  └── weaver.drain()  ─── FFI ──→ Rust
                                    │
                                    ├── ThreadPool (2-4 workers)
                                    │     ├── PersistProcessor (DB writes)
                                    │     ├── EmbedProcessor (callback → JS → Transformers.js)
                                    │     └── IndexProcessor (lucivy, rayon interne)
                                    │
                                    └── block_on(drain_future) → retour JSON au JS
```

Le `ThreadPool` remplace `pollster::block_on()` pour les opérations internes du drain.
Le point d'entrée FFI reste synchrone (le JS attend le résultat), mais les processors
tournent en parallèle sur le pool.

---

## 6. Fichiers créés/modifiés

| Fichier | Action |
|---------|--------|
| `extension/rag3weaver/Cargo.toml` | Ajout `futures` (thread-pool), `rayon`, `tokio/rt` dans feature `wasm-emscripten` |
| `extension/rag3weaver/src/wasm_ffi.rs` | Ajout 4 fonctions de test (threads, async_pool, rayon, tokio_mt) |
| `tools/wasm/src_cpp/weaver_bindings.cpp` | Ajout 4 static methods embind correspondantes |
| `tools/wasm/test/browser/threading.html` | Nouveau — page HTML pour les tests |
| `tools/wasm/test/browser/threading_worker.js` | Nouveau — Web Worker exécutant les tests |
| `tools/wasm/test/browser/threading.spec.js` | Nouveau — 3 tests Playwright (A, B, C ; D retiré) |

### Note sur les fonctions de test

Les fonctions de test sont des validations d'infrastructure. Elles peuvent être
conservées (coût négligeable en taille binaire) ou retirées. Le test D (tokio)
reste dans le code Rust mais n'est pas appelé depuis le worker JS.

### Conclusion

Pour rag3weaver en WASM, l'architecture de parallélisme recommandée est :
- **Rayon** pour le data parallelism (batch embed, batch persist) — work-stealing
- **`futures::executor::ThreadPool`** pour les tâches async multi-thread
- **`tokio::sync`** pour la coordination (channels, watch) — sans runtime tokio
- **`pollster::block_on()`** aux points d'entrée FFI (bridge sync→async)

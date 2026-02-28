# 04 — Plan : Drain Parallelism (doc 26 Part B)

## Contexte

`flush()` (queue.rs:231) traite les groupes séquentiellement par priorité : insert (prio 1) → link (prio 2) → embed (prio 3). Chaque groupe est aussi traité batch par batch séquentiellement.

**Objectif** : `rayon::join(inserts, embeds)` en parallèle sur le pool dédié, puis links séquentiels.

**Pourquoi ça marche** : `EmbedProcessor` (catalog.rs:1052) appelle `entity_ref.ready().await` en Phase 1 qui bloque (via un `tokio::sync::watch` channel) jusqu'à ce que `InsertProcessor` (catalog.rs:940) résolve le ref avec `resolver.resolve(uuid)`. En parallèle via rayon, le thread embed attend naturellement que le thread insert ait fini la résolution UUID.

Les links restent séquentiels après (ils ont besoin des UUIDs résolus pour le MATCH Cypher).

## Fichiers à modifier

| Fichier | Changement |
|---------|-----------|
| `extension/rag3weaver/src/queue.rs` | `Box<dyn Processor>` → `Arc<dyn Processor>`, méthodes `pub`, `run_processor()` fn, 3 helpers |
| `extension/rag3weaver/src/catalog.rs` | Nouvelle méthode `drain_parallel()` feature-gated `wasm-emscripten` |
| `extension/rag3weaver/src/wasm_ffi.rs` | Utiliser `drain_parallel()` dans drain sync et async |

## Étapes détaillées

### B1. `processors` → `Arc<dyn Processor>` (`queue.rs`)

**Ligne 141** — changer le type du champ :
```rust
// Avant
processors: HashMap<&'static str, Box<dyn Processor>>,
// Après
processors: HashMap<&'static str, std::sync::Arc<dyn Processor>>,
```

**Ligne 177** — `register_processor()` convertit Box → Arc :
```rust
pub fn register_processor(&mut self, op_type: &'static str, processor: Box<dyn Processor>) {
    self.processors.insert(op_type, std::sync::Arc::from(processor));
}
```

L'API publique ne change pas (toujours `Box<dyn Processor>` en entrée).

**Ligne 313** — `flush()` utilise `self.processors.get(op_type)` qui retourne `Option<&Arc<dyn Processor>>`. `Arc<dyn Processor>` déréf auto en `&dyn Processor` donc `processor.process(batch).await` compile sans changement.

### B2. Rendre les méthodes `OperationItem` publiques + helpers (`queue.rs`)

**Lignes 46-67** — Les méthodes `mark_processing()`, `mark_completed()`, `mark_failed(error)`, `can_retry()` sont actuellement `fn` (privées au module). Il faut les rendre `pub fn` pour que `drain_parallel()` dans catalog.rs puisse les appeler.

```rust
pub fn mark_processing(&mut self) { ... }
pub fn mark_completed(&mut self) { ... }
pub fn mark_failed(&mut self, error: String) { ... }
pub fn can_retry(&self) -> bool { ... }
```

Note : `mark_persisted()` reste `fn` — pas besoin en WASM (pas de persistence).

**3 nouveaux helpers publics** sur `OperationQueue` :

```rust
/// Extraire tous les items pending, groupés par op type.
/// Les items sont retirés de self.items (std::mem::take).
pub fn take_pending_grouped(&mut self) -> Vec<(&'static str, Vec<OperationItem>)> {
    let all = std::mem::take(&mut self.items);
    let (to_process, keep): (Vec<_>, Vec<_>) = all.into_iter().partition(|item| {
        matches!(item.state, ItemState::Pending | ItemState::Persisted)
    });
    self.items = keep;

    // Group by operation type, same logic as flush()
    let mut by_type: Vec<(&'static str, Vec<OperationItem>)> = Vec::new();
    let mut type_index: HashMap<&'static str, usize> = HashMap::new();
    for item in to_process {
        let t = item.op.operation_type();
        if let Some(&idx) = type_index.get(t) {
            by_type[idx].1.push(item);
        } else {
            type_index.insert(t, by_type.len());
            by_type.push((t, vec![item]));
        }
    }
    by_type.sort_by_key(|(_, items)| items[0].op.priority());
    by_type
}

/// Remettre les items non-complétés dans la queue.
/// Les Completed sont filtrés (déjà traités).
pub fn return_items(&mut self, items: Vec<OperationItem>) {
    self.items.extend(
        items.into_iter().filter(|item| !matches!(item.state, ItemState::Completed))
    );
}

/// Obtenir un clone Arc d'un processor par nom.
pub fn get_processor(&self, name: &str) -> Option<std::sync::Arc<dyn Processor>> {
    self.processors.get(name).cloned()
}
```

**Nouveau `run_processor()` — fonction libre dans queue.rs** :

```rust
/// Process items with a processor synchronously (block_on).
/// Same logic as flush() batch processing but standalone.
/// Returns (processed, failed) counts.
pub fn run_processor(
    processor: Option<&dyn Processor>,
    items: &mut Vec<OperationItem>,
) -> FlushResult {
    let mut result = FlushResult::default();

    if items.is_empty() {
        return result;
    }

    let batch_size = items[0].op.config().batch_size;

    for batch in items.chunks_mut(batch_size) {
        for item in batch.iter_mut() {
            item.mark_processing();
        }

        if let Some(proc) = processor {
            match futures::executor::block_on(proc.process(batch)) {
                Ok(()) => {
                    for item in batch.iter_mut() {
                        item.mark_completed();
                        result.processed += 1;
                    }
                }
                Err(e) => {
                    for item in batch.iter_mut() {
                        if item.can_retry() {
                            item.retries += 1;
                            item.state = ItemState::Pending;
                            item.error = Some(e.clone());
                        } else {
                            item.mark_failed(e.clone());
                            result.failed += 1;
                        }
                    }
                }
            }
        } else {
            for item in batch.iter_mut() {
                item.mark_failed(format!("no processor registered"));
                result.failed += 1;
            }
        }
    }
    result
}
```

Note : `OperationItem.retries` est `pub` (l.41). `OperationItem.state` aussi (l.38). Donc `item.retries += 1; item.state = ItemState::Pending;` compile. Pour la fonction `run_processor()`, on a besoin que `mark_processing`, `mark_completed`, `mark_failed`, `can_retry` soient `pub` — fait en B2.

### B3. `Catalog::drain_parallel(pool)` (`catalog.rs`)

Feature-gated `#[cfg(feature = "wasm-emscripten")]` pour que ça n'affecte pas les tests natifs.

```rust
#[cfg(feature = "wasm-emscripten")]
pub fn drain_parallel(&mut self, pool: &rayon::ThreadPool) -> FlushResult {
    use crate::queue::run_processor;

    let mut groups = self.queue.take_pending_grouped();
    if groups.is_empty() {
        return FlushResult::default();
    }

    // Extract groups by name
    let mut inserts = Vec::new();
    let mut embeds = Vec::new();
    let mut links = Vec::new();
    // groups is Vec<(&str, Vec<OperationItem>)> sorted by priority
    for (op_type, items) in groups.drain(..) {
        match op_type {
            "insert" => inserts = items,
            "embed" => embeds = items,
            "link" => links = items,
            _ => {} // ignore unknown
        }
    }

    let insert_proc = self.queue.get_processor("insert");
    let embed_proc = self.queue.get_processor("embed");
    let link_proc = self.queue.get_processor("link");

    // Phase 1 : inserts + embeds en parallèle via rayon::join
    let (r_insert, r_embed) = pool.install(|| {
        rayon::join(
            || run_processor(insert_proc.as_deref(), &mut inserts),
            || run_processor(embed_proc.as_deref(), &mut embeds),
        )
    });

    // Phase 2 : links séquentiels (ont besoin des UUIDs résolus)
    let r_link = run_processor(link_proc.as_deref(), &mut links);

    // Remettre les items non-complétés
    let mut all = inserts;
    all.extend(embeds);
    all.extend(links);
    self.queue.return_items(all);

    FlushResult {
        processed: r_insert.processed + r_embed.processed + r_link.processed,
        failed: r_insert.failed + r_embed.failed + r_link.failed,
        persisted: 0,
    }
}
```

**Pourquoi `pool.install(|| rayon::join(...))`** : `pool.install` exécute la closure sur notre pool dédié (4 threads), pas le pool rayon global (partagé avec tantivy). `rayon::join` fork les deux tâches sur ce pool.

**Pas de persistence** : en WASM il n'y a pas de `OperationPersistence`, donc on skip l'étape persist que fait `flush()` (lignes 276-292 de queue.rs).

**Pas de reentrancy guard** : le `drain_lock` dans `wasm_ffi.rs` remplit ce rôle.

### B4. Utiliser dans `wasm_ffi.rs`

**`rag3weaver_drain()` (ligne 1044)** :
```rust
// Avant
let result = futures::executor::block_on(catalog.drain());
// Après
let result = catalog.drain_parallel(&ctx.pool);
```

**`rag3weaver_drain_async()` (ligne 1097)** :
```rust
// Avant
let result = futures::executor::block_on(cat.drain());
// Après
let result = cat.drain_parallel(/* pool */);
```

Pour le drain_async, le pool n'est pas directement disponible dans la closure (on a cloné catalog, drain_lock, refs mais pas pool). **Deux options** :

**Option A** — Cloner le pool via `Arc`. Changer `pool: rayon::ThreadPool` en `pool: std::sync::Arc<rayon::ThreadPool>` dans `WeaverContext`. Le pool rayon n'implémente pas Clone nativement, donc Arc est nécessaire pour le partager dans la closure spawn.

**Option B** — Passer `&ctx.pool` en référence avant le spawn, puis dans la closure utiliser le pool qui est déjà celui qui exécute la closure. Problème : `pool.install(|| rayon::join(...))` dans `drain_parallel` attend un `&rayon::ThreadPool`, mais la closure tourne déjà sur ce pool... On pourrait utiliser `rayon::join` directement sans `pool.install` vu qu'on est déjà dans le pool context.

**Choix : Option A** — `pool: Arc<rayon::ThreadPool>`. Plus simple, pas d'ambiguïté.

```rust
pub struct WeaverContext {
    catalog: std::sync::Arc<std::sync::Mutex<Catalog>>,
    drain_lock: std::sync::Arc<std::sync::Mutex<()>>,
    pool: std::sync::Arc<rayon::ThreadPool>,  // ← Arc
    refs: std::sync::Arc<std::sync::Mutex<Vec<crate::refs::EntityRef>>>,
}
```

Et dans `rag3weaver_drain_async` :
```rust
let pool = ctx.pool.clone();  // Arc::clone, cheap

ctx.pool.spawn(move || {
    // ...lock drain_lock, catalog...
    let result = cat.drain_parallel(&pool);
    // ...build JSON, callback...
});
```

Dans `rag3weaver_drain` sync :
```rust
let result = catalog.drain_parallel(&ctx.pool);
```

Le drain séquentiel (`catalog.drain()`) reste disponible pour les tests natifs (tokio runtime).

## Impact sur les tests existants

- **271 tests Rust** : `flush()` et `drain()` async restent inchangés. Les nouveaux helpers + `run_processor()` n'affectent pas le code existant. `Box<dyn Processor>` → `Arc<dyn Processor>` est transparent (register_processor convertit).
- **Tests Playwright** : même API JS, mêmes résultats. Le drain est juste plus parallélisé en interne.

## Vérification

```bash
# 1. Tests unitaires Rust (271 tests — pas de régression)
cd extension/rag3weaver && cargo test --lib

# 2. Build WASM
cd extension/rag3weaver && ./build_wasm.sh --clean
cd ../../build_wasm && source ~/emsdk/emsdk_env.sh
emmake cmake --build . --target rag3db_wasm -j$(nproc)

# 3. Tests Playwright (6 tests — même résultats qu'avant)
cd tools/wasm && npx playwright test
```

## Résumé des changements par fichier

### queue.rs
- Ligne 141 : `Box<dyn Processor>` → `Arc<dyn Processor>`
- Ligne 177 : `register_processor` — `Arc::from(processor)` au lieu de direct insert
- Lignes 46-67 : `mark_processing`, `mark_completed`, `mark_failed`, `can_retry` → `pub fn`
- Nouveau : `take_pending_grouped()`, `return_items()`, `get_processor()` (3 helpers)
- Nouveau : `run_processor()` (fonction libre publique)

### catalog.rs
- Nouveau : `drain_parallel(&mut self, pool: &rayon::ThreadPool) -> FlushResult` — feature-gated `wasm-emscripten`

### wasm_ffi.rs
- `WeaverContext.pool` : `rayon::ThreadPool` → `Arc<rayon::ThreadPool>`
- `rag3weaver_catalog_new` : wrap pool dans `Arc::new()`
- `rag3weaver_drain` : `block_on(catalog.drain())` → `catalog.drain_parallel(&ctx.pool)`
- `rag3weaver_drain_async` : idem + clone `ctx.pool` dans la closure

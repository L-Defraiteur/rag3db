# Doc 21 — Session : Pure Dataflow Ingestion — Phase 1

Date : 6 mars 2026

## Objectif

Implémenter la Phase 1 du doc 20 (Fondations) : préparer le framework dataflow pour recevoir des données d'ingestion sur les ports, sans casser l'existant.

## Travail effectué

### 1.1 — PortType + PortValue ingestion (`port.rs`)

8 nouveaux variants `PortType` pour l'ingestion :

```
Ops, Inserts, Links, Chunks, Aggregates, Embeds, SparseEmbeds, DualEmbeds
```

**Problème rencontré** : les types d'ops (`InsertOp`, `LinkOp`) contiennent des `EntityRefResolver` / `RelationRefResolver` (oneshot channels) — ni `Clone` ni `Serialize`. Impossible de les mettre directement dans `PortValue` (qui dérive les deux).

**Solution** : `BatchPayload` — wrapper type-erased :
- `Arc<Mutex<Option<Box<dyn Any + Send>>>>` pour le stockage
- `Clone` via `Arc` (cheap, pas de deep copy)
- `Serialize` en summary `{ batch_type, count }` (suffisant pour l'observabilité)
- `take::<T>()` pour extraire les données (consomme, retourne `None` si déjà pris)
- Sur type mismatch, les données sont **préservées** (remises dans le Mutex)

```rust
// Création
let payload = BatchPayload::new(PortType::Inserts, insert_ops);
// Transport
let pv = PortValue::Batch(payload);
// Extraction dans un nœud
let ops: Vec<InsertOp> = ctx.take_input("ops")
    .and_then(|pv| match pv { PortValue::Batch(p) => p.take::<InsertOp>(), _ => None })
    .ok_or("missing inserts")?;
```

+5 tests unitaires (compatibilité types, take/clone, serialize, merge error).

### 1.2 — `execute(&mut self)` (`node.rs`)

Changement de signature des traits :

```rust
// Avant
async fn execute(&self, ctx: &mut NodeContext) -> Result<(), String>;
async fn execute_dynamic(&self, ctx: &mut NodeContext, emitter: &mut GraphEmitter) -> Result<(), String>;

// Après
async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String>;
async fn execute_dynamic(&mut self, ctx: &mut NodeContext, emitter: &mut GraphEmitter) -> Result<(), String>;
```

**Impact** : mise à jour de toutes les implémentations :
- 4 search nodes (`search_nodes.rs`)
- 5 ingestion nodes + 2 DynamicNodes (`ingestion_nodes.rs`)
- 5 test nodes (`graph.rs`, `runtime.rs`)
- Tests : `let node` → `let mut node`

**Résultat** : suppression des 5 blocs `unsafe` dans `ingestion_nodes.rs` :

```rust
// Avant (5 occurrences)
let items = unsafe {
    &mut *(std::ptr::addr_of!(self.items) as *mut Vec<InsertOp>)
};

// Après
let items = &mut self.items;
```

### 1.3 — Graph `&mut` access (`graph.rs` + `runtime.rs`)

Dans `runtime.rs`, changement de `&graph.nodes[idx]` → `&mut graph.nodes[idx]` pour permettre les appels `&mut self` sur les nœuds. Le runtime exécute séquentiellement par nœud, donc pas besoin de Mutex.

### 1.4 + 1.5 — ServiceRegistry string keys + NodeContext (`services.rs`, `node.rs`, `runtime.rs`)

`ServiceRegistry` migré de `TypeId` keys → string keys :

```rust
// Avant
registry.register(Arc::new(my_db));        // TypeId::of::<MyDb>()
registry.get::<MyDb>()                      // ne marche pas avec dyn Trait

// Après
registry.register("conn", Arc::new(my_db));
registry.get::<MyDb>("conn")
```

Avantages : compatible JSON config, fonctionne avec trait objects (pas de `TypeId::of::<dyn Trait>()`).

`NodeContext` enrichi :

```rust
pub struct NodeContext {
    inputs: HashMap<String, PortValue>,
    outputs: HashMap<String, PortValue>,
    services: Arc<ServiceRegistry>,  // ← nouveau
}

impl NodeContext {
    pub fn service<T: Send + Sync + 'static>(&self, key: &str) -> Option<Arc<T>> { ... }
}
```

`DataflowRuntime` :

```rust
// Nouveau constructeur
DataflowRuntime::with_services(max_iterations, registry)
// Le runtime passe automatiquement les services à chaque NodeContext
```

+2 tests unitaires (wrong type, multiple services).

## Fichiers modifiés

| Fichier | Changement |
|---|---|
| `src/dataflow/port.rs` | +8 PortType variants, +BatchPayload, +PortValue::Batch, +5 tests |
| `src/dataflow/node.rs` | `execute(&mut self)`, `execute_dynamic(&mut self)`, +ServiceRegistry dans NodeContext |
| `src/dataflow/graph.rs` | TestNode `execute(&mut self)` |
| `src/dataflow/runtime.rs` | `&mut graph.nodes[idx]`, +services field, `with_services()`, `NodeContext::with_services()` |
| `src/dataflow/services.rs` | Réécrit : string keys, `has()`, `keys()`, +2 tests |
| `src/dataflow/search_nodes.rs` | 4× `execute(&mut self)`, 1× `execute_dynamic(&mut self)`, tests `let mut` |
| `src/dataflow/ingestion_nodes.rs` | 5× `execute(&mut self)`, 2× `execute_dynamic(&mut self)`, **5 unsafe supprimés** |
| `src/dataflow/report.rs` | `summarize_port_value` → handle `Batch` variant |
| `src/dataflow/mod.rs` | Export `BatchPayload` |

## Validation

- `cargo check --tests` : 0 erreur
- `cargo test --lib` : **382 pass**, 0 fail, 13 ignored
- 0 `unsafe` restant dans `ingestion_nodes.rs`

## Incident

`sed -i` a vidé `search_nodes.rs` (725 lignes → 0). Restauré via `git checkout --`. Leçon : utiliser l'outil Edit pour les remplacements, pas sed sur des fichiers Rust multi-lignes.

## Prochaine étape

**Phase 2** (doc 20) : Nœuds data-on-ports — les nœuds d'ingestion reçoivent leurs ops via `PortValue::Batch` au lieu des constructeurs. `SplitOpsNode` routeur. Réécriture de `build_ingestion_graph()`.

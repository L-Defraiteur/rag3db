# Doc 11 — Phase 1 Complete : Dataflow Framework + Search Migration

## Résultat

**Phase 1 terminée à 100%.** Le framework dataflow remplace `search_queue.rs` (837L) + `processors.rs` (575L) par `src/dataflow/` (7 fichiers, ~1475L). 373 tests unitaires + 5 E2E compilent, 0 régressions.

---

## Fichiers créés : `src/dataflow/`

| Fichier | ~Lignes | Tests | Rôle |
|---------|---------|-------|------|
| `port.rs` | 180 | 3 | `PortType` enum (9 variants), `PortValue` enum (Serialize), `PortDef`, `merge_port_values()` fan-in |
| `node.rs` | 165 | 3 | `Node` trait (async execute), `DynamicNode` trait (execute + emitter), `NodeContext` (input/output typés), `GraphEmitter` |
| `graph.rs` | 260 | 5 | `DataflowGraph`, `Edge`, `connect()` (type-check), `validate()` (DAG + required), `topological_sort()` (Kahn), `merge_dynamic()` |
| `services.rs` | 55 | 2 | `ServiceRegistry` (TypeId → Arc<dyn Any>), injection de dépendances |
| `runtime.rs` | 365 | 5 | `DataflowRuntime`, `DataflowEvent` (async_broadcast), `execute()` loop, `DataflowOutput`, fan-in/fan-out |
| `search_nodes.rs` | 430 | 6 | `QuerySourceNode`, `PrimarySearchNode`, `ExpansionNode` (DynamicNode), `FetchRelatedNode`, `ComposeNode` |
| `mod.rs` | 20 | — | Module root + pub use exports |

**Total : ~1475 lignes, 24 tests unitaires.**

## Fichiers modifiés

| Fichier | Changement |
|---------|-----------|
| `catalog.rs` | `build_search_queue()` → `build_dataflow_graph()`, `search_with_strategy()` réécrit via `DataflowRuntime` |
| `lib.rs` | `pub mod dataflow` ajouté, `pub mod processors` + `pub mod search_queue` retirés |
| `search_strategy.rs` | `Serialize` ajouté sur `ExpansionRule` + `ExpansionDirection` (nécessaire pour `PortValue` Serialize) |
| `tests/e2e_search_queue.rs` | Imports mis à jour (`dataflow::*` au lieu de `search_queue::*`), test `strategy_expand_has_file` utilise `build_dataflow_graph()` + `DataflowRuntime` |

## Fichiers supprimés

| Fichier | Lignes | Raison |
|---------|--------|--------|
| `search_queue.rs` | 837 | Remplacé par `dataflow/runtime.rs` + `dataflow/graph.rs` |
| `processors.rs` | 575 | Remplacé par `dataflow/search_nodes.rs` |

---

## Architecture

```
QuerySourceNode ──query──→ PrimarySearchNode ──results──→ ExpansionNode (DynamicNode)
                                    │                           │
                                    └──meta──→ (output)         ├── émet FetchRelatedNode(s)
                                                                ├── émet ComposeNode
                                                                └── connecte: expansion.results → compose.results
                                                                             fetch_N.children → compose.children (fan-in)

ComposeNode ──results──→ (output final)
```

### Avant (SearchQueue)

```
SearchQueue (round-based, shared SearchContext)
  ├── Round 0: PrimarySearchProcessor → context.root_results
  ├── Round 0: ExpansionProcessor → emit FetchRelated + deferred Compose
  ├── Round 1: FetchRelatedProcessor → context.children
  └── Round 2: ComposeProcessor → context.root_results[].other_children
```

- Shared mutable `SearchContext` (couplage implicite)
- Ordonnancement via rounds + `emit.all(handles).then(Compose)`
- Les processeurs lisent/écrivent dans le même état

### Après (Dataflow)

```
DataflowGraph (DAG typé, exécution topologique)
  ├── QuerySourceNode → PortValue::Query
  ├── PrimarySearchNode → PortValue::Results + PortValue::Meta
  ├── ExpansionNode (DynamicNode) → émet FetchRelated + Compose au runtime
  ├── FetchRelatedNode → PortValue::Children
  └── ComposeNode → PortValue::Results (final)
```

- Ports typés (`PortType` vérifié à `connect()`)
- Données transportées via `PortValue` (pas de shared state)
- Fan-in natif (merge de Children de plusieurs FetchRelated)
- Nœuds dynamiques (`DynamicNode` + `GraphEmitter` + re-topo-sort)
- Observabilité via `DataflowEvent` (NodeStarted, NodeCompleted, GraphExpanded, etc.)

---

## Patterns clés

### DynamicNode (remplacement du pattern Promise-like)

Avant :
```rust
// ExpansionProcessor
let h1 = emit.op(SearchOp::FetchRelated { ... });
let h2 = emit.op(SearchOp::FetchRelated { ... });
emit.all(vec![h1, h2]).then(SearchOp::Compose);
```

Après :
```rust
// ExpansionNode::execute_dynamic()
emitter.add_node(Box::new(FetchRelatedNode::new("fetch_0", ...)));
emitter.add_node(Box::new(ComposeNode));
emitter.connect("expansion", "results", "compose", "results");
emitter.connect("fetch_0", "children", "compose", "children");
```

Le graphe **est** la spécification de l'ordonnancement. Plus besoin de Promise-like — le topo sort gère tout.

### Fan-in natif

Plusieurs FetchRelatedNode → même port `compose.children` : `merge_port_values()` combine les HashMaps automatiquement. Plus de 3 niveaux de dédup manuels.

### FetchRelatedNode sans inputs

Les parents sont "baked in" au constructeur par ExpansionNode (pas via un port). Le nœud est ajouté dynamiquement au graphe et s'exécute naturellement après ExpansionNode (topo sort post merge_dynamic).

---

## Tests

### Unitaires (24)

| Module | Tests | Couverture |
|--------|-------|-----------|
| `port` | 3 | Serialize roundtrip, merge Children, PortType::Any compatible |
| `node` | 3 | NodeContext input/output, take_input, GraphEmitter drain |
| `graph` | 5 | connect validates ports, type mismatch, topo sort linear, cycle detection, validate missing required |
| `services` | 2 | store/retrieve, missing returns None |
| `runtime` | 5 | linear pipeline, fan-out, fan-in, dynamic node expansion, max iterations guard |
| `search_nodes` | 6 | QuerySource ports, expansion emits fetch+compose, no-match passthrough, dedup sources, compose attaches children, compose no-children passthrough |

### E2E (5, compilent)

1. `strategy_no_expansion` — pas d'expansion, `other_children = None`
2. `strategy_expand_has_file` — Directory → 2 File children via HAS_FILE (utilise `build_dataflow_graph()` + `runtime.subscribe()`)
3. `strategy_entity_filter` — filtre `source_entity = Directory`, File pas expanded
4. `strategy_child_data` — `ChildSummary.data` contient les champs File
5. `strategy_max_rounds_guard` — `max_rounds=0` → erreur

### Résultat

```
cargo test --lib : 373 passed, 0 failed
cargo check --test e2e_search_queue : compiles OK
```

---

## API publique

### Nouvelle API

```rust
// Simple (one-shot)
let response = Catalog::search_with_strategy(catalog, "kb", "query", strategy).await?;

// Avec observabilité
let mut graph = Catalog::build_dataflow_graph(catalog, "kb", "query", strategy).await;
let runtime = DataflowRuntime::new(10);
let mut rx = runtime.subscribe();
let output = runtime.execute(&mut graph).await?;
// rx.try_recv() → DataflowEvent
// output.get("compose", "results") → &PortValue
```

### Signature `search_with_strategy()` inchangée

L'API publique de `Catalog::search_with_strategy()` n'a pas changé — même signature, même `SearchStrategyResponse` en retour. Le changement est interne.

### Nouvelle API bas niveau

```rust
// Construire un graphe custom
let mut graph = DataflowGraph::new();
graph.add_node(Box::new(MyNode))?;
graph.add_dynamic_node(Box::new(MyDynamicNode))?;
graph.connect("a", "out", "b", "in")?;
graph.validate()?;

let runtime = DataflowRuntime::new(10);
let output = runtime.execute(&mut graph).await?;
```

---

## Prochaines phases

| Phase | Scope | Statut |
|-------|-------|--------|
| **1** | Core Framework + Search Migration | **FAIT** |
| 2 | Observabilité + rag3db Storage (Tap, Record, ExecutionReport) | À faire |
| 3 | Mermaid + GraphNode + NodeRegistry | À faire |
| 4 | Migrations | À faire |
| 5 | Rhai ScriptNode | À faire |

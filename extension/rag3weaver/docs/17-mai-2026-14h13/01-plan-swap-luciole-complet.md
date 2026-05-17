# Doc 01 — Plan : swap complet dataflow rag3weaver → luciole

Date : 17 mai 2026

## Etat des lieux

### Ce qui est fait
- Phase 0 : **DbConnection sync** ✅ (tout le stack est sync, 591 tests passent)
- luciole 0.1.0 + lucivy-core 2.0.0 comme deps crates.io ✅
- Plus de dépendance sur le submodule local ld-lucivy ✅

### Ce qui reste
- Phase 2 : ServiceRegistry
- Phase 3 : PortValue
- Phase 4 : 14 nodes → luciole::Node
- Phase 5 : Runtime swap (DataflowGraph → Dag, DataflowRuntime → execute_dag)
- Phase 6 : Nettoyage

---

## Points a considerer

### 1. GraphNode (sub-DAG as node) ✅

Luciole a `GraphNode` — un Dag wrappé comme un Node. Builder pattern :
```rust
let graph = GraphNode::builder(inner_dag)
    .input("value", "inner_node", "in", PortType::of::<T>())
    .output("result", "inner_node", "out", PortType::of::<T>())
    .build();
outer_dag.add_node("compute", graph);
```

**Impact rag3weaver** : Notre `build_dataflow_graph()` dans catalog.rs construit un DAG plat
avec ~14 nodes. Certaines séquences sont des sous-pipelines logiques :
- Pipeline chunking : `chunk → chunk_insert → chunk_link`
- Pipeline embed : `embed → sparse_commit`
- Pipeline KB : `kb_gather → kb_update → kb_chunk → kb_embed`

On pourrait wrapper ces séquences en `GraphNode` pour :
- Clarifier la structure
- Permettre la réutilisation (le pipeline KB est le même pour tous les KB)
- Tester les sous-pipelines en isolation

**Recommandation** : migrer d'abord en DAG plat (identique à aujourd'hui), puis refactorer
en GraphNode dans un second temps. Pas critique pour la migration initiale.

### 2. ServiceRegistry — Arc<T> vs &T

| | rag3weaver actuel | luciole |
|---|---|---|
| `register` | `register(key, Arc<T>)` | `register(key, T)` |
| `get` | `→ Option<Arc<T>>` | `→ Option<&T>` |
| Storage | `Arc<dyn Any>` | `Box<dyn Any>` |

91 appels `ctx.service::<T>(key)` dans 4 fichiers.

**Options** :
A. Utiliser le ServiceRegistry de luciole tel quel → `get::<Arc<dyn DbConnection>>()` retourne
   `&Arc<dyn DbConnection>`, on `.clone()` pour avoir un owned Arc. +91 `.clone()`.
B. Garder notre ServiceRegistry et le passer à luciole via un adapter.
C. Modifier notre `get()` pour retourner `&T` au lieu de `Arc<T>`.

**Recommandation** : Option A. Les `.clone()` sur Arc sont O(1) (atomic increment).
Et en fait beaucoup de nos nodes font déjà `let conn = ctx.service(...)?.clone()` implicitement
parce que le `?` move le `Arc`. Le changement est surtout syntaxique.

**Pattern concret** :
```rust
// Avant (rag3weaver ServiceRegistry)
let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
    .ok_or("conn not registered")?;
// conn: Arc<dyn DbConnection>

// Après (luciole ServiceRegistry)
let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
    .ok_or("conn not registered")?
    .clone();
// conn: Arc<dyn DbConnection>
```

### 3. PortValue — enum vs Any-based

**Aujourd'hui** : PortValue est un enum fermé avec ~12 variants + `BatchPayload` (Any-based).
Le paradoxe : `BatchPayload` est déjà `Box<dyn Any>` sous le capot. On a donc un enum
qui wrappe du Any. C'est redondant.

**Luciole** : `PortValue::new(T)` + `downcast::<T>()`. Extensible, pas besoin de modifier
l'enum pour un nouveau type.

**Migration** :
- Ingestion nodes utilisent `BatchPayload` → trivial, c'est déjà Any
- Search nodes utilisent `PortValue::Results(vec)` → `PortValue::new(vec)`
- `PortValue::Empty` → `PortValue::Trigger`

**Types à transporter** (inventaire complet) :
| PortType actuel | Type Rust | Usage |
|---|---|---|
| Entities | `Vec<EntityRecord>` via BatchPayload | Ingestion |
| Relations | `Vec<RelationRecord>` via BatchPayload | Ingestion |
| KBContent | `Vec<KBContentRecord>` via BatchPayload | KB pipeline |
| Updates | `Vec<UpdateRecord>` via BatchPayload | Drain |
| Deletes | `Vec<DeleteRecord>` via BatchPayload | Drain |
| Results | `Vec<UnifiedResult>` | Search |
| Children | `HashMap<String, Vec<ChildSummary>>` | Search |
| Meta | `SearchMeta` | Search |
| Query | `(String, String, SearchOptions, Option<SearchTarget>)` | Search |
| Rules | `Vec<ExpansionRule>` | Search |
| Map | `serde_json::Value` | Generic |
| Empty | unit / trigger | Control flow |

Tous ces types deviennent `PortValue::new(value)` + `PortType::of::<T>()` pour la validation.

**BatchPayload.take()** → remplacé par `ctx.take_input("port")?.take::<Vec<EntityRecord>>()`
(`PortValue::take<T>()` dans luciole consume la valeur, equivalent exact).

### 4. Node trait — diff de signature

| | rag3weaver Node | luciole Node |
|---|---|---|
| `execute` | `fn execute(&mut self, ctx: &mut NodeContext) → Result<(), String>` | identique ✅ |
| `inputs` | `fn inputs(&self) → &[PortDef]` (slice ref) | `fn inputs(&self) → Vec<PortDef>` (owned) |
| `outputs` | `fn outputs(&self) → &[PortDef]` (slice ref) | `fn outputs(&self) → Vec<PortDef>` (owned) |
| `name` | `fn name(&self) → &str` (instance name) | pas dans le trait (le Dag assigne le nom) |
| `node_type` | `fn node_type() → &'static str` | `fn node_type() → &'static str` ✅ |
| `node_config` | `→ serde_json::Value` | `→ Option<Box<dyn Any + Send>>` |
| `can_undo` | `fn can_undo() → bool` | identique ✅ |
| `undo_context` | `→ Option<serde_json::Value>` | `→ Option<Box<dyn Any + Send>>` |
| `undo` | `fn undo(&mut self, ctx, serde_json::Value)` | `fn undo(&mut self, Box<dyn Any + Send>)` |
| bound | `Send + Sync` | `Send` |

**Points d'attention** :
- `inputs/outputs` : nos nodes stockent des `Vec<PortDef>` en field et retournent `&[PortDef]`.
  Avec luciole on retourne `Vec<PortDef>` directement. Soit on clone, soit on construit a chaque appel.
  En pratique c'est appele une seule fois (a la construction du DAG), donc pas de perf issue.
- `name` disparait du trait : le nom est assigné par `dag.add_node("insert", node)`. Nos nodes
  stockent un `name: String` field — on le garde pour le logging/undo mais on ne l'expose plus
  via le trait.
- `undo` : on boxe le JSON dans `Box<dyn Any>`. Au checkpoint, on downcast back.
- `Send + Sync` → `Send` : luciole est moins restrictif. Pas de changement nécessaire.

### 5. DAG construction — flat API change

**Aujourd'hui** :
```rust
let mut graph = DataflowGraph::new();
graph.add_node(Box::new(InsertRecordNode::new("insert"))).unwrap();
graph.add_node(Box::new(ChunkRecordNode::new("chunk"))).unwrap();
graph.connect("insert", "inserted", "chunk", "entities").unwrap();
```

**Luciole** :
```rust
let mut dag = Dag::new();
dag.add_node("insert", InsertRecordNode::new());
dag.add_node("chunk", ChunkRecordNode::new());
dag.connect("insert", "inserted", "chunk", "entities").unwrap();
```

Diff mineure : le nom est passé a `add_node()` au lieu du constructeur du node. Les nodes
n'ont plus besoin de stocker leur nom.

### 6. Runtime — execution parallele

**Aujourd'hui** : `DataflowRuntime::run()` — séquentiel, topo sort, un node a la fois.

**Luciole** : `execute_dag()` — parallèle par niveau. Les nodes au même niveau du DAG
s'exécutent en parallèle sur le thread pool.

**Impact** : dans notre pipeline d'ingestion, la plupart des nodes sont en série
(insert → chunk → embed → flush). Pas de gain de parallélisme immédiat.

**Gain futur** : search DAG avec `fan_out_merge` :
```
VectorSearchNode ─┐
BM25SearchNode ───┤→ FuseNode → EnrichNode
SparseSearchNode ─┘
```
Les 3 search nodes s'exécutent en parallèle automatiquement.

### 7. Checkpoint — adapter l'interface

**Aujourd'hui** : `CypherCheckpointStore` implémente notre trait `CheckpointStore`.
On vient de le rendre sync.

**Luciole** : a son propre `CheckpointStore` trait avec des méthodes similaires mais pas
identiques. Il faudra soit :
A. Adapter `CypherCheckpointStore` pour implémenter le trait luciole
B. Wrapper via un adapter

Le trait luciole (`checkpoint.rs`) :
- `save(key, data: &[u8])` / `load(key) → Option<Vec<u8>>` / `delete(key)` / `list() → Vec<String>`
- Plus simple que le nôtre (qui a `create_execution`, `save_node_completed`, etc.)

**Solution** : notre `CypherCheckpointStore` garde sa logique métier (exécutions, nodes, statuts)
et délègue le stockage brut au trait luciole. Ou on garde notre impl checkpoint au-dessus de
luciole, en passant `None` pour le checkpoint a `execute_dag(dag, None)` pendant la transition.

**Recommandation** : Phase 1 = `execute_dag(dag, None)` sans checkpoint. Phase 2 = adapter.

### 8. EventBus — DataflowEvent vs DagEvent

**Aujourd'hui** : `async_broadcast` sender/receiver pour `DataflowEvent`.
**Luciole** : `subscribe_dag_events()` retourne un receiver de `DagEvent`.

Les events sont similaires :
| rag3weaver | luciole |
|---|---|
| `NodeStarted { node, inputs }` | `DagEvent::NodeStarted { name }` |
| `NodeCompleted { node, duration, outputs, metrics }` | `DagEvent::NodeCompleted { name, duration, metrics }` |
| `NodeFailed { node, error }` | `DagEvent::NodeFailed { name, error }` |
| `NodeLog { node, level, text }` | `DagEvent::NodeLog { name, level, text }` |
| `Completed { total, duration }` | retourné dans `DagResult` |
| `CheckpointResumed` | pas d'equivalent (pas besoin si pas de checkpoint) |

**Migration** : adapter les subscribers (progress bars, logging) aux nouveaux events.
Pas de changement de logique.

### 9. PollNode — pour les nodes longs

Luciole a `PollNode` — un node qui s'exécute par étapes (cooperative yielding).
Utile pour les nodes d'embedding qui traitent des gros batches.

**Aujourd'hui** : notre `EmbedNode` fait tout en un bloc. Avec `PollNode`, on pourrait
yielder après chaque batch de 32 textes, permettant aux autres nodes/acteurs de progresser.

**Recommandation** : garder les nodes comme `Node` standard pour la migration initiale.
Refactorer en `PollNode` plus tard si nécessaire (gros batches, WASM).

### 10. StreamDag — ingestion streaming

Pour l'ingestion incrémentale (watch mode, webhooks), `StreamDag` permettrait de piper
les documents directement dans le pipeline sans reconstruire le DAG a chaque fois.

**Aujourd'hui** : on reconstruit le DAG a chaque `drain()`. Avec StreamDag, on crée le
pipeline une fois et on feed les items en continu.

**Recommandation** : Phase future (après la migration de base).

### 11. BranchNode / GateNode — pipelines conditionnels

Utile pour nos pipelines qui ont des branches conditionnelles :
- `has_sparse` ? → route vers SparseCommitNode
- `has_kb` ? → route vers KBGatherNode
- `is_chunked` ? → route vers ChunkRecordNode ou skip

**Aujourd'hui** : la logique conditionnelle est dans `build_dataflow_graph()` — on n'ajoute
les nodes conditionnels que si la condition est vraie. Ca marche mais c'est statique.

Avec `BranchNode(|| has_sparse)`, le DAG peut être construit une fois et la condition
évaluée dynamiquement a l'exécution.

**Recommandation** : garder la construction statique pour la migration initiale.
Refactorer vers BranchNode plus tard pour les cas dynamiques.

### 12. fan_out_merge — search parallele

Le killer feature pour notre search :
```rust
dag.fan_out_merge(
    "search",
    vec![
        ("vector", VectorSearchNode::new()),
        ("bm25", BM25SearchNode::new()),
        ("sparse", SparseSearchNode::new()),
    ],
    "fuse",
    FuseNode::new(),
);
```

Les 3 search s'exécutent en parallèle, les résultats sont fusionnés automatiquement.

**Recommandation** : Phase post-migration. C'est le Search DAG (Phase C du plan multi-backend).

---

## Plan d'execution

### Etape 1 : Adapter les types (non-breaking)

Créer des type aliases et helpers dans `dataflow/compat.rs` :
```rust
pub use luciole::Node as LucioleNode;
pub use luciole::NodeContext as LucioleNodeContext;
pub use luciole::{Dag, PortDef, PortType, PortValue, ServiceRegistry};
```

### Etape 2 : Migrer les 14 nodes (un par un)

Pour chaque node :
1. `impl luciole::Node` au lieu de `impl crate::dataflow::Node`
2. `inputs/outputs` : retourner `Vec<PortDef>` avec `PortType::of::<T>()`
3. `execute` : `ctx.take_input().take::<T>()` au lieu de `BatchPayload::take()`
4. `undo` : boxer le JSON dans `Box<dyn Any>`
5. Tester en isolation

Ordre :
1. InsertRecordNode (source node, le plus simple)
2. LinkRecordNode
3. ChunkRecordNode
4. EmbedNode
5. FlushNode / SparseCommitNode
6. DeleteRecordNode / UpdateRecordNode / RechunkDeleteNode
7. KBGatherNode / KBUpdateNode / KBChunkNode / KBEmbedNode

### Etape 3 : Swap le DAG builder

Remplacer `DataflowGraph` par `luciole::Dag` dans `catalog.rs::build_dataflow_graph()`.
Même structure, API similaire :
```rust
// Avant
graph.add_node(Box::new(node)).unwrap();
graph.connect("a", "out", "b", "in").unwrap();

// Après
dag.add_node("a", node);
dag.connect("a", "out", "b", "in").unwrap();
```

### Etape 4 : Swap le runtime

Remplacer `DataflowRuntime::run()` par `luciole::execute_dag()` dans `catalog.rs::drain()`.
Sans checkpoint pour commencer (`execute_dag(&mut dag, None)`).

### Etape 5 : Nettoyage

Supprimer :
- `dataflow/graph.rs` — remplacé par luciole::Dag
- `dataflow/runtime.rs` — remplacé par luciole::execute_dag
- `dataflow/services.rs` — remplacé par luciole::ServiceRegistry
- `dataflow/port.rs` — PortType/PortValue remplacés par luciole, garder BatchPayload types

Garder :
- `dataflow/record_nodes.rs` — les nodes eux-mêmes
- `dataflow/node.rs` — uniquement si on garde un trait adapter
- `dataflow/checkpoint_store.rs` — persistence Cypher
- `dataflow/node_factories.rs` — reconstruction depuis checkpoint

### Etape 6 : Checkpoint adapter

Adapter `CypherCheckpointStore` pour fonctionner avec `execute_dag_with_checkpoint()`.

---

## Risques

| Risque | Impact | Mitigation |
|---|---|---|
| Parallélisme inattendu | Nos nodes partagent des `Arc<Mutex<_>>` — safe, mais l'ordre d'exécution change | Les nodes au même niveau sont indépendants par construction |
| PortValue downcast runtime | Mauvais type = `None` au lieu d'erreur compile | Les tests E2E couvrent tous les chemins |
| ServiceRegistry &T vs Arc<T> | 91 call sites | Ajout mécanique de `.clone()` |
| Checkpoint compat | Notre format est riche (exécution, statut, undo) vs luciole (simple k/v) | Phase séparée, `None` pendant la transition |
| async_broadcast EventBus | On l'utilise pour les progress bars | Adapter vers `subscribe_dag_events()` |

## Verification

A chaque étape :
```bash
cargo test -p rag3weaver --lib            # 591 tests
./run_e2e.sh --summary                    # E2E complet
```

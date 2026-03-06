# Doc 10 — Phases d'implémentation Dataflow Graph

## Principe

Chaque phase est **terminée à 100%** — on n'y revient pas. Les phases mêlent parfois plusieurs concepts si ils sont couplés (ex: le framework core n'est pas testable sans les search nodes, donc ils sont ensemble).

Le code actuel remplacé :
- `search_queue.rs` — 837 lignes (SearchQueue, Emitter, OpHandle, SearchProcessor, SearchQueueEvent)
- `processors.rs` — 575 lignes (PrimarySearch, Expansion, FetchRelated, Compose)

Le code préservé :
- `search_strategy.rs` — types (UnifiedResult, ChildSummary, SearchStrategy, ExpansionRule) — ne change pas
- `catalog.rs` — `search_with_strategy()` réécrit mais même signature publique
- `tests/e2e_search_queue.rs` — 5 tests, imports mis à jour

---

## Phase 1 : Core Framework + Search Migration

**Objectif** : Remplacer SearchQueue + processors par `src/dataflow/`. Les 367 tests unitaires (reécrits) + 5 E2E passent.

**Pas de nouvelles dépendances** — tout est dans Cargo.toml : async-broadcast, tokio, async-trait, serde.

### 1.1 — `dataflow/port.rs` : Types de transport

**Crée** : `src/dataflow/port.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    Results,    // Vec<UnifiedResult>
    Children,   // HashMap<String, Vec<ChildSummary>>
    Uuids,      // Vec<(String, String)>  — (source_uuid, result_uuid)
    Meta,       // SearchMeta
    Query,      // (kb_name, query, SearchOptions)
    Rules,      // Vec<ExpansionRule>
    Map,        // serde_json::Value
    Any,        // catch-all (Rhai, custom)
    Empty,      // trigger / unit
}

#[derive(Debug, Clone, Serialize)]
pub enum PortValue {
    Results(Vec<UnifiedResult>),
    Children(HashMap<String, Vec<ChildSummary>>),
    Uuids(Vec<(String, String)>),
    Meta(SearchMeta),
    Query { kb_name: String, query: String, options: SearchOptions },
    Rules(Vec<ExpansionRule>),
    Map(serde_json::Value),
    Any(serde_json::Value),
    Empty,
}

pub struct PortDef {
    pub name: &'static str,
    pub value_type: PortType,
    pub required: bool,
}
```

**Fan-in merge** (dans ce même fichier) :

```rust
pub fn merge_port_values(a: PortValue, b: PortValue) -> Result<PortValue, String> {
    // Children: HashMap merge (extend)
    // Results: concat
    // Uuids: concat
    // Empty + X = X
    // Sinon: erreur
}
```

**Compatibilité PortType** :

```rust
impl PortType {
    pub fn compatible_with(&self, other: &PortType) -> bool {
        self == other || *other == PortType::Any || *self == PortType::Any
    }
}
```

**Tests** (3) :
- `port_value_serialize_roundtrip` — Results via serde_json
- `merge_children_combines_hashmaps` — 2 Children → merged
- `port_type_any_compatible` — Any accepte tout

**Dépend de** : rien

---

### 1.2 — `dataflow/node.rs` : Traits et contexte d'exécution

**Crée** : `src/dataflow/node.rs`

```rust
#[async_trait]
pub trait Node: Send + Sync {
    fn name(&self) -> &str;
    fn inputs(&self) -> &[PortDef];
    fn outputs(&self) -> &[PortDef];
    async fn execute(&self, ctx: &mut NodeContext) -> Result<(), String>;
}
```

**DynamicNode** — pour les nœuds qui émettent de nouveaux nœuds au runtime :

```rust
#[async_trait]
pub trait DynamicNode: Send + Sync {
    fn name(&self) -> &str;
    fn inputs(&self) -> &[PortDef];
    fn outputs(&self) -> &[PortDef];
    async fn execute_dynamic(
        &self,
        ctx: &mut NodeContext,
        emitter: &mut GraphEmitter,
    ) -> Result<(), String>;
}
```

**NodeContext** — lit les inputs, écrit les outputs :

```rust
pub struct NodeContext {
    inputs: HashMap<String, PortValue>,
    outputs: HashMap<String, PortValue>,
}

impl NodeContext {
    pub fn input(&self, port: &str) -> Option<&PortValue>;
    pub fn take_input(&mut self, port: &str) -> Option<PortValue>;
    pub fn set_output(&mut self, port: &str, value: PortValue);
    pub(crate) fn set_input(&mut self, port: &str, value: PortValue);
    pub(crate) fn drain_outputs(&mut self) -> HashMap<String, PortValue>;
}
```

**GraphEmitter** — accumule les mutations du graphe émises par un DynamicNode :

```rust
pub struct GraphEmitter {
    added_nodes: Vec<Box<dyn Node>>,
    added_edges: Vec<Edge>,
}

impl GraphEmitter {
    pub fn add_node(&mut self, node: Box<dyn Node>);
    pub fn connect(
        &mut self,
        from_node: &str, from_port: &str,
        to_node: &str, to_port: &str,
    );
    pub(crate) fn drain(self) -> (Vec<Box<dyn Node>>, Vec<Edge>);
    pub fn is_empty(&self) -> bool;
}
```

**Tests** (3) :
- `node_context_input_output` — set_input, read input, set_output, drain
- `node_context_take_input` — take_input moves la valeur
- `graph_emitter_drain` — add_node + connect, drain, verify

**Dépend de** : 1.1

---

### 1.3 — `dataflow/graph.rs` : Structure du graphe

**Crée** : `src/dataflow/graph.rs`

```rust
#[derive(Debug, Clone)]
pub struct Edge {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
}

pub struct DataflowGraph {
    nodes: Vec<(String, Box<dyn Node>)>,       // ou DynNode enum (Node | DynamicNode)
    dynamic_nodes: Vec<(String, Box<dyn DynamicNode>)>,
    edges: Vec<Edge>,
}
```

**API** :

```rust
impl DataflowGraph {
    pub fn new() -> Self;
    pub fn add_node(&mut self, node: Box<dyn Node>) -> Result<(), String>;
    pub fn add_dynamic_node(&mut self, node: Box<dyn DynamicNode>) -> Result<(), String>;
    pub fn connect(
        &mut self,
        from_node: &str, from_port: &str,
        to_node: &str, to_port: &str,
    ) -> Result<(), String>;    // vérifie existence + PortType compatible
    pub fn validate(&self) -> Result<Vec<String>, String>;  // DAG + required inputs
    pub fn topological_sort(&self) -> Result<Vec<String>, String>;  // Kahn's algorithm
    pub(crate) fn merge_dynamic(
        &mut self,
        nodes: Vec<Box<dyn Node>>,
        edges: Vec<Edge>,
    ) -> Result<(), String>;
    pub fn node_names(&self) -> Vec<&str>;
}
```

**Design** :
- Nœuds stockés en `Vec` (pas HashMap) — le graphe est petit (4-15 nœuds), l'ordre d'insertion aide le topo sort
- `connect()` vérifie : les 2 nœuds existent, les ports existent sur chaque nœud, PortType compatible
- `validate()` : vérifie DAG (pas de cycles) + tous les ports `required: true` sont connectés
- `topological_sort()` : Kahn's algorithm — in-degree map, BFS. Retourne l'ordre d'exécution
- `merge_dynamic()` : intègre les nœuds/edges émis par un DynamicNode (après son exécution)

**Tests** (5) :
- `graph_connect_validates_ports` — port inexistant → erreur
- `graph_connect_type_mismatch` — Results → Children → erreur
- `graph_topological_sort_linear` — A→B→C = [A, B, C]
- `graph_cycle_detection` — A→B→A → erreur
- `graph_validate_missing_required` — required input non connecté → erreur

**Dépend de** : 1.2

---

### 1.4 — `dataflow/services.rs` : Injection de dépendances

**Crée** : `src/dataflow/services.rs`

```rust
pub struct ServiceRegistry {
    services: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl ServiceRegistry {
    pub fn new() -> Self;
    pub fn register<T: Send + Sync + 'static>(&mut self, service: Arc<T>);
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>>;
}
```

Utilisé pour passer `Arc<Mutex<Catalog>>` et `Arc<dyn DbConnection>` aux nœuds sans les coupler au graphe.

**Note Phase 1** : Les search nodes reçoivent leurs dépendances en constructeur (pas via le registry). Le registry est prêt mais pas encore utilisé — il sert à partir de la Phase 3 (Mermaid nodes instanciés depuis du JSON).

**Tests** (2) :
- `registry_store_retrieve` — register + get
- `registry_missing_returns_none`

**Dépend de** : rien (standalone)

---

### 1.5 — `dataflow/runtime.rs` : Moteur d'exécution

**Crée** : `src/dataflow/runtime.rs`

```rust
#[derive(Debug, Clone)]
pub enum DataflowEvent {
    NodeReady { node: String },
    NodeStarted { node: String },
    NodeCompleted { node: String, duration_ms: u64, outputs: Vec<String> },
    NodeFailed { node: String, error: String },
    GraphExpanded { by_node: String, added_nodes: Vec<String>, added_edges: usize },
    Completed { total_nodes: usize, duration_ms: u64 },
    Failed { error: String },
}

pub struct DataflowRuntime {
    max_rounds: usize,
    event_tx: Sender<DataflowEvent>,
    _inactive_rx: InactiveReceiver<DataflowEvent>,
}

pub struct DataflowOutput {
    outputs: HashMap<String, HashMap<String, PortValue>>,
}
```

**Algorithme `execute()`** :

```
fn execute(&self, graph: &mut DataflowGraph) -> Result<DataflowOutput>:

    graph.validate()?
    let order = graph.topological_sort()?

    let mut completed: HashSet<String>
    let mut port_data: HashMap<(node, port), PortValue>
    let mut round = 0

    loop:
        // Trouver les nœuds prêts (tous inputs required disponibles)
        let ready = order.iter()
            .filter(|n| !completed.contains(n))
            .filter(|n| all_required_inputs_available(n, &port_data, &graph.edges))
            .collect()

        if ready.is_empty():
            if completed.len() == total_nodes: break Ok(output)
            else: break Err("deadlock")

        for node_name in ready:
            // 1. Collecter les inputs via les edges
            let inputs = collect_inputs(node_name, &port_data, &graph.edges)
            // Fan-in: merge si plusieurs edges vers le même port
            let merged_inputs = merge_fan_in(inputs)

            // 2. Construire NodeContext
            let mut ctx = NodeContext::new()
            for (port, value) in merged_inputs:
                ctx.set_input(port, value)

            // 3. Exécuter
            if node is DynamicNode:
                let mut emitter = GraphEmitter::new()
                node.execute_dynamic(&mut ctx, &mut emitter).await?
                // Intégrer les nœuds émis
                if !emitter.is_empty():
                    let (new_nodes, new_edges) = emitter.drain()
                    graph.merge_dynamic(new_nodes, new_edges)?
                    // Re-sort pour inclure les nouveaux nœuds
                    order = graph.topological_sort()?
                    emit GraphExpanded event
            else:
                node.execute(&mut ctx).await?

            // 4. Stocker les outputs
            for (port, value) in ctx.drain_outputs():
                port_data.insert((node_name, port), value)

            completed.insert(node_name)
            emit NodeCompleted event

        round += 1
        if round > max_rounds: break Err("max_rounds exceeded")

    Ok(DataflowOutput { outputs: port_data reorganisé par nœud })
```

**Points critiques** :
- Le re-sort après merge_dynamic ne ré-exécute PAS les nœuds déjà complétés
- Fan-in : si 2 edges ciblent le même port, `merge_port_values()` les combine
- Fan-out : si 1 port a 2 edges sortantes, la valeur est clonée pour chaque destination
- Les nœuds sans inputs required (ou avec inputs optionnels non connectés) sont prêts immédiatement

**API** :

```rust
impl DataflowRuntime {
    pub fn new(max_rounds: usize) -> Self;
    pub fn subscribe(&self) -> Receiver<DataflowEvent>;
    pub async fn execute(&self, graph: &mut DataflowGraph) -> Result<DataflowOutput, String>;
}

impl DataflowOutput {
    pub fn get(&self, node: &str, port: &str) -> Option<&PortValue>;
}
```

**Tests** (5) :
- `runtime_linear_pipeline` — A→B→C, données propagées correctement
- `runtime_fanout` — A.out→B.in + A.out→C.in, les deux reçoivent les données
- `runtime_fanin` — A.out→C.in + B.out→C.in (même port), merge
- `runtime_dynamic_node` — DynamicNode ajoute un nœud, il s'exécute après
- `runtime_max_rounds` — expansion infinie → erreur

**Dépend de** : 1.1-1.4

---

### 1.6 — `dataflow/search_nodes.rs` : Nœuds de recherche

**Crée** : `src/dataflow/search_nodes.rs`

#### QuerySourceNode

Nœud trivial qui émet la query et les options comme PortValue :

```rust
pub struct QuerySourceNode {
    kb_name: String,
    query: String,
    options: SearchOptions,
}
// Inputs: aucun
// Outputs: "query" (PortType::Query)
```

#### PrimarySearchNode

```rust
pub struct PrimarySearchNode {
    catalog: Arc<tokio::sync::Mutex<Catalog>>,
}
// Inputs: "query" (PortType::Query)
// Outputs: "results" (PortType::Results), "meta" (PortType::Meta)
```

`execute()` : lock catalog, appelle `catalog.search()`, convertit `SearchResult` → `UnifiedResult` via `From`, écrit dans outputs.

Réutilise exactement la logique de l'actuel `PrimarySearchProcessor` (processors.rs lignes 30-76).

#### ExpansionNode (DynamicNode)

```rust
pub struct ExpansionNode {
    conn: Arc<dyn DbConnection>,
    rules: Vec<ExpansionRule>,
}
// Inputs: "results" (PortType::Results)
// Outputs: "results" (PortType::Results) — pass-through pour le cas sans match
```

`execute_dynamic()` :
1. Prend les results en input
2. Pour chaque rule, filtre par `source_entity` via `source_info()` (même logique que ExpansionProcessor)
3. Déduplique les parents par `source_uuid` (HashSet, même fix qu'avant)
4. Pour chaque rule avec parents matchants :
   - Crée un `FetchRelatedNode` avec parents baked-in
   - `emitter.add_node(fetch_node)`
5. Si au moins un FetchRelated émis :
   - Crée un `ComposeNode`
   - `emitter.add_node(compose_node)`
   - Pour chaque FetchRelated : `emitter.connect(fetch_name, "children", "compose", "children")`
   - Passe les results en output pour que l'edge `expansion.results → compose.results` fonctionne
6. Si aucun match : passe les results directement en output

#### FetchRelatedNode

```rust
pub struct FetchRelatedNode {
    name: String,
    conn: Arc<dyn DbConnection>,
    parents: Vec<(String, String)>,  // baked-in par ExpansionNode
    relation: String,
    direction: ExpansionDirection,
    limit: usize,
}
// Inputs: aucun required (le nœud a ses données en constructeur)
// Outputs: "children" (PortType::Children)
```

`execute()` : même Cypher UNWIND que l'actuel FetchRelatedProcessor (processors.rs lignes 155-265). Déduplique source_uuids (HashSet). Parse les rows → `ChildSummary`.

**Note sur l'ordonnancement** : FetchRelatedNode n'a pas d'input port connecté. Il est placé après ExpansionNode dans le topo sort car il est ajouté dynamiquement pendant l'exécution d'ExpansionNode — le runtime le place naturellement après. Si on veut un edge explicite pour le topo sort, on peut ajouter un port "trigger" (Empty) connecté depuis ExpansionNode.

#### ComposeNode

```rust
pub struct ComposeNode;
// Inputs: "results" (PortType::Results, required), "children" (PortType::Children, required=false)
// Outputs: "results" (PortType::Results)
```

`execute()` : même logique que ComposeProcessor (processors.rs lignes 283-309). Utilise `get().cloned()` pour la dédup (pas `remove()`).

**Tests** (6) :
- `primary_search_node_ports` — vérifie les PortDef
- `expansion_emits_fetch_and_compose` — mock results + rules → emitter a les bons nœuds/edges
- `expansion_no_match_passthrough` — pas de match → pas de nœuds émis, results pass-through
- `expansion_dedup_sources` — 3 index entries même source → 1 seul parent
- `compose_attaches_children` — results + children → other_children set
- `compose_no_children_passthrough` — results seuls → other_children reste None

**Dépend de** : 1.5

---

### 1.7 — Intégration catalog.rs + module root

**Crée** : `src/dataflow/mod.rs`

```rust
pub mod port;
pub mod node;
pub mod graph;
pub mod services;
pub mod runtime;
pub mod search_nodes;

pub use port::{PortDef, PortType, PortValue};
pub use node::{Node, DynamicNode, NodeContext, GraphEmitter};
pub use graph::{DataflowGraph, Edge};
pub use services::ServiceRegistry;
pub use runtime::{DataflowRuntime, DataflowEvent, DataflowOutput};
pub use search_nodes::*;
```

**Modifie** : `src/catalog.rs`

Remplace `build_search_queue()` (lignes 1645-1690) par :

```rust
pub async fn build_dataflow_graph(
    catalog: Arc<tokio::sync::Mutex<Catalog>>,
    kb_name: &str,
    query: &str,
    strategy: SearchStrategy,
) -> DataflowGraph {
    let conn = { catalog.lock().await.conn.clone() };
    let mut graph = DataflowGraph::new();

    // Source node
    graph.add_node(Box::new(QuerySourceNode::new(kb_name, query, &strategy.search))).unwrap();

    // Primary search
    graph.add_node(Box::new(PrimarySearchNode::new(catalog.clone()))).unwrap();
    graph.connect("query_source", "query", "primary_search", "query").unwrap();

    if !strategy.expansions.is_empty() {
        // Expansion (dynamic — émettra FetchRelated + Compose au runtime)
        graph.add_dynamic_node(Box::new(
            ExpansionNode::new(conn, strategy.expansions)
        )).unwrap();
        graph.connect("primary_search", "results", "expansion", "results").unwrap();
    }

    graph
}
```

Remplace `search_with_strategy()` (lignes 1692-1708) par :

```rust
pub async fn search_with_strategy(
    catalog: Arc<tokio::sync::Mutex<Catalog>>,
    kb_name: &str,
    query: &str,
    strategy: SearchStrategy,
) -> Result<SearchStrategyResponse, CatalogError> {
    let max_rounds = strategy.max_rounds;
    let has_expansions = !strategy.expansions.is_empty();
    let mut graph = Self::build_dataflow_graph(catalog, kb_name, query, strategy).await;

    let runtime = DataflowRuntime::new(max_rounds);
    let output = runtime.execute(&mut graph).await
        .map_err(|e| CatalogError::DbError(e))?;

    // Résultats du nœud terminal
    let results_node = if has_expansions { "compose" } else { "primary_search" };
    let results = output.get(results_node, "results")
        .and_then(|v| match v {
            PortValue::Results(r) => Some(r.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let meta = output.get("primary_search", "meta")
        .and_then(|v| match v {
            PortValue::Meta(m) => Some(m.clone()),
            _ => None,
        })
        .ok_or_else(|| CatalogError::DbError("no meta".into()))?;

    Ok(SearchStrategyResponse { results, meta })
}
```

**Dépend de** : 1.6

---

### 1.8 — Cleanup + E2E

**Supprime** :
- `src/search_queue.rs`
- `src/processors.rs`

**Modifie** : `src/lib.rs`
- Retire `pub mod search_queue;` et `pub mod processors;`
- Ajoute `pub mod dataflow;`
- Met à jour les re-exports

**Modifie** : `tests/e2e_search_queue.rs`

Les 4 tests qui utilisent `search_with_strategy()` ne changent que les imports. Le test `strategy_expand_has_file` qui utilise `build_search_queue()` + `subscribe()` :

```rust
// Avant :
let mut queue = Catalog::build_search_queue(...).await;
let mut rx = queue.subscribe();
queue.process().await.unwrap();
// lire les SearchQueueEvent

// Après :
let mut graph = Catalog::build_dataflow_graph(...).await;
let runtime = DataflowRuntime::new(10);
let mut rx = runtime.subscribe();
let output = runtime.execute(&mut graph).await.unwrap();
// lire les DataflowEvent
```

**Vérification** :
```bash
cargo test --lib   # tous les unit tests (anciens + nouveaux)
./run_e2e.sh --build --test e2e_search_queue   # 5 E2E
./run_e2e.sh --test e2e_search   # non-régression
./run_e2e.sh --test e2e_result_mode   # non-régression
```

**Dépend de** : 1.7

---

## Phase 2 : Observabilité + rag3db Storage

**Objectif** : Tap per-edge, recording dans rag3db, ExecutionReport pour le frontend.

### 2.1 — PortValue Serialize complet

Ajouter `Deserialize` sur les types qui en ont besoin. `SearchOptions` peut passer par `serde_json::Value` en interne si ses sous-types sont complexes.

### 2.2 — Tap system (`dataflow/observe.rs`)

```rust
impl DataflowRuntime {
    pub fn tap(&mut self, from_node: &str, from_port: &str,
               to_node: &str, to_port: &str) -> TapReceiver;
    pub fn tap_all(&mut self) -> TapReceiver;
}
```

Le Tap s'insère entre la propagation d'une valeur sur une edge : quand la runtime propage un output, si un tap est posé sur cette edge, elle clone la valeur et l'envoie dans le TapReceiver (channel).

Coût zéro si pas de tap posé.

### 2.3 — Record to rag3db (`dataflow/record.rs`)

Schema :
```
_DataflowExecution { _uuid, pipeline_name, status, duration_ms, created_at, variables }
  ├── [:HAS_NODE] → _DataflowNodeRun { node_name, node_type, status, duration_ms, inputs_summary, outputs_summary, inputs_json, outputs_json }
  └── [:HAS_EDGE] → _DataflowEdgeRun { from_node, from_port, to_node, to_port, value_summary, value_json }
```

Écriture en un seul batch Cypher **après** l'exécution complète.

```rust
pub enum RecordSink {
    Database(Arc<dyn DbConnection>),
    File(PathBuf),     // JSONL fallback
    Both(Arc<dyn DbConnection>, PathBuf),
    None,
}
```

### 2.4 — ExecutionReport (`dataflow/report.rs`)

```rust
#[derive(Serialize)]
pub struct ExecutionReport {
    pub nodes: Vec<NodeReport>,
    pub edges: Vec<EdgeReport>,
    pub duration_ms: u64,
}
```

Construit depuis les DataflowEvent après exécution. Le frontend peut l'afficher directement.

### 2.5 — Rétention

```rust
pub struct RecordRetention {
    pub max_per_pipeline: Option<usize>,
    pub max_age_days: Option<u32>,
    pub keep_errors: bool,
}
```

### Tests

- Record → query rag3db → vérifier les nœuds/edges
- Tap sur une edge → vérifier les données reçues
- JSONL export/import
- ExecutionReport structure

**Fichiers** : `dataflow/observe.rs`, `dataflow/record.rs`, `dataflow/report.rs`, modif `runtime.rs`

---

## Phase 3 : Mermaid + GraphNode + NodeRegistry

**Objectif** : Définir des pipelines en Mermaid, composer des sous-graphes, templates.

### 3.1 — NodeRegistry (`dataflow/node_registry.rs`)

```rust
pub struct NodeRegistry {
    factories: HashMap<String, Box<dyn NodeFactory>>,
}

pub trait NodeFactory: Send + Sync {
    fn create(&self, config: serde_json::Value, services: &ServiceRegistry) -> Result<Box<dyn Node>, String>;
    fn schema(&self) -> NodeSchema;
}
```

Enregistre PrimarySearch, FetchRelated, Compose, etc. comme factories.

### 3.2 — Parser Mermaid (`dataflow/parser.rs`)

Parse le sous-ensemble `graph LR/TD` :
- `NodeId["NodeType(param='value')"]` → nœud + config
- `-->|port_name|` → edge
- `$variable` → substitution

Pas besoin d'un parser Mermaid complet — juste les patterns ci-dessus avec une regex ou un parser simple ligne par ligne.

### 3.3 — GraphNode (`dataflow/graph_node.rs`)

Un graphe wrappé en Node. Ports exposés = bords libres (inputs non connectés, outputs non consommés).

```rust
pub struct GraphNode {
    name: String,
    graph: DataflowGraph,
    inputs: Vec<PortDef>,   // déduits
    outputs: Vec<PortDef>,  // déduits
}
```

Annotations `@expose_input`/`@expose_output` pour renommer les ports.

### 3.4 — Templates built-in

4 fichiers `.mmd` :
1. Search simple (pas d'expansion)
2. Search avec expansion
3. Search avec double expansion (matched + others)
4. Ingestion avec normalisation LLM

### 3.5 — Export

`DataflowGraph::to_mermaid()` — génère le Mermaid depuis le graphe (pour debug, doc).

### Tests

- Parse chaque template → validate → OK
- Parse template + execute → résultats corrects
- GraphNode exposed_ports() détecte les bords libres
- Erreurs de validation lisibles (port inexistant, type mismatch)
- Variable substitution
- to_mermaid() roundtrip

**Fichiers** : `dataflow/parser.rs`, `dataflow/node_registry.rs`, `dataflow/graph_node.rs`, modif `graph.rs`

---

## Phase 4 : Migrations

**Objectif** : Migrations de schema graph à la Supabase, basées sur le framework dataflow.

### 4.1 — Nœuds de migration

| Nœud | Inputs | Outputs | Action |
|------|--------|---------|--------|
| `QueryNode` | — | entities | MATCH Cypher |
| `BackupNode` | entities | entities (pass-through) | Sauvegarde les valeurs originales |
| `ValidateNode` | entities | entities (pass-through) | Assert (count, schema) |
| `TransformNode` | entities | entities | Rename, merge, convert, compute |
| `WriteNode` | entities | uuids | SET/CREATE/DELETE Cypher |
| `RenameFieldNode` | entities | entities | Raccourci pour rename |
| `AddFieldNode` | entities | entities | Raccourci pour add field |

### 4.2 — Schema tracking

```
_DataflowMigration { version, name, status, applied_at, execution_uuid, checksum }
```

### 4.3 — MigrationRunner

```rust
pub struct MigrationRunner { ... }

impl MigrationRunner {
    pub async fn pending(&self) -> Vec<MigrationFile>;
    pub async fn apply(&self, version: &str, mode: MigrationMode) -> MigrationResult;
    pub async fn rollback_last(&self) -> MigrationResult;
    pub async fn status(&self) -> Vec<MigrationStatus>;
}

pub enum MigrationMode { Apply, DryRun, Rollback { execution_uuid: String } }
```

### 4.4 — Convention fichiers

```
migrations/
├── 001_initial_schema.mmd
├── 002_add_user_email.mmd
├── 003_rename_field.mmd
```

### Tests

- Apply migration → verify schema changed
- Dry-run → verify nothing changed
- Rollback → verify schema restored
- Status → list applied/pending

**Fichiers** : `dataflow/migration_nodes.rs`, `dataflow/migrations.rs`

---

## Phase 5 : Rhai ScriptNode

**Objectif** : Nœuds custom en Rhai pour les power users.

### 5.1 — ScriptNode

```rust
pub struct ScriptNode {
    name: String,
    ast: rhai::AST,
    inputs: Vec<PortDef>,
    outputs: Vec<PortDef>,
}
```

Annotations dans le script :
```javascript
// @input results: Results
// @output filtered: Results
fn execute(results) { ... }
```

### 5.2 — ScriptDynamicNode

Flag `@dynamic` → le script reçoit `emit` pour ajouter des nœuds.

### 5.3 — PortValue ↔ Rhai

Conversion bidirectionnelle PortValue → rhai::Dynamic et retour.

### 5.4 — Sandbox

Limites : pas d'IO, pas de sleep, timeout, mémoire bornée.

### Tests

- Script filter → résultats filtrés
- Script dynamic → nœuds émis
- Script invalid → erreur claire
- Sandbox timeout

**Fichiers** : `dataflow/script_node.rs`, modif `Cargo.toml` (rhai feature flag)

---

## Résumé

| Phase | Scope | Fichiers créés | Modifiés | Supprimés |
|-------|-------|---------------|----------|-----------|
| 1 | Core + Search | 7 (`dataflow/*.rs`) | `catalog.rs`, `lib.rs`, `e2e_search_queue.rs` | `search_queue.rs`, `processors.rs` |
| 2 | Observabilité | 3 (`observe.rs`, `record.rs`, `report.rs`) | `runtime.rs` | — |
| 3 | Mermaid | 3 (`parser.rs`, `node_registry.rs`, `graph_node.rs`) + 4 `.mmd` | `graph.rs` | — |
| 4 | Migrations | 2 (`migration_nodes.rs`, `migrations.rs`) | — | — |
| 5 | Rhai | 1 (`script_node.rs`) | `Cargo.toml` | — |

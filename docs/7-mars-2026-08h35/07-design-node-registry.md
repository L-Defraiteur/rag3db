# Doc 07 — Design : NodeRegistry + suppression DynamicNode

Date : 7 mars 2026

## Motivation

Aujourd'hui, la création de nœuds est codée en dur à 3 endroits :

1. **`build_ingestion_graph()`** — `Box::new(InsertRecordNode::new("inserts"))` etc.
2. **`build_dataflow_graph()`** — `Box::new(QuerySourceNode::new(...))` etc.
3. **`create_node_from_checkpoint()`** — `match node_type { "InsertRecordNode" => ... }`

Le problème :
- Ajouter un nœud = modifier 3 match/if en dur
- Le checkpoint a un match qui duplique la logique de construction
- Pas possible de définir un pipeline en Mermaid sans savoir instancier un nœud à partir de son nom
- Pas extensible (nœuds custom / plugins)

De plus, `DynamicNode` + `GraphEmitter` + la boucle itérative d'expansion dans le runtime ajoutent de la complexité pour un seul use case (ExpansionNode) qui peut être rendu statique.

## Décision : suppression de DynamicNode

### Pourquoi

Le seul `DynamicNode` en production est `ExpansionNode`. Il émet dynamiquement N `FetchRelatedNode` + 1 `ComposeNode` selon les résultats. Mais le nombre de rules est connu à la construction du graphe — seuls les `parents` sont runtime. On peut passer les parents via ports.

### Ce qu'on supprime

| Élément | Fichier | Lignes |
|---|---|---|
| `DynamicNode` trait | `node.rs` | ~25 lignes |
| `GraphEmitter` struct | `node.rs` | ~80 lignes |
| `NodeSlot` enum (Static/Dynamic) | `graph.rs` | ~20 lignes |
| `add_dynamic_node()` | `graph.rs` | ~15 lignes |
| Boucle itérative d'expansion dans runtime | `runtime.rs` | ~60 lignes |
| `ExpansionNode` (DynamicNode impl) | `search_nodes.rs` | ~100 lignes |

Total : ~300 lignes supprimées.

### Remplacement : DispatchExpansionNode (statique)

Avant (dynamique) :
```
primary_search ──results──▶ ExpansionNode [DynamicNode]
                              │ (émet dynamiquement au runtime)
                              ├──▶ fetch_related_0 ──children──▶ compose
                              ├──▶ fetch_related_1 ──children──┘
                              └──▶ compose ◄── results ──────────┘
```

Après (statique) :
```
primary_search ──results──▶ DispatchExpansionNode
                              ├──parents_0──▶ FetchRelatedNode("fetch_0") ──children──▶ ComposeNode
                              ├──parents_1──▶ FetchRelatedNode("fetch_1") ──children──┘
                              └──results─────────────────────────results──┘
```

### DispatchExpansionNode

```rust
/// Filtre les résultats par rule et émet les parents sur des ports nommés.
/// Un port `parents_{i}` par rule d'expansion.
pub struct DispatchExpansionNode {
    name: String,
    rules: Vec<ExpansionRule>,
}

#[async_trait]
impl Node for DispatchExpansionNode {
    fn inputs(&self) -> &[PortDef] {
        &[PortDef { name: "results", port_type: PortType::Results, required: true }]
    }
    fn outputs(&self) -> &[PortDef] {
        // "results" (passthrough) + "parents_0", "parents_1", ... (un par rule)
        // Note : outputs dynamiques par le nombre de rules (connu au build)
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let results = ctx.take_input("results")?;
        for (i, rule) in self.rules.iter().enumerate() {
            let parents = filter_parents(&results, rule);
            ctx.set_output(&format!("parents_{i}"), PortValue::Uuids(parents));
        }
        ctx.set_output("results", PortValue::Results(results));
        Ok(())
    }
}
```

### FetchRelatedNode modifié

```rust
// Avant : parents dans constructeur (baked in par ExpansionNode)
pub struct FetchRelatedNode {
    conn: Arc<dyn DbConnection>,
    parents: Vec<(String, String)>,  // ← construit dynamiquement
    relation: String, direction: ExpansionDirection, limit: usize,
}

// Après : parents via input port, conn via service
pub struct FetchRelatedNode {
    name: String,
    relation: String,
    direction: ExpansionDirection,
    limit: usize,
}

impl Node for FetchRelatedNode {
    fn inputs(&self) -> &[PortDef] {
        &[PortDef { name: "parents", port_type: PortType::Uuids, required: true }]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let conn = ctx.service::<dyn DbConnection>("conn").ok_or("...")?;
        let parents = ctx.take_input("parents"); // Vec<(source_uuid, result_uuid)>
        // ... même logique de traversal Cypher
    }
}
```

### build_dataflow_graph() modifié

```rust
pub async fn build_dataflow_graph(catalog, kb_name, query, strategy) -> DataflowGraph {
    let mut graph = DataflowGraph::new();

    graph.add_node(Box::new(QuerySourceNode::new(kb_name, query, &strategy.search)))?;
    graph.add_node(Box::new(PrimarySearchNode::new()))?;
    graph.connect("query_source", "query", "primary_search", "query")?;

    if !strategy.expansions.is_empty() {
        // Dispatch : filtre parents par rule
        graph.add_node(Box::new(DispatchExpansionNode::new(
            "expansion", strategy.expansions.clone(),
        )))?;
        graph.connect("primary_search", "results", "expansion", "results")?;

        // Un FetchRelatedNode par rule
        for (i, rule) in strategy.expansions.iter().enumerate() {
            let fetch_name = format!("fetch_related_{i}");
            graph.add_node(Box::new(FetchRelatedNode::new(
                &fetch_name, rule.relation.clone(), rule.direction, rule.limit,
            )))?;
            graph.connect("expansion", &format!("parents_{i}"), &fetch_name, "parents")?;
        }

        // ComposeNode collecte tous les children
        graph.add_node(Box::new(ComposeNode::new("compose")))?;
        graph.connect("expansion", "results", "compose", "results")?;
        for (i, _) in strategy.expansions.iter().enumerate() {
            graph.connect(&format!("fetch_related_{i}"), "children", "compose", "children")?;
        }
    }

    graph
}
```

### Impact sur le runtime

Le runtime se simplifie considérablement :

```rust
// Avant : boucle itérative avec expansion
loop {
    let ready = graph.topo_sort_ready();
    for node in ready {
        match &mut graph.nodes[idx] {
            NodeSlot::Static(n) => n.execute(&mut ctx).await?,
            NodeSlot::Dynamic(n) => {
                let mut emitter = GraphEmitter::new();
                n.execute_dynamic(&mut ctx, &mut emitter).await?;
                graph.apply_mutations(emitter)?;  // ← complexe
            }
        }
    }
    if !graph.has_pending_mutations() { break; }
}

// Après : une seule passe topologique
let order = graph.topo_sort()?;
for node_idx in order {
    node.execute(&mut ctx).await?;
}
```

## Design NodeRegistry

### NodeFactory trait

```rust
/// Factory pour un type de nœud. Une instance par type (pas par instance de nœud).
pub trait NodeFactory: Send + Sync {
    /// Crée un nœud à partir de son nom d'instance et sa config JSON.
    ///
    /// Les dépendances lourdes (conn, embedder, etc.) sont résolues
    /// à l'exécution via `ctx.service()` — PAS à la création.
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String>;

    /// Identifiant du type (= ce que retourne `node.node_type()`).
    fn node_type(&self) -> &str;

    /// Schema déclaratif : inputs, outputs, paramètres config acceptés.
    fn schema(&self) -> NodeSchema;
}
```

### NodeSchema

```rust
pub struct NodeSchema {
    pub node_type: String,
    pub description: String,
    pub inputs: Vec<PortDef>,
    pub outputs: Vec<PortDef>,
    pub config_params: Vec<ConfigParam>,
}

pub struct ConfigParam {
    pub name: String,
    pub param_type: ConfigParamType,  // String, Int, Float, Bool, Json
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: String,
}

pub enum ConfigParamType {
    String,
    Int,
    Float,
    Bool,
    Json,
}
```

### NodeRegistry

```rust
pub struct NodeRegistry {
    factories: HashMap<String, Box<dyn NodeFactory>>,
}

impl NodeRegistry {
    pub fn new() -> Self;

    /// Enregistre une factory. Clé = node_type.
    pub fn register(&mut self, factory: Box<dyn NodeFactory>);

    /// Crée un nœud par type + nom + config.
    pub fn create(
        &self,
        node_type: &str,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn Node>, String>;

    /// Retourne le schema d'un type de nœud.
    pub fn schema(&self, node_type: &str) -> Option<&NodeSchema>;

    /// Liste tous les types enregistrés.
    pub fn types(&self) -> Vec<&str>;

    /// Registry par défaut avec tous les built-in nodes.
    pub fn with_builtins() -> Self;
}
```

## Nœuds à enregistrer — 14 types

### Record nodes (ingestion) — 8 types

| node_type | Config | Notes |
|---|---|---|
| `InsertRecordNode` | — | nom seulement |
| `LinkRecordNode` | — | nom seulement |
| `EmbedRecordNode` | `gpu_batch_size: usize` (default 32) | seul nœud avec config |
| `ChunkRecordNode` | — | |
| `GatherKBNode` | — | |
| `UpdateKBNode` | — | |
| `ChunkKBNode` | — | |
| `FlushFTSNode` | — | |

Pas de changement nécessaire : tous accèdent déjà aux dépendances via `ctx.service()`.

### Search nodes — 5 types (après refactoring)

| node_type | Config | Changement nécessaire |
|---|---|---|
| `QuerySourceNode` | `kb_name`, `query`, `options` | Config seulement, OK |
| `PrimarySearchNode` | — | **Refactor** : `catalog: Arc<Mutex<Catalog>>` → `ctx.service("catalog")` |
| `DispatchExpansionNode` | `rules: Vec<ExpansionRule>` (JSON) | **Nouveau** : remplace `ExpansionNode` (DynamicNode) |
| `FetchRelatedNode` | `relation`, `direction`, `limit` | **Refactor** : `conn` → `ctx.service("conn")`, `parents` → input port |
| `ComposeNode` | — | Pas de changement |

### Nœud supprimé

| Ancien | Remplacement |
|---|---|
| `ExpansionNode` (DynamicNode) | `DispatchExpansionNode` (Node statique) |

## Intégration

### 1. Checkpoint — remplace `create_node_from_checkpoint()`

```rust
// Avant : match en dur dans checkpoint.rs
pub fn create_node_from_checkpoint(name, node_type, config) -> Box<dyn Node> {
    match node_type { "InsertRecordNode" => ..., /* 8 arms */ }
}

// Après : une ligne
let node = registry.create(node_type, name, config)?;
```

### 2. `drain_resume()` — utilise le registry

```rust
for node_def in &checkpoint.graph_def.nodes {
    let node = self.registry.create(&node_def.node_type, &node_def.name, &node_def.config)?;
    graph.add_node(node)?;
}
```

### 3. `build_ingestion_graph()` — pas de changement immédiat

La logique conditionnelle reste. Optionnellement :
```rust
graph.add_node(registry.create("InsertRecordNode", "inserts", &json!({}))?)?;
```

La vraie valeur du registry pour l'ingestion viendra avec les templates Mermaid (Phase 3.2).

## Refactoring search nodes

### PrimarySearchNode

```rust
// Avant
pub struct PrimarySearchNode { catalog: Arc<Mutex<Catalog>> }

// Après
pub struct PrimarySearchNode { name: String }

impl Node for PrimarySearchNode {
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let catalog = ctx.service::<Mutex<Catalog>>("catalog")
            .ok_or("PrimarySearchNode: 'catalog' service not registered")?;
        // ... même logique
    }
}
```

### FetchRelatedNode

```rust
// Avant : conn + parents dans constructeur
pub struct FetchRelatedNode {
    conn: Arc<dyn DbConnection>,
    parents: Vec<(String, String)>,
    relation: String, direction: ExpansionDirection, limit: usize,
}

// Après : conn via service, parents via input port, config pour le reste
pub struct FetchRelatedNode {
    name: String,
    relation: String,
    direction: ExpansionDirection,
    limit: usize,
}
```

## Implémentation — Étapes

### Étape 1 — Supprimer DynamicNode + simplifier runtime

| Fichier | Changement |
|---|---|
| `node.rs` | Supprimer `DynamicNode` trait, `GraphEmitter` struct |
| `graph.rs` | Supprimer `NodeSlot`, `add_dynamic_node()`. Les nœuds sont `Vec<Box<dyn Node>>` |
| `runtime.rs` | Supprimer boucle itérative d'expansion, simplifier execute() |
| `search_nodes.rs` | Supprimer `ExpansionNode` |
| `mod.rs` | Supprimer exports DynamicNode, GraphEmitter |

Tests : vérifier que les tests existants de search et ingestion passent toujours (sauf ceux qui testaient spécifiquement DynamicNode/GraphEmitter).

### Étape 2 — DispatchExpansionNode + refactoring FetchRelatedNode

| Fichier | Changement |
|---|---|
| `search_nodes.rs` | `DispatchExpansionNode` : filtre parents par rule, outputs nommés |
| `search_nodes.rs` | `FetchRelatedNode` : parents via input port, conn via service |
| `search_nodes.rs` | `PrimarySearchNode` : catalog via service |
| `catalog.rs` | `build_dataflow_graph()` : construction statique du graphe search |

Tests : E2E search passent, tests unitaires search passent.

### Étape 3 — NodeSchema + NodeFactory trait + NodeRegistry struct

Fichier : `src/dataflow/node_registry.rs`

- `ConfigParam`, `ConfigParamType`, `NodeSchema`
- `NodeFactory` trait (retourne `Box<dyn Node>`)
- `NodeRegistry` (register, create, schema, types, with_builtins)

Tests : `register_and_create`, `unknown_type_errors`, `with_builtins_has_all`, `schema_describes_ports`

### Étape 4 — Factories pour les 13 node types

Fichier : `node_registry.rs`

Macro `simple_factory!` pour les nœuds sans config (10 sur 13).
Factories manuelles pour `EmbedRecordNode`, `DispatchExpansionNode`, `FetchRelatedNode`.

Tests : `create_all_nodes_from_registry`, `embed_node_config_roundtrip`, `dispatch_expansion_config_roundtrip`

### Étape 5 — Intégration checkpoint + cleanup

| Fichier | Changement |
|---|---|
| `checkpoint.rs` | Supprimer `create_node_from_checkpoint()` |
| `catalog.rs` | Stocker `NodeRegistry` dans Catalog, utiliser dans `drain_resume()` |
| `mod.rs` | Supprimer export `create_node_from_checkpoint` |

Tests : `checkpoint_resume_uses_registry` (remplace `create_node_from_checkpoint_all_types`)

### Étape 6 — Helper `DataflowGraph::add_from_registry()`

```rust
pub fn add_from_registry(
    &mut self,
    registry: &NodeRegistry,
    node_type: &str,
    name: &str,
    config: &serde_json::Value,
) -> Result<(), String>
```

Tests : `graph_add_from_registry`

## Vérification

```bash
cargo test --lib           # ~390 pass, 0 régression
./run_e2e.sh               # 86 E2E pass
grep -r "DynamicNode\|GraphEmitter\|NodeSlot" src/  # 0 match (hors tests legacy)
grep -r "create_node_from_checkpoint" src/           # 0 match
```

## Ce qui N'EST PAS dans ce doc

- **Mermaid parser** → Phase 3.2 (doc séparé)
- **GraphNode** → Phase 3.3 (doc séparé)
- **Rhai ScriptNode** → Phase 5

Le NodeRegistry est la **fondation** sur laquelle ces features seront construites. La suppression de DynamicNode simplifie toute la stack pour la suite.

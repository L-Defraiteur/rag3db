# Doc 20 — Design : Pure Dataflow Ingestion

Date : 6 mars 2026

## Objectif

Transformer le pipeline d'ingestion en un vrai dataflow composable, où les données circulent sur les ports (pas dans les constructeurs), les nœuds sont instanciables par nom, et le graphe est descriptible en JSON — pour permettre un éditeur visuel type n8n / shader graph.

## Diagnostic : état actuel

### Ce qui marche

- **Framework dataflow** : `Node` / `DynamicNode` traits, `DataflowGraph`, `DataflowRuntime`, topo sort, edges typées, `DataflowEvent`, observabilité (tap, report, record).
- **7 ingestion nodes** : `ChunkBatchNode`, `InsertBatchNode`, `LinkBatchNode`, `AggregateBatchNode`, `EmbedBatchNode`, `SparseEmbedBatchNode`, `DualEmbedBatchNode`.
- **`ServiceRegistry`** : existe déjà (`TypeId → Arc<dyn Any>`), pas utilisé par l'ingestion.

### Ce qui ne va pas (5 problèmes)

**P1. Data baked dans les constructeurs**

```rust
// Actuel — les ops sont dans le struct, pas sur les ports
InsertBatchNode::new("inserts", ops, conn, cache)
```

Les edges `trigger → done` transportent `PortValue::Empty`. Aucune donnée ne circule sur les edges d'ingestion. Ce sont des batch processors séquencés, pas un dataflow.

**P2. Pas de `PortValue` pour l'ingestion**

`PortValue` a 9 variants (Results, Children, Query, Meta, ...) — tous pour la search. Rien pour les ops, chunks, embeddings. Impossible de connecter un nœud custom à la sortie d'un InsertBatchNode.

**P3. Services passés en constructeurs**

```rust
// ChunkBatchNode prend 12 arguments
ChunkBatchNode::new(config, kb_metadata, chunker_cache, has_sparse, has_dual,
    items, conn, node_id_cache, embedder, sparse_embedder, dual_embedder, embedding_dim)
```

`conn`, `embedder`, `node_id_cache` sont passés en `Arc` à chaque nœud. Le `ServiceRegistry` existe mais n'est pas utilisé. Un éditeur visuel ne peut pas instancier ces nœuds sans connaître les services.

**P4. `build_ingestion_graph()` monolithique**

120 lignes de code procédural dans `catalog.rs` qui hardcode la topologie : partition par type → `add_node` → `connect`. La topologie n'est pas descriptive, pas sérialisable, pas modifiable par l'utilisateur.

**P5. `unsafe` pour muter `&self`**

```rust
let items = unsafe {
    &mut *(std::ptr::addr_of!(self.items) as *mut Vec<InsertOp>)
};
```

6 occurrences. Causé par `Node::execute(&self, ctx)` qui prend `&self`. Les nœuds ont besoin de muter leurs items (résolution d'EntityRef, etc.).

## Design cible

### Architecture visuelle

```
┌──────────┐     ┌───────────┐     ┌──────────┐     ┌───────────┐
│ OpsSource│──ops→│ SplitOps  │──ins→│ Insert   │──uuids→│ ...       │
│          │     │           │──lnk→│ Link     │     │           │
│          │     │           │──chk→│ Chunk    │──ops→│ Insert    │
│          │     │           │──agg→│ Aggregate│──ops→│ Embed     │
│          │     │           │──emb→│ Embed    │     └───────────┘
└──────────┘     └───────────┘     └──────────┘
```

Chaque flèche = données typées sur un port. Chaque nœud = instanciable par nom + config JSON. Le graphe entier = descriptible en JSON.

### Changement A : `PortType` / `PortValue` — variants ingestion

```rust
// Nouveaux variants PortType
PortType::Ops,          // Vec<CatalogOp>
PortType::Inserts,      // Vec<InsertOp>
PortType::Links,        // Vec<LinkOp>
PortType::Embeds,       // Vec<EmbedOp>
PortType::SparseEmbeds, // Vec<SparseEmbedOp>
PortType::DualEmbeds,   // Vec<DualEmbedOp>
PortType::Chunks,       // Vec<ChunkOp>
PortType::Aggregates,   // Vec<AggregateOp>
PortType::Uuids,        // Vec<(String, String)> — déjà existant, réutilisé

// Nouveaux variants PortValue correspondants
PortValue::Ops(Vec<CatalogOp>),
PortValue::Inserts(Vec<InsertOp>),
PortValue::Links(Vec<LinkOp>),
PortValue::Embeds(Vec<EmbedOp>),
PortValue::SparseEmbeds(Vec<SparseEmbedOp>),
PortValue::DualEmbeds(Vec<DualEmbedOp>),
PortValue::Chunks(Vec<ChunkOp>),
PortValue::Aggregates(Vec<AggregateOp>),
```

**Alternative : `PortValue::Ops(Vec<CatalogOp>)` seul + split dans chaque nœud.**
Plus simple (1 variant au lieu de 8), mais perd le typage statique des edges. L'éditeur visuel ne saurait pas quel type d'op sort de quel port.

**Recommandation** : variants typés (8 variants). C'est plus de code mais c'est ce qui rend l'éditeur utile — l'utilisateur voit "ce port sort des InsertOps" vs "ce port sort des EmbedOps".

### Changement B : `Node::execute(&mut self, ctx)` — signature mutable

```rust
#[async_trait]
pub trait Node: Send + Sync {
    fn name(&self) -> &str;
    fn inputs(&self) -> &[PortDef];
    fn outputs(&self) -> &[PortDef];
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String>;
}
```

Élimine les 6 `unsafe` hacks. Le runtime possède déjà les nœuds en `Box<dyn Node>`, donc `&mut` est safe.

**Impact** : modifier `graph.rs` pour stocker `Box<dyn Node>` sans Mutex (le runtime exécute séquentiellement par nœud), et `runtime.rs` pour appeler `execute(&mut self)`.

**DynamicNode** idem :
```rust
async fn execute_dynamic(&mut self, ctx: &mut NodeContext, emitter: &mut GraphEmitter) -> Result<(), String>;
```

### Changement C : `ServiceRegistry` dans `NodeContext`

```rust
pub struct NodeContext {
    inputs: HashMap<String, PortValue>,
    outputs: HashMap<String, PortValue>,
    services: Arc<ServiceRegistry>,  // ← nouveau
}

impl NodeContext {
    pub fn service<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.services.get::<T>()
    }
}
```

Le `DataflowRuntime` reçoit un `ServiceRegistry` à la construction et le passe à chaque `NodeContext`.

```rust
impl DataflowRuntime {
    pub fn new(max_iterations: usize, services: ServiceRegistry) -> Self { ... }
}
```

Les services enregistrés par le `Catalog` :

| Service | Type | Clé |
|---|---|---|
| DB connection | `dyn DbConnection` | `DbConnection` |
| Dense embedder | `dyn Embedder` | `Embedder` |
| Sparse embedder | `dyn SparseEmbedder` | `SparseEmbedder` |
| Dual embedder | `dyn DualEmbedder` | `DualEmbedder` |
| Node ID cache | `RwLock<NodeIdCache>` | `RwLock<NodeIdCache>` |
| Catalog config | `CatalogConfig` | `CatalogConfig` |
| KB metadata | `HashMap<String, KBMetadata>` | custom wrapper |

**Impact sur les nœuds** : les constructeurs ne prennent plus les services. Le nœud les récupère dans `execute()` :

```rust
async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
    let conn = ctx.service::<dyn DbConnection>()
        .ok_or("DbConnection service not registered")?;
    // ...
}
```

**Problem: `dyn DbConnection` est un trait object, pas un type concret.** `TypeId::of::<dyn DbConnection>()` ne marche pas. Solution : wrapper newtype.

```rust
pub struct ConnService(pub Arc<dyn DbConnection>);
pub struct EmbedService(pub Arc<dyn Embedder>);
// etc.
```

Ou : registry par string key au lieu de TypeId.

```rust
pub struct ServiceRegistry {
    services: HashMap<String, Arc<dyn Any + Send + Sync>>,
}
registry.register("conn", conn.clone());
let conn: Arc<dyn DbConnection> = registry.get("conn")?;
```

**Recommandation** : string keys — plus simple, compatible JSON config, pas de newtypes.

### Changement D : `SplitOpsNode` — routeur par type

Nœud utilitaire qui prend un `Vec<CatalogOp>` en input et le distribue sur des ports typés.

```rust
struct SplitOpsNode;

impl Node for SplitOpsNode {
    fn name(&self) -> &str { "split_ops" }
    fn inputs(&self) -> &[PortDef] {
        &[PortDef { name: "ops", port_type: PortType::Ops, required: true }]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "inserts",  port_type: PortType::Inserts,  required: false },
            PortDef { name: "links",    port_type: PortType::Links,    required: false },
            PortDef { name: "chunks",   port_type: PortType::Chunks,   required: false },
            PortDef { name: "aggregates", port_type: PortType::Aggregates, required: false },
            PortDef { name: "embeds",   port_type: PortType::Embeds,   required: false },
            PortDef { name: "sparse_embeds", port_type: PortType::SparseEmbeds, required: false },
            PortDef { name: "dual_embeds",   port_type: PortType::DualEmbeds,   required: false },
        ]
    }
}
```

Premier nœud dans tout graphe d'ingestion. Remplace le `match op { ... }` de `build_ingestion_graph()`.

### Changement E : Nœuds data-on-ports

Les nœuds reçoivent leurs ops via le port input, plus via le constructeur.

**Avant** :
```rust
InsertBatchNode::new("inserts", ops, conn, cache)
// execute: itère sur self.items
```

**Après** :
```rust
InsertBatchNode::new("inserts")
// execute: let ops = ctx.take_input("ops")? -> PortValue::Inserts(items)
//          let conn = ctx.service::<ConnService>()?
```

Le nœud est **stateless** (sauf son nom). Toute donnée arrive par les ports, tout service arrive par le registry.

**Impact sur DynamicNodes** : `ChunkBatchNode` et `AggregateBatchNode` sont des `DynamicNode` — ils émettent de nouveaux nœuds downstream. Les nœuds émis reçoivent aussi leurs données par ports :

```rust
// ChunkBatchNode::execute_dynamic
let chunk_ops = ctx.take_input("ops")?; // PortValue::Chunks
// ... parallel chunking ...
// Émet un InsertBatchNode + set son input port
emitter.add_node(Box::new(InsertBatchNode::new("chunk_inserts")));
emitter.set_initial_input("chunk_inserts", "ops", PortValue::Inserts(inserts));
emitter.connect("chunk_batch", "done", "chunk_inserts", "trigger");
```

Besoin d'ajouter `set_initial_input()` au `GraphEmitter` — permet de passer des données initiales aux nœuds émis dynamiquement.

### Changement F : `NodeDescriptor` + `NodeRegistry`

Pour l'éditeur visuel : chaque type de nœud expose une description statique.

```rust
pub struct NodeDescriptor {
    pub type_name: &'static str,    // "InsertBatch"
    pub category: &'static str,     // "ingestion", "search", "transform"
    pub description: &'static str,  // "Batch INSERT via Cypher"
    pub inputs: &'static [PortDef],
    pub outputs: &'static [PortDef],
    pub config_schema: Option<serde_json::Value>, // JSON Schema des params
}

pub struct NodeRegistry {
    descriptors: HashMap<String, NodeDescriptor>,
    factories: HashMap<String, Box<dyn Fn(serde_json::Value) -> Box<dyn Node>>>,
}

impl NodeRegistry {
    pub fn register<F>(&mut self, desc: NodeDescriptor, factory: F)
    where F: Fn(serde_json::Value) -> Box<dyn Node> + 'static;

    pub fn create(&self, type_name: &str, config: serde_json::Value) -> Option<Box<dyn Node>>;
    pub fn list(&self) -> Vec<&NodeDescriptor>;
}
```

L'éditeur visuel appelle `registry.list()` pour afficher les nœuds disponibles dans la palette, et `registry.create()` pour instancier.

### Changement G : Graph descriptif (JSON)

```json
{
  "nodes": [
    { "id": "source", "type": "OpsSource" },
    { "id": "split", "type": "SplitOps" },
    { "id": "insert", "type": "InsertBatch" },
    { "id": "link", "type": "LinkBatch" },
    { "id": "embed", "type": "EmbedBatch" }
  ],
  "edges": [
    { "from": "source:ops", "to": "split:ops" },
    { "from": "split:inserts", "to": "insert:ops" },
    { "from": "split:links", "to": "link:ops" },
    { "from": "insert:done", "to": "link:trigger" },
    { "from": "link:done", "to": "embed:trigger" },
    { "from": "split:embeds", "to": "embed:ops" }
  ],
  "services": ["conn", "embedder", "node_id_cache"]
}
```

`DataflowGraph::from_json(json, &registry)` construit le graphe.
`DataflowGraph::to_json()` sérialise (pour sauvegarder depuis l'éditeur).

`build_ingestion_graph()` dans `catalog.rs` devient un one-liner qui charge le JSON par défaut.

## Ordre d'implémentation

### Phase 1 : Fondations (A + B + C) — pas de breaking changes fonctionnels

| Étape | Fichier | Quoi | Tests |
|---|---|---|---|
| 1.1 | `port.rs` | Ajouter 8 variants PortType + PortValue pour ingestion | 3 unit |
| 1.2 | `node.rs` | `execute(&mut self)` + `execute_dynamic(&mut self)` | Mise à jour des impls |
| 1.3 | `graph.rs` | Adapter stockage pour `&mut` (pas besoin de Mutex) | Existants passent |
| 1.4 | `runtime.rs` | `execute` appelle `&mut self` + passe services dans ctx | Existants passent |
| 1.5 | `services.rs` | Passer à string keys, `ServiceRegistry` dans `NodeContext` | 2 unit |

**Validation** : `cargo test --lib` — 375+ tests passent, 0 regression.

### Phase 2 : Nœuds data-on-ports (D + E)

| Étape | Fichier | Quoi | Tests |
|---|---|---|---|
| 2.1 | `ingestion_nodes.rs` | `SplitOpsNode` — nouveau nœud routeur | 3 unit |
| 2.2 | `ingestion_nodes.rs` | `InsertBatchNode` — input port `ops: Inserts`, services via ctx | Existants |
| 2.3 | `ingestion_nodes.rs` | `LinkBatchNode` — idem | Existants |
| 2.4 | `ingestion_nodes.rs` | `EmbedBatchNode` / `SparseEmbedBatchNode` / `DualEmbedBatchNode` — idem | Existants |
| 2.5 | `ingestion_nodes.rs` | `ChunkBatchNode` — `DynamicNode`, input port `ops: Chunks`, `emitter.set_initial_input()` | Existants |
| 2.6 | `ingestion_nodes.rs` | `AggregateBatchNode` — idem | Existants |
| 2.7 | `catalog.rs` | Réécrire `build_ingestion_graph()` avec `SplitOpsNode` + services | Tous E2E |

**Validation** : tous les 459 tests E2E passent avec le nouveau pipeline.

### Phase 3 : Registry + sérialisation (F + G)

| Étape | Fichier | Quoi | Tests |
|---|---|---|---|
| 3.1 | `dataflow/registry.rs` | `NodeDescriptor` + `NodeRegistry` + `register()` / `create()` / `list()` | 5 unit |
| 3.2 | `dataflow/registry.rs` | Enregistrer les 8 nœuds built-in (7 ingestion + SplitOps) + 4 search | 1 unit |
| 3.3 | `dataflow/graph.rs` | `from_json()` + `to_json()` — sérialisation du graphe | 3 unit |
| 3.4 | `catalog.rs` | `build_ingestion_graph()` charge le JSON default template | E2E |

**Validation** : round-trip JSON → graph → exécution → même résultat.

## Estimation

| Phase | Lignes ajoutées | Lignes modifiées | Net |
|---|---|---|---|
| Phase 1 | ~150 | ~200 | +150 |
| Phase 2 | ~100 | ~800 | -200 (suppression constructeurs/unsafe) |
| Phase 3 | ~300 | ~50 | +300 |
| Phase 4 | ~200 | ~30 | +200 |
| **Total** | ~750 | ~1080 | **+450** |

### Phase 4 : GraphNode — blocs haut-niveau composables

Le vrai game changer pour des utilisateurs non-experts. Un `GraphNode` encapsule un sous-graphe complet derrière des ports simples — le pattern exact de Blender (shader nodes) et Unreal (Blueprints).

```rust
struct GraphNode {
    name: String,
    inner_graph: DataflowGraph,
    exposed_inputs: Vec<(String, String)>,   // (external_port, inner_node:port)
    exposed_outputs: Vec<(String, String)>,
}
```

**Vue "expert"** (nœuds bas-niveau) :
```
SplitOps → Insert → Link → Aggregate → Chunk → Embed
```

**Vue "simple"** (blocs haut-niveau) :
```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────┐
│  Ingest Docs    │──→──│ Make Searchable   │──→──│ Link Rels   │
└─────────────────┘     └──────────────────┘     └─────────────┘
```

| Bloc haut-niveau | Contenu réel | Config exposée |
|---|---|---|
| **Ingest Docs** | Split → Insert | entity type, fields |
| **Make Searchable** | Aggregate → Chunk → Embed | chunk size, model, KB name |
| **Link Relations** | Link | relation type, from/to |
| **Add Semantic Search** | Embed (dense) | model, dimension |

L'utilisateur compose avec 2-3 blocs. L'expert peut "ouvrir" un bloc et customiser l'intérieur. Le registry expose ces blocs au même titre que les nœuds atomiques.

| Étape | Fichier | Quoi | Tests |
|---|---|---|---|
| 4.1 | `dataflow/graph_node.rs` | `GraphNode` struct, `exposed_ports()` (bords libres = ports externes) | 3 unit |
| 4.2 | `dataflow/registry.rs` | Enregistrer les blocs haut-niveau comme `GraphNode` templates | 2 unit |
| 4.3 | `dataflow/graph.rs` | `flatten()` — inliner un GraphNode dans le graphe parent avant exécution | 2 unit |

## Cas d'usage : pipelines par KB

Le vrai bonus de cette architecture : **un pipeline d'ingestion différent par KB ou par type d'entité**.

Exemples concrets :

- **Code KB** : chunker AST (tree-sitter) au lieu du chunker texte par défaut, skip l'aggregation (un fichier = un document, pas de cross-entity), embedder code-spécialisé (CodeBERT)
- **Documentation KB** : chunker markdown (split par heading), aggregation cross-page (un chapitre = plusieurs pages), embedder multilingue
- **Chat/logs KB** : pas de chunking (messages courts), pas d'aggregation, sparse-only (BM25 suffit), skip dense embedding

Aujourd'hui `build_ingestion_graph()` construit le même pipeline pour tout. Avec le JSON descriptif (Phase 3), on pourrait avoir :

```
config/
  pipelines/
    default.json          ← pipeline standard (Split → Insert → Link → Aggregate → Chunk → Embed)
    code.json             ← pipeline code (Split → Insert → ASTChunk → CodeEmbed)
    lightweight.json      ← pipeline léger (Split → Insert → Link, pas d'embedding)
```

Le `CatalogConfig` référencerait le pipeline par KB :

```rust
pub struct KBConfig {
    // ... champs existants ...
    pub pipeline: Option<String>,  // "code", "lightweight", ou None = default
}
```

Et `build_ingestion_graph()` chargerait le bon template :
```rust
let pipeline_name = kb_config.pipeline.as_deref().unwrap_or("default");
let json = load_pipeline_template(pipeline_name)?;
DataflowGraph::from_json(json, &registry)
```

C'est le même graphe runtime, juste une topologie différente. Les tests E2E valident chaque variante indépendamment.

À garder en tête lors de l'implémentation des Phases 1-3 : ne pas hardcoder d'hypothèses sur la topologie dans les nœuds. Chaque nœud doit fonctionner indépendamment de ses voisins — c'est ce qui permet de les recomposer librement.

## Points de discussion

1. **PortValue granulaire (8 variants) vs générique (1 `Ops` variant)** — 8 variants = meilleur pour l'éditeur visuel (typage des connexions), plus de code. 1 variant = plus simple, split fait au runtime.

2. **ServiceRegistry string keys vs TypeId** — string keys = compatible JSON config, plus ergonomique. TypeId = type-safe au compile time, mais ne marche pas avec trait objects.

3. **`GraphEmitter::set_initial_input()`** — pour DynamicNodes qui émettent des nœuds avec données pré-chargées. Alternative : un port spécial `initial` sur le runtime.

4. **Phase 3 vs Mermaid** — le doc 10 prévoyait un parser Mermaid (Phase 3). JSON est plus simple à sérialiser/désérialiser et compatible avec un éditeur visuel React. Mermaid peut être un export read-only.

5. **Backward compat** — on n'en a pas besoin (lib en construction). Toutes les phases peuvent casser l'API interne tant que les E2E passent.

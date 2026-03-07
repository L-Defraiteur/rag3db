# Doc 13 — Design : GraphNode (sous-graphe composable)

Date : 7 mars 2026

## Motivation

On peut maintenant définir des pipelines en Mermaid et les matérialiser via `from_definition()`. Mais on ne peut pas **composer** des pipelines entre eux. Exemple : réutiliser un pipeline search comme nœud dans un pipeline plus grand, ou encapsuler un sous-graphe de migration comme un seul nœud.

GraphNode = un nœud qui contient un sous-graphe. Il implémente `Node` comme n'importe quel nœud, mais délègue `execute()` à l'exécution de son sous-graphe interne.

## Concept : ports exposés = bords libres

Un sous-graphe a des nœuds avec des ports non-connectés :
- **Entrées libres** = inputs du sous-graphe (ports d'entrée sans edge entrant)
- **Sorties libres** = outputs du sous-graphe (ports de sortie sans edge sortant)

Ces ports deviennent les inputs/outputs du GraphNode. Le nommage suit la convention `node_name.port_name` pour éviter les collisions.

### Exemple

Sous-graphe search :
```mermaid
graph LR
    qs["QuerySourceNode(kb_name='$kb', query='$q')"]
    ps["PrimarySearchNode"]
    qs -->|query| ps
```

Ports libres :
- **Entrées** : aucune (QuerySourceNode n'a pas d'input requis)
- **Sorties** : `ps.results`, `ps.meta`

Le GraphNode expose donc 0 inputs et 2 outputs. Si on l'insère dans un graphe parent, on peut connecter `graphnode.ps.results -->|results| next_node`.

### Convention de nommage des ports

Les ports du GraphNode sont nommés `{inner_node}.{port}` :
- Input : `{inner_node}.{input_port}` — port d'entrée sans edge entrant
- Output : `{inner_node}.{output_port}` — port de sortie sans edge sortant

Optionnel : le constructeur accepte des **aliases** pour simplifier : `alias("results", "ps.results")` → le port s'appelle juste `results` vu de l'extérieur.

## Flux d'exécution

```
Parent graph execute(GraphNode) :
  1. Matérialiser le sous-graphe : GraphDefinition → DataflowGraph via from_definition()
  2. Pour chaque input du GraphNode :
     - ctx.take_input("inner_node.port") → graph.set_initial_input("inner_node", "port", value)
  3. Créer DataflowRuntime::with_services(services) — même services que le parent
  4. runtime.execute(&mut sub_graph)
  5. Pour chaque sortie libre du sous-graphe :
     - output.get("inner_node", "port") → ctx.set_output("inner_node.port", value)
```

Les services (conn, embedder, etc.) sont partagés : le sous-graphe accède aux mêmes `Arc<T>` que le parent via le `ServiceRegistry` passé dans le `NodeContext`.

## Struct

```rust
pub struct GraphNode {
    name: String,
    definition: GraphDefinition,
    registry: Arc<NodeRegistry>,
    inputs: Vec<PortDef>,   // computed from free input ports
    outputs: Vec<PortDef>,  // computed from free output ports
    /// Map external port name → (inner_node, inner_port)
    input_map: HashMap<String, (String, String)>,
    output_map: HashMap<String, (String, String)>,
}
```

### Construction

```rust
impl GraphNode {
    pub fn from_definition(
        name: &str,
        definition: GraphDefinition,
        registry: Arc<NodeRegistry>,
    ) -> Result<Self, String>
```

Le constructeur :
1. Itère les NodeDef, query `registry.schema(node_type)` pour obtenir les PortDef de chaque nœud
2. Identifie les ports d'entrée sans edge entrant → `inputs`
3. Identifie les ports de sortie sans edge sortant → `outputs`
4. Construit `input_map` et `output_map`

### Aliases (optionnel, v2)

```rust
    pub fn alias_input(&mut self, alias: &str, inner: &str) -> Result<(), String>
    pub fn alias_output(&mut self, alias: &str, inner: &str) -> Result<(), String>
```

Renomme un port exposé pour simplifier les connexions côté parent.

## NodeFactory pour GraphNode

```rust
pub struct GraphNodeFactory {
    definition: GraphDefinition,
    registry: Arc<NodeRegistry>,
}
```

Enregistrable dans le NodeRegistry : `registry.register(Box::new(GraphNodeFactory { ... }))`.

Le `node_type()` serait dynamique (nom du template), par ex. `"SearchPipeline"`.

## Impact sur le code existant

- **Nouveau fichier** : `src/dataflow/graph_node.rs` (~200 lignes)
- **Modifié** : `src/dataflow/mod.rs` (ajouter module + export)
- **Modifié** : `src/dataflow/node_registry.rs` — `schema()` doit être accessible publiquement (déjà le cas)
- **Aucun changement** au runtime — GraphNode est un Node standard, le runtime l'exécute normalement

## Tests prévus (~10)

1. GraphNode depuis sous-graphe simple (2 nœuds) — ports détectés correctement
2. GraphNode execute — inputs injectés, outputs collectés
3. GraphNode dans un graphe parent — connexion + exécution bout-en-bout
4. Alias de port
5. Sous-graphe invalide (cycle) → erreur à la construction
6. Port manquant dans les inputs → erreur à l'exécution
7. Round-trip : Mermaid → GraphDefinition → GraphNode → exécution
8. node_type() et node_config() pour checkpoint
9. GraphNodeFactory — enregistrement + create()
10. Sous-graphe vide → erreur

## Complexité identifiée

### Services

Le GraphNode a besoin du `ServiceRegistry` au moment de `execute()`, pas au moment de la construction. C'est bon : `NodeContext::with_services()` le fournit, et on le passe au `DataflowRuntime` interne.

### PortType des ports exposés

Pour calculer les `PortDef` des ports exposés, on a besoin des schémas des nœuds internes. `NodeRegistry::schema()` retourne un `NodeSchema` avec les `PortDef`. Ça marche si tous les nœuds internes sont dans le registry (cas des builtins). Pour les GraphNode imbriqués (graph-in-graph-in-graph), le GraphNode parent doit aussi être enregistré dans le registry — c'est le rôle de `GraphNodeFactory`.

### Checkpoint

`GraphNode::node_type()` retourne un nom dynamique (ex: `"GraphNode:SearchPipeline"`).
`GraphNode::node_config()` retourne la `GraphDefinition` sérialisée.
Le checkpoint traite le GraphNode comme un nœud atomique (pas de checkpoint par nœud interne). Pour le checkpoint interne, c'est un futur nice-to-have.

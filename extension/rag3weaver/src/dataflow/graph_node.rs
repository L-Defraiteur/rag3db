//! GraphNode: a composable sub-graph node.
//!
//! Wraps a [`GraphDefinition`] and implements [`Node`], delegating `execute()`
//! to luciole's DAG engine via the bridge. Free (unconnected) ports of the inner
//! graph become the inputs/outputs of the GraphNode.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;


use super::checkpoint::GraphDefinition;
use super::graph::DataflowGraph;
use super::node::{Node, NodeContext};
use super::node_registry::{NodeFactory, NodeRegistry, NodeSchema};
use super::port::PortDef;

// ─── GraphNode ──────────────────────────────────────────────────────────────

/// A node that contains and executes a sub-graph.
///
/// Free input ports (inputs without incoming edges) become the GraphNode's inputs.
/// Free output ports (outputs without outgoing edges) become the GraphNode's outputs.
/// Port names follow the convention `inner_node.port_name`.
impl std::fmt::Debug for GraphNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphNode")
            .field("name", &self.name)
            .field("inputs", &self.inputs.len())
            .field("outputs", &self.outputs.len())
            .finish()
    }
}

pub struct GraphNode {
    name: String,
    /// `GraphNode` pour un sous-graphe monté à la main ; le type déclaré
    /// quand une [`GraphNodeFactory`] le crée.
    node_type: String,
    /// La configuration reçue de la fabrique — ce qu'un point de reprise
    /// lui repassera ; `None` pour un sous-graphe monté à la main.
    config: Option<serde_json::Value>,
    definition: GraphDefinition,
    registry: Arc<NodeRegistry>,
    inputs: Vec<PortDef>,
    outputs: Vec<PortDef>,
    /// Map external port name → (inner_node, inner_port)
    input_map: HashMap<String, (String, String)>,
    /// Map external port name → (inner_node, inner_port)
    output_map: HashMap<String, (String, String)>,
}

impl GraphNode {
    /// Create a GraphNode from a graph definition.
    ///
    /// Introspects the registry to discover port schemas, then identifies
    /// free ports (unconnected inputs/outputs) as the GraphNode's interface.
    pub fn from_definition(
        name: &str,
        definition: GraphDefinition,
        registry: Arc<NodeRegistry>,
    ) -> Result<Self, String> {
        if definition.nodes.is_empty() {
            return Err("cannot create GraphNode from empty definition".into());
        }

        // Collect all edges as sets for fast lookup
        let connected_inputs: HashSet<(String, String)> = definition
            .edges
            .iter()
            .map(|e| (e.to_node.clone(), e.to_port.clone()))
            .collect();

        let connected_outputs: HashSet<(String, String)> = definition
            .edges
            .iter()
            .map(|e| (e.from_node.clone(), e.from_port.clone()))
            .collect();

        let mut input_map = HashMap::new();
        let mut output_map = HashMap::new();
        // Un port libre est republié **entier** sous son nom externe : son
        // genre (Open, Finite) et sa posture traversent la frontière, sinon un
        // flux ouvert enveloppé échapperait au refus Complete ← Open.
        let mut inputs: Vec<PortDef> = Vec::new();
        let mut outputs: Vec<PortDef> = Vec::new();
        let mut inner_ports = Vec::new();

        for node_def in &definition.nodes {
            let schema = registry.schema_for(&node_def.node_type, &node_def.config).ok_or_else(|| {
                format!(
                    "unknown node type '{}' for inner node '{}'",
                    node_def.node_type, node_def.name
                )
            })?;

            // Free inputs: input ports without incoming edge
            for port in &schema.inputs {
                if !connected_inputs.contains(&(node_def.name.clone(), port.name.to_string())) {
                    let ext_name = format!("{}.{}", node_def.name, port.name);
                    input_map.insert(
                        ext_name.clone(),
                        (node_def.name.clone(), port.name.to_string()),
                    );
                    inputs.push(PortDef { name: ext_name.into(), ..port.clone() });
                }
            }

            // Free outputs: output ports without outgoing edge
            for port in &schema.outputs {
                if !connected_outputs.contains(&(node_def.name.clone(), port.name.to_string())) {
                    let ext_name = format!("{}.{}", node_def.name, port.name);
                    output_map.insert(
                        ext_name.clone(),
                        (node_def.name.clone(), port.name.to_string()),
                    );
                    outputs.push(PortDef { name: ext_name.into(), ..port.clone() });
                }
            }
            inner_ports.push(super::kinds::NodePorts {
                name: node_def.name.clone(),
                inputs: schema.inputs,
                outputs: schema.outputs,
            });
        }

        // **Un seul lecteur intérieur par entrée libre, un seul émetteur par
        // sortie libre** (relecture du proto) : le pont vivant leur délègue
        // les canaux extérieurs, dont les comptes supposent un bout de chaque
        // côté. Vrai par construction aujourd'hui ; dit au montage le jour où
        // un alias nommerait deux ports intérieurs.
        for (map, what) in [(&input_map, "entrée"), (&output_map, "sortie")] {
            let mut seen = HashSet::new();
            for (ext, inner) in map.iter() {
                if !seen.insert(inner.clone()) {
                    return Err(format!(
                        "sous-graphe « {name} » : l'{what} libre « {ext} » vise {}.{}, déjà visé par une autre",
                        inner.0, inner.1
                    ));
                }
            }
        }

        // **Le genre d'une sortie libre, résolu à l'intérieur** : fixe, ou
        // suivant (de proche en proche) une entrée libre, republiée sous son
        // nom externe.
        let inner_edges: Vec<super::kinds::EdgeRef> = definition
            .edges
            .iter()
            .map(|e| super::kinds::EdgeRef {
                from_node: e.from_node.clone(),
                from_port: e.from_port.clone(),
                to_node: e.to_node.clone(),
                to_port: e.to_port.clone(),
            })
            .collect();
        let resolved = super::kinds::resolve(&inner_ports, &inner_edges)?;
        super::kinds::refuse_complete_on_open(&inner_ports, &inner_edges, |n, p| {
            resolved.get(&(n.to_string(), p.to_string())).map(|r| r.with_value_inputs())
        })?;
        for out in &mut outputs {
            let inner = &output_map[out.name.as_ref()];
            match resolved.get(inner) {
                Some(super::kinds::Resolved::Fixed(k)) => {
                    out.kind = *k;
                    out.follows = None;
                }
                Some(super::kinds::Resolved::Follows { node, port, at_least }) => {
                    out.kind = *at_least;
                    out.follows = Some(format!("{node}.{port}").into());
                }
                None => {}
            }
        }

        Ok(Self {
            name: name.to_string(),
            node_type: "GraphNode".into(),
            config: None,
            definition,
            registry,
            inputs,
            outputs,
            input_map,
            output_map,
        })
    }

    /// Rename an exposed input port for simpler external connections.
    pub fn alias_input(&mut self, alias: &str, inner: &str) -> Result<(), String> {
        let (node, port) = self
            .input_map
            .remove(inner)
            .ok_or_else(|| format!("no exposed input '{inner}'"))?;
        self.input_map
            .insert(alias.to_string(), (node, port.clone()));

        // Update the PortDef
        if let Some(pd) = self.inputs.iter_mut().find(|p| p.name == inner) {
            pd.name = alias.to_string().into();
        }
        // Une sortie qui suivait cette entrée la suit sous son nouveau nom.
        for out in &mut self.outputs {
            if out.follows.as_deref() == Some(inner) {
                out.follows = Some(alias.to_string().into());
            }
        }
        Ok(())
    }

    /// Rename an exposed output port for simpler external connections.
    pub fn alias_output(&mut self, alias: &str, inner: &str) -> Result<(), String> {
        let (node, port) = self
            .output_map
            .remove(inner)
            .ok_or_else(|| format!("no exposed output '{inner}'"))?;
        self.output_map
            .insert(alias.to_string(), (node, port.clone()));

        // Update the PortDef
        if let Some(pd) = self.outputs.iter_mut().find(|p| p.name == inner) {
            pd.name = alias.to_string().into();
        }
        Ok(())
    }
}


impl GraphNode {
    /// Le sous-graphe sur les canaux délégués du parent : une entrée lue
    /// élément par élément est lue par son nœud intérieur sur l'arête
    /// extérieure, une sortie branchée est posée par son nœud intérieur sur
    /// l'arête extérieure. Les valeurs reçues en entier passent comme valeurs
    /// initiales, comme avant.
    fn execute_delegated(
        &self,
        ctx: &mut NodeContext,
        mut sub_graph: DataflowGraph,
        delegation: super::stream::Delegation,
    ) -> Result<(), String> {
        let super::stream::Delegation { hub, worker, worker_name, mut inputs, mut pending, outputs, input_kinds } = delegation;
        let mut outer_inputs = HashMap::new();
        let mut free_kinds = HashMap::new();
        for (ext_name, (inner_node, inner_port)) in &self.input_map {
            let key = (inner_node.clone(), inner_port.clone());
            if let Some(chans) = inputs.remove(ext_name) {
                let queue = pending.remove(ext_name).unwrap_or_default();
                free_kinds.insert(key.clone(), input_kinds.get(ext_name).copied().unwrap_or_default());
                // L'entrée déléguée compte comme servie pour la validation.
                sub_graph.set_initial_stream(inner_node, inner_port, Vec::new());
                outer_inputs.insert(key, (chans, queue));
            } else if let Some(queue) = pending.remove(ext_name) {
                // Une entrée lue élément par élément sans arête : sa valeur initiale.
                sub_graph.set_initial_stream(inner_node, inner_port, queue.into_iter().collect());
            } else if let Some(value) = ctx.take_input(ext_name) {
                sub_graph.set_initial_input(inner_node, inner_port, value);
            }
        }
        let mut outer_outputs = HashMap::new();
        for (ext_name, (inner_node, inner_port)) in &self.output_map {
            if let Some(chans) = outputs.get(ext_name) {
                outer_outputs.insert((inner_node.clone(), inner_port.clone()), chans.clone());
            }
        }
        let services = if ctx.run_id().is_empty() {
            ctx.services_arc()
        } else {
            let mut layer = super::services::ServiceRegistry::layered(ctx.services_arc());
            layer.register("parent_run", ctx.run_id().to_string());
            std::sync::Arc::new(layer)
        };
        let max_iterations = sub_graph.nodes.len().max(1);
        let inner = super::runtime::DataflowRuntime::with_services_arc(max_iterations, services);
        let outer = super::concurrent::Outer { hub, leave: worker, prefix: worker_name, inputs: outer_inputs, outputs: outer_outputs };
        let output = inner.execute_delegated(&mut sub_graph, outer, free_kinds)?;

        // Ce que les nœuds intérieurs ont posé sur les arêtes du parent compte
        // pour les sorties de ce nœud ; une sortie libre en flux non branchée a
        // jeté ses éléments, leur compte dit qu'elle a été posée.
        for (ext_name, (inner_node, inner_port)) in &self.output_map {
            if outputs.contains_key(ext_name) {
                ctx.note_sent(ext_name, output.sent(inner_node, inner_port));
            } else if ctx.is_stream_output(ext_name) {
                ctx.note_dropped(ext_name, output.dropped(inner_node, inner_port));
            } else if let Some(value) = output.get(inner_node, inner_port) {
                ctx.set_output(ext_name, value.clone());
            }
        }
        Ok(())
    }
}

impl Node for GraphNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn inputs(&self) -> Vec<PortDef> {
        self.inputs.clone()
    }

    fn outputs(&self) -> Vec<PortDef> {
        self.outputs.clone()
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        // 1. Materialize the sub-graph
        let mut sub_graph = DataflowGraph::from_definition(&self.definition, &self.registry)?;

        // **Le pont vivant** (page 05, §4.1) : sous l'exécuteur concurrent, le
        // sous-graphe s'exécute sur les canaux du parent, ses éléments coulent
        // à travers la frontière.
        if let Some(delegation) = ctx.delegate_streams() {
            return self.execute_delegated(ctx, sub_graph, delegation);
        }

        // 2. Inject inputs from parent context into sub-graph initial_inputs.
        // **Hors du pont vivant** (un parent sous l'exécuteur par niveaux, qui
        // n'a pas de flux ouvert) : une entrée lue élément par élément reçoit
        // ses éléments en entier, puis les pose d'avance dans le sous-graphe.
        let per_element: std::collections::HashSet<String> = self
            .inputs
            .iter()
            .filter(|p| p.posture == super::port::Posture::PerElement)
            .map(|p| p.name.to_string())
            .collect();
        for (ext_name, (inner_node, inner_port)) in &self.input_map {
            if per_element.contains(ext_name) {
                let mut elements = Vec::new();
                while let Some(v) = ctx.next(ext_name)? {
                    elements.push(v);
                }
                if !elements.is_empty() {
                    sub_graph.set_initial_stream(inner_node, inner_port, elements);
                }
            } else if let Some(value) = ctx.take_input(ext_name) {
                sub_graph.set_initial_input(inner_node, inner_port, value);
            }
        }

        // 3. Execute the sub-graph on our runtime — under the parent run,
        //    so its nodes trace as children of this one.
        let services = if ctx.run_id().is_empty() {
            ctx.services_arc()
        } else {
            let mut layer = super::services::ServiceRegistry::layered(ctx.services_arc());
            layer.register("parent_run", ctx.run_id().to_string());
            std::sync::Arc::new(layer)
        };
        let max_iterations = sub_graph.nodes.len().max(1);
        let inner = super::runtime::DataflowRuntime::with_services_arc(max_iterations, services);
        // L'annulation du parent arrête le sous-graphe à flux.
        let stop = inner.cancel_handle();
        ctx.cancellation().on_cancel(move || stop.cancel());
        let output = inner.execute(&mut sub_graph)?;

        // 5. Collect free outputs and set them on the parent context. Une
        // sortie en flux rend sa valeur intérieure élément par élément ; ses
        // éléments jetés (sans arête dedans) comptent pour elle.
        for (ext_name, (inner_node, inner_port)) in &self.output_map {
            if ctx.is_stream_output(ext_name) {
                ctx.note_dropped(ext_name, output.dropped(inner_node, inner_port));
                if let Some(v) = output.get(inner_node, inner_port) {
                    match ctx.emit(ext_name, v.clone()) {
                        Ok(()) | Err(super::node::Stopped::NoReader) => {}
                        Err(e) => return Err(e.into()),
                    }
                }
            } else if let Some(value) = output.get(inner_node, inner_port) {
                ctx.set_output(ext_name, value.clone());
            }
        }

        Ok(())
    }

    fn node_type(&self) -> &str {
        &self.node_type
    }

    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        if let Some(config) = &self.config {
            return Some(Box::new(config.clone()));
        }
        let val = serde_json::to_value(&self.definition).unwrap_or_default();
        Some(Box::new(val))
    }
}

// ─── GraphNodeFactory ───────────────────────────────────────────────────────

/// Factory for creating GraphNode instances from a stored GraphDefinition.
///
/// Registered in the NodeRegistry under a custom name (e.g., "SearchPipeline").
pub struct GraphNodeFactory {
    definition: GraphDefinition,
    registry: Arc<NodeRegistry>,
    /// Cached schema (computed once at construction)
    schema: NodeSchema,
}

impl GraphNodeFactory {
    /// Create a factory for a named sub-graph template, with no config params.
    pub fn new(
        type_name: &str,
        description: &str,
        definition: GraphDefinition,
        registry: Arc<NodeRegistry>,
    ) -> Result<Self, String> {
        Self::templated(type_name, description, definition, vec![], registry)
    }

    /// Une fabrique de sous-graphe **paramétrée**.
    ///
    /// `config_params: vec![]` était le trou : un sous-graphe enregistré
    /// n'avait aucune façon de recevoir quoi que ce soit, donc aucune façon
    /// d'être réutilisé autrement qu'à l'identique. Ici les paramètres sont
    /// publiés dans le [`NodeSchema`] — donc visibles à l'introspection comme
    /// n'importe quel nœud — et la configuration reçue est **substituée dans
    /// le gabarit** (les `$param`) avant que le sous-graphe soit matérialisé.
    ///
    /// C'est ce qui rend un [`crate::dataflow::GraphTool`] *contenable* : un
    /// graphe-outil devient un type de nœud que d'autres graphes-outils
    /// utilisent, avec sa propre fiche pour configuration.
    pub fn templated(
        type_name: &str,
        description: &str,
        definition: GraphDefinition,
        config_params: Vec<crate::dataflow::node_registry::ConfigParam>,
        registry: Arc<NodeRegistry>,
    ) -> Result<Self, String> {
        // Build a temporary GraphNode to extract port schemas. Les `$param`
        // qui traînent dans les configurations ne gênent pas : la sonde ne lit
        // que les schémas de types, jamais les valeurs.
        let temp = GraphNode::from_definition("__schema_probe", definition.clone(), registry.clone())?;

        let schema = NodeSchema {
            node_type: type_name.to_string().into(),
            description: description.to_string().into(),
            inputs: temp.inputs.clone(),
            outputs: temp.outputs.clone(),
            config_params,
        };

        Ok(Self {
            definition,
            registry,
            schema,
        })
    }
}

impl NodeFactory for GraphNodeFactory {
    fn create(&self, name: &str, config: &serde_json::Value) -> Result<Box<dyn Node>, String> {
        let definition = if self.schema.config_params.is_empty() {
            self.definition.clone()
        } else {
            // Mêmes règles que pour un appel d'outil : argument manquant, en
            // trop ou du mauvais type est une erreur, jamais un sous-graphe
            // lancé à moitié.
            let args = super::graph_tool::resolve_params(&self.schema.config_params, config)
                .map_err(|e| format!("{}: {e}", self.schema.node_type))?;
            // Les listes closes se vérifient ici aussi ; celles du catalogue
            // l'ont été à la fiche, où le catalogue est disponible.
            super::graph_tool::check_choices(&self.schema.config_params, &args, None)
                .map_err(|e| format!("{}: {e}", self.schema.node_type))?;
            super::graph_tool::substitute_definition(&self.definition, &args)
        };
        let mut node = GraphNode::from_definition(name, definition, self.registry.clone())?;
        node.node_type = self.schema.node_type.to_string();
        node.config = Some(config.clone());
        Ok(Box::new(node))
    }

    fn node_type(&self) -> &str {
        &self.schema.node_type
    }

    fn schema(&self) -> NodeSchema {
        self.schema.clone()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::checkpoint::{EdgeDef, NodeDef};
    use crate::dataflow::node_factories::register_builtins;

    fn test_registry() -> Arc<NodeRegistry> {
        let mut registry = NodeRegistry::new();
        register_builtins(&mut registry);
        Arc::new(registry)
    }

    /// Build a simple 2-node definition: InsertRecordNode → LinkRecordNode
    fn ingestion_subgraph_def() -> GraphDefinition {
        GraphDefinition {
            nodes: vec![
                NodeDef {
                    name: "inserts".into(),
                    node_type: "InsertRecordNode".into(),
                    config: serde_json::json!({}),
                },
                NodeDef {
                    name: "links".into(),
                    node_type: "LinkRecordNode".into(),
                    config: serde_json::json!({}),
                },
            ],
            edges: vec![EdgeDef {
                from_node: "inserts".into(),
                from_port: "done".into(),
                to_node: "links".into(),
                to_port: "trigger".into(),
            }],
        }
    }

    /// Build a simple search subgraph: KBQuerySourceNode → SearchSourceNode
    fn search_subgraph_def() -> GraphDefinition {
        GraphDefinition {
            nodes: vec![
                NodeDef {
                    name: "qs".into(),
                    node_type: "KBQuerySourceNode".into(),
                    config: serde_json::json!({"kb_name": "TestKB", "query": "hello"}),
                },
                NodeDef {
                    name: "ps".into(),
                    node_type: "SearchSourceNode".into(),
                    config: serde_json::json!({"target": "TestKB", "query": "hello"}),
                },
            ],
            edges: vec![EdgeDef {
                from_node: "qs".into(),
                from_port: "query".into(),
                to_node: "ps".into(),
                to_port: "query".into(),
            }],
        }
    }

    // ── Test 1: ports detected correctly ─────────────────────────────

    #[test]
    fn graph_node_detects_free_ports() {
        let registry = test_registry();
        let def = ingestion_subgraph_def();
        let gn = GraphNode::from_definition("test_gn", def, registry).unwrap();

        // InsertRecordNode inputs: entities (required), trigger (optional)
        //   - trigger is free (no incoming edge)
        //   - entities is free (no incoming edge)
        // LinkRecordNode inputs: relations (required), trigger (optional)
        //   - trigger is connected (edge from inserts.done)
        //   - relations is free
        let input_names: Vec<&str> = gn.inputs.iter().map(|p| &*p.name).collect();
        assert!(input_names.contains(&"inserts.entities"), "inputs: {:?}", input_names);
        assert!(input_names.contains(&"inserts.trigger"), "inputs: {:?}", input_names);
        assert!(input_names.contains(&"links.relations"), "inputs: {:?}", input_names);
        // links.trigger should NOT be free (connected via edge)
        assert!(!input_names.contains(&"links.trigger"), "inputs: {:?}", input_names);

        // Outputs: InsertRecordNode has inserted (free), LinkRecordNode has done (free)
        let output_names: Vec<&str> = gn.outputs.iter().map(|p| &*p.name).collect();
        assert!(output_names.contains(&"inserts.inserted"), "outputs: {:?}", output_names);
        assert!(output_names.contains(&"links.done"), "outputs: {:?}", output_names);
    }

    // ── Test 2: search subgraph ports ────────────────────────────────

    #[test]
    fn graph_node_search_subgraph_ports() {
        let registry = test_registry();
        let def = search_subgraph_def();
        let gn = GraphNode::from_definition("search", def, registry).unwrap();

        // KBQuerySourceNode n'a pas d'entrée ; l'entrée `query` de
        // `SearchSourceNode` (optionnelle depuis le pas B) est câblée par
        // l'arête → pas libre, donc pas exposée.
        let input_names: Vec<&str> = gn.inputs.iter().map(|p| &*p.name).collect();
        assert!(!input_names.contains(&"ps.query"), "inputs: {:?}", input_names);

        // Sorties libres : la sortie `query` de l'amont est câblée → absorbée ;
        // `query` et `meta` de la source restent libres.
        let output_names: Vec<&str> = gn.outputs.iter().map(|p| &*p.name).collect();
        assert!(output_names.contains(&"ps.query"), "outputs: {:?}", output_names);
        assert!(output_names.contains(&"ps.meta"), "outputs: {:?}", output_names);
    }

    /// **Le genre et la posture traversent la frontière d'un sous-graphe** :
    /// un flux ouvert enveloppé reste ouvert, et le refus Complete ← Open
    /// s'applique encore au graphe qui l'emploie.
    #[test]
    fn graph_node_keeps_kind_and_posture_of_free_ports() {
        use crate::dataflow::port::{PortKind, PortType, Posture};
        struct StreamFactory;
        impl NodeFactory for StreamFactory {
            fn create(&self, _: &str, _: &serde_json::Value) -> Result<Box<dyn Node>, String> {
                Err("le schéma suffit à ce témoin".into())
            }
            fn node_type(&self) -> &str {
                "Stream"
            }
            fn schema(&self) -> NodeSchema {
                NodeSchema {
                    node_type: "Stream".into(),
                    description: "".into(),
                    inputs: vec![PortDef::value("in", PortType::Results, true).posture(Posture::PerElement)],
                    outputs: vec![PortDef::value("out", PortType::Results, false).stream(PortKind::Open)],
                    config_params: vec![],
                }
            }
        }
        let mut registry = NodeRegistry::new();
        registry.register(Box::new(StreamFactory));
        let def = GraphDefinition {
            nodes: vec![NodeDef { name: "s".into(), node_type: "Stream".into(), config: serde_json::json!({}) }],
            edges: vec![],
        };
        let gn = GraphNode::from_definition("wrapped", def, Arc::new(registry)).unwrap();
        let out = gn.outputs.iter().find(|p| p.name == "s.out").expect("s.out libre");
        assert_eq!((out.kind, out.required), (PortKind::Open, false));
        let inp = gn.inputs.iter().find(|p| p.name == "s.in").expect("s.in libre");
        assert_eq!((inp.posture, inp.required), (Posture::PerElement, true));
    }

    /// **Une sortie qui suit traverse la frontière résolue** : derrière une
    /// source ouverte intérieure, elle est `Open` ; sur une entrée libre, elle
    /// suit l'entrée externe (et son alias) ; un lecteur du tout derrière une
    /// source ouverte est refusé dès le sous-graphe.
    #[test]
    fn graph_node_resolves_following_outputs() {
        use crate::dataflow::port::{PortKind, PortType, Posture};
        struct Fixed(&'static str, Vec<PortDef>, Vec<PortDef>);
        impl NodeFactory for Fixed {
            fn create(&self, _: &str, _: &serde_json::Value) -> Result<Box<dyn Node>, String> {
                Err("le schéma suffit à ce témoin".into())
            }
            fn node_type(&self) -> &str {
                self.0
            }
            fn schema(&self) -> NodeSchema {
                NodeSchema {
                    node_type: self.0.into(),
                    description: "".into(),
                    inputs: self.1.clone(),
                    outputs: self.2.clone(),
                    config_params: vec![],
                }
            }
        }
        let mut registry = NodeRegistry::new();
        registry.register(Box::new(Fixed("Open", vec![], vec![PortDef::value("out", PortType::Results, false).stream(PortKind::Open)])));
        registry.register(Box::new(Fixed(
            "Follow",
            vec![PortDef::value("element", PortType::Results, true).posture(Posture::PerElement)],
            vec![PortDef::value("out", PortType::Results, false).follows("element")],
        )));
        registry.register(Box::new(Fixed("Whole", vec![PortDef::value("in", PortType::Results, true)], vec![])));
        let registry = Arc::new(registry);
        let node = |name: &str, ty: &str| NodeDef { name: name.into(), node_type: ty.into(), config: serde_json::json!({}) };
        let edge = |f: &str, fp: &str, t: &str, tp: &str| EdgeDef {
            from_node: f.into(),
            from_port: fp.into(),
            to_node: t.into(),
            to_port: tp.into(),
        };

        let alone = GraphDefinition { nodes: vec![node("f", "Follow")], edges: vec![] };
        let mut gn = GraphNode::from_definition("g", alone, registry.clone()).unwrap();
        let out = gn.outputs.iter().find(|p| p.name == "f.out").unwrap();
        assert_eq!((out.kind, out.follows.as_deref()), (PortKind::Value, Some("f.element")));
        gn.alias_input("element", "f.element").unwrap();
        assert_eq!(gn.outputs[0].follows.as_deref(), Some("element"), "l'alias suit");

        let behind_open = GraphDefinition {
            nodes: vec![node("src", "Open"), node("f", "Follow")],
            edges: vec![edge("src", "out", "f", "element")],
        };
        let gn = GraphNode::from_definition("g", behind_open, registry.clone()).unwrap();
        let out = gn.outputs.iter().find(|p| p.name == "f.out").unwrap();
        assert_eq!((out.kind, out.follows.as_deref()), (PortKind::Open, None));

        let whole = GraphDefinition {
            nodes: vec![node("src", "Open"), node("f", "Follow"), node("w", "Whole")],
            edges: vec![edge("src", "out", "f", "element"), edge("f", "out", "w", "in")],
        };
        let err = GraphNode::from_definition("g", whole, registry).err().expect("refusé");
        assert!(err.contains("f.out") && err.contains("w.in"), "{err}");
    }

    // ── Test 3: empty definition errors ──────────────────────────────

    #[test]
    fn graph_node_empty_definition_errors() {
        let registry = test_registry();
        let def = GraphDefinition {
            nodes: vec![],
            edges: vec![],
        };
        let err = GraphNode::from_definition("empty", def, registry).unwrap_err();
        assert!(err.contains("empty"), "error: {err}");
    }

    // ── Test 4: unknown node type errors ─────────────────────────────

    #[test]
    fn graph_node_unknown_type_errors() {
        let registry = test_registry();
        let def = GraphDefinition {
            nodes: vec![NodeDef {
                name: "x".into(),
                node_type: "BogusNode".into(),
                config: serde_json::json!({}),
            }],
            edges: vec![],
        };
        let err = GraphNode::from_definition("bad", def, registry).unwrap_err();
        assert!(err.contains("unknown node type"), "error: {err}");
    }

    // ── Test 5: alias input ──────────────────────────────────────────

    #[test]
    fn graph_node_alias_input() {
        let registry = test_registry();
        let def = ingestion_subgraph_def();
        let mut gn = GraphNode::from_definition("test", def, registry).unwrap();

        gn.alias_input("entities", "inserts.entities").unwrap();

        let input_names: Vec<&str> = gn.inputs.iter().map(|p| &*p.name).collect();
        assert!(input_names.contains(&"entities"), "inputs: {:?}", input_names);
        assert!(!input_names.contains(&"inserts.entities"), "inputs: {:?}", input_names);

        // Map still works
        let (node, port) = gn.input_map.get("entities").unwrap();
        assert_eq!(node, "inserts");
        assert_eq!(port, "entities");
    }

    // ── Test 6: alias output ─────────────────────────────────────────

    #[test]
    fn graph_node_alias_output() {
        let registry = test_registry();
        let def = search_subgraph_def();
        let mut gn = GraphNode::from_definition("test", def, registry).unwrap();

        gn.alias_output("payload", "ps.query").unwrap();

        let output_names: Vec<&str> = gn.outputs.iter().map(|p| &*p.name).collect();
        assert!(output_names.contains(&"payload"), "outputs: {:?}", output_names);
        assert!(!output_names.contains(&"ps.query"), "outputs: {:?}", output_names);
    }

    // ── Test 7: alias nonexistent port errors ────────────────────────

    #[test]
    fn graph_node_alias_nonexistent_errors() {
        let registry = test_registry();
        let def = ingestion_subgraph_def();
        let mut gn = GraphNode::from_definition("test", def, registry).unwrap();

        assert!(gn.alias_input("x", "bogus.port").is_err());
        assert!(gn.alias_output("x", "bogus.port").is_err());
    }

    // ── Test 8: node_type and node_config ────────────────────────────

    #[test]
    fn graph_node_type_and_config() {
        let registry = test_registry();
        let def = ingestion_subgraph_def();
        let gn = GraphNode::from_definition("test", def.clone(), registry).unwrap();

        assert_eq!(gn.node_type(), "GraphNode");
        let config = *gn.node_config()
            .and_then(|b| b.downcast::<serde_json::Value>().ok())
            .expect("expected serde_json::Value config");
        // Config should be the serialized definition
        let restored: GraphDefinition = serde_json::from_value(config).unwrap();
        assert_eq!(restored.nodes.len(), def.nodes.len());
        assert_eq!(restored.edges.len(), def.edges.len());
    }

    // ── Test 9: GraphNodeFactory ─────────────────────────────────────

    #[test]
    fn graph_node_factory_create() {
        let registry = test_registry();
        let def = search_subgraph_def();

        let factory = GraphNodeFactory::new(
            "SearchPipeline",
            "Reusable search sub-graph",
            def,
            registry,
        )
        .unwrap();

        assert_eq!(factory.node_type(), "SearchPipeline");

        let schema = factory.schema();
        assert_eq!(schema.node_type, "SearchPipeline");
        assert!(!schema.outputs.is_empty());

        let node = factory.create("my_search", &serde_json::json!({})).unwrap();
        assert_eq!(node.name(), "my_search");
        assert_eq!(node.node_type(), "SearchPipeline");
    }

    // ── Test 10: GraphNode in parent graph ───────────────────────────

    #[test]
    fn graph_node_in_parent_graph() {
        let registry = test_registry();
        let def = search_subgraph_def();
        let gn = GraphNode::from_definition("search_sub", def, registry).unwrap();

        // Should be usable as a regular node in a DataflowGraph
        let mut parent = DataflowGraph::new();
        parent.add_node(Box::new(gn)).unwrap();
        assert_eq!(parent.node_names(), vec!["search_sub"]);

        // Validate passes (no required unconnected inputs from search subgraph)
        parent.validate().unwrap();
    }

    // ── Test 11: round-trip Mermaid → GraphNode ──────────────────────

    #[test]
    fn graph_node_from_mermaid() {
        use crate::dataflow::mermaid::parse_mermaid;

        let mermaid = r#"graph LR
    qs["KBQuerySourceNode(kb_name='TestKB', query='hello')"]
    ps["SearchSourceNode(target='TestKB', query='hello')"]
    qs -->|query| ps
"#;
        let def = parse_mermaid(mermaid).unwrap();
        let registry = test_registry();
        let gn = GraphNode::from_definition("mermaid_search", def, registry).unwrap();

        let output_names: Vec<&str> = gn.outputs.iter().map(|p| &*p.name).collect();
        assert!(output_names.contains(&"ps.query"));
        assert!(output_names.contains(&"ps.meta"));
    }

    /// Un sous-graphe déclaré sous un nom garde ce nom à travers
    /// `to_definition` : c'est par lui qu'un point de reprise le retrouve.
    #[test]
    fn a_declared_subgraph_keeps_its_type_through_a_definition_round_trip() {
        let inner = test_registry();
        let mut registry = NodeRegistry::new();
        register_builtins(&mut registry);
        registry.register(Box::new(
            GraphNodeFactory::new(
                "Ingest",
                "declared ingestion",
                ingestion_subgraph_def(),
                inner,
            )
            .unwrap(),
        ));
        let outer = GraphDefinition {
            nodes: vec![NodeDef {
                name: "sub".into(),
                node_type: "Ingest".into(),
                config: serde_json::json!({}),
            }],
            edges: vec![],
        };
        let graph =
            crate::dataflow::graph::DataflowGraph::from_definition(&outer, &registry).unwrap();
        let back = graph.to_definition();
        assert_eq!(back.nodes[0].node_type, "Ingest");
        crate::dataflow::graph::DataflowGraph::from_definition(&back, &registry).unwrap();
    }
}

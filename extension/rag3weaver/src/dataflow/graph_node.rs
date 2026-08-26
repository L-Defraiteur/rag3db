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
use super::port::{PortDef, PortType};

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
        let mut input_defs: Vec<(String, PortType, bool)> = Vec::new();
        let mut output_defs: Vec<(String, PortType, bool)> = Vec::new();

        for node_def in &definition.nodes {
            let schema = registry.schema(&node_def.node_type).ok_or_else(|| {
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
                    input_defs.push((ext_name, port.port_type, port.required));
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
                    output_defs.push((ext_name, port.port_type, port.required));
                }
            }
        }

        // Build PortDefs — we use leaked &'static str for the names
        // since PortDef requires &'static str and these are dynamic.
        let inputs: Vec<PortDef> = input_defs
            .iter()
            .map(|(name, pt, req)| {
                let leaked: &'static str = Box::leak(name.clone().into_boxed_str());
                PortDef {
                    name: leaked,
                    port_type: *pt,
                    required: *req,
                }
            })
            .collect();

        let outputs: Vec<PortDef> = output_defs
            .iter()
            .map(|(name, pt, req)| {
                let leaked: &'static str = Box::leak(name.clone().into_boxed_str());
                PortDef {
                    name: leaked,
                    port_type: *pt,
                    required: *req,
                }
            })
            .collect();

        Ok(Self {
            name: name.to_string(),
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
            let leaked: &'static str = Box::leak(alias.to_string().into_boxed_str());
            pd.name = leaked;
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
            let leaked: &'static str = Box::leak(alias.to_string().into_boxed_str());
            pd.name = leaked;
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

        // 2. Inject inputs from parent context into sub-graph initial_inputs
        for (ext_name, (inner_node, inner_port)) in &self.input_map {
            if let Some(value) = ctx.take_input(ext_name) {
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
        let output = super::runtime::DataflowRuntime::with_services_arc(max_iterations, services)
            .execute(&mut sub_graph)?;

        // 5. Collect free outputs and set them on the parent context
        for (ext_name, (inner_node, inner_port)) in &self.output_map {
            if let Some(value) = output.get(inner_node, inner_port) {
                ctx.set_output(ext_name, value.clone());
            }
        }

        Ok(())
    }

    fn node_type(&self) -> &'static str {
        "GraphNode"
    }

    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
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

        // Leak type_name for &'static str
        let leaked_type: &'static str = Box::leak(type_name.to_string().into_boxed_str());
        let leaked_desc: &'static str = Box::leak(description.to_string().into_boxed_str());

        let schema = NodeSchema {
            node_type: leaked_type,
            description: leaked_desc,
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
        let node = GraphNode::from_definition(name, definition, self.registry.clone())?;
        Ok(Box::new(node))
    }

    fn node_type(&self) -> &'static str {
        self.schema.node_type
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

    /// Build a simple search subgraph: KBQuerySourceNode → KBSearchNode
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
                    node_type: "KBSearchNode".into(),
                    config: serde_json::json!({}),
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
        let input_names: Vec<&str> = gn.inputs.iter().map(|p| p.name).collect();
        assert!(input_names.contains(&"inserts.entities"), "inputs: {:?}", input_names);
        assert!(input_names.contains(&"inserts.trigger"), "inputs: {:?}", input_names);
        assert!(input_names.contains(&"links.relations"), "inputs: {:?}", input_names);
        // links.trigger should NOT be free (connected via edge)
        assert!(!input_names.contains(&"links.trigger"), "inputs: {:?}", input_names);

        // Outputs: InsertRecordNode has inserted (free), LinkRecordNode has done (free)
        let output_names: Vec<&str> = gn.outputs.iter().map(|p| p.name).collect();
        assert!(output_names.contains(&"inserts.inserted"), "outputs: {:?}", output_names);
        assert!(output_names.contains(&"links.done"), "outputs: {:?}", output_names);
    }

    // ── Test 2: search subgraph ports ────────────────────────────────

    #[test]
    fn graph_node_search_subgraph_ports() {
        let registry = test_registry();
        let def = search_subgraph_def();
        let gn = GraphNode::from_definition("search", def, registry).unwrap();

        // KBQuerySourceNode has no inputs → no free inputs from it
        // KBSearchNode has query (connected) → no free input from it
        // So the only free inputs could be optional ones
        let input_names: Vec<&str> = gn.inputs.iter().map(|p| p.name).collect();
        // KBQuerySourceNode has no inputs at all
        // KBSearchNode's query input is connected via edge
        assert!(!input_names.contains(&"ps.query"), "inputs: {:?}", input_names);

        // Free outputs: KBQuerySourceNode's query output is connected → not free
        // KBSearchNode's results and meta are free
        let output_names: Vec<&str> = gn.outputs.iter().map(|p| p.name).collect();
        assert!(output_names.contains(&"ps.results"), "outputs: {:?}", output_names);
        assert!(output_names.contains(&"ps.meta"), "outputs: {:?}", output_names);
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

        let input_names: Vec<&str> = gn.inputs.iter().map(|p| p.name).collect();
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

        gn.alias_output("results", "ps.results").unwrap();

        let output_names: Vec<&str> = gn.outputs.iter().map(|p| p.name).collect();
        assert!(output_names.contains(&"results"), "outputs: {:?}", output_names);
        assert!(!output_names.contains(&"ps.results"), "outputs: {:?}", output_names);
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
        assert_eq!(node.node_type(), "GraphNode");
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
    ps["KBSearchNode"]
    qs -->|query| ps
"#;
        let def = parse_mermaid(mermaid).unwrap();
        let registry = test_registry();
        let gn = GraphNode::from_definition("mermaid_search", def, registry).unwrap();

        let output_names: Vec<&str> = gn.outputs.iter().map(|p| p.name).collect();
        assert!(output_names.contains(&"ps.results"));
        assert!(output_names.contains(&"ps.meta"));
    }
}

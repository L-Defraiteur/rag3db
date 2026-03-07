//! Dataflow graph structure: nodes, edges, validation, topological sort.

use std::collections::{HashMap, HashSet, VecDeque};

use super::node::{DynamicNode, Node};
use super::port::{PortType, PortValue};

// ─── Edge ────────────────────────────────────────────────────────────────────

/// A directed edge connecting an output port to an input port.
#[derive(Debug, Clone)]
pub struct Edge {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
}

// ─── NodeSlot ────────────────────────────────────────────────────────────────

/// Internal: a node is either static or dynamic.
pub(crate) enum NodeSlot {
    Static(Box<dyn Node>),
    Dynamic(Box<dyn DynamicNode>),
}

impl NodeSlot {
    pub fn name(&self) -> &str {
        match self {
            Self::Static(n) => n.name(),
            Self::Dynamic(n) => n.name(),
        }
    }

    pub fn inputs(&self) -> &[super::port::PortDef] {
        match self {
            Self::Static(n) => n.inputs(),
            Self::Dynamic(n) => n.inputs(),
        }
    }

    pub fn outputs(&self) -> &[super::port::PortDef] {
        match self {
            Self::Static(n) => n.outputs(),
            Self::Dynamic(n) => n.outputs(),
        }
    }
}

// ─── DataflowGraph ───────────────────────────────────────────────────────────

/// A directed acyclic graph of nodes connected by typed edges.
pub struct DataflowGraph {
    pub(crate) nodes: Vec<NodeSlot>,
    pub(crate) edges: Vec<Edge>,
    /// Pre-loaded inputs for nodes (consumed by runtime at execution start).
    pub(crate) initial_inputs: HashMap<String, HashMap<String, PortValue>>,
}

impl DataflowGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            initial_inputs: HashMap::new(),
        }
    }

    /// Pre-load an input port on a node with initial data.
    /// Used to feed data into entry nodes (e.g., SplitOpsNode).
    pub fn set_initial_input(&mut self, node_name: &str, port: &str, value: PortValue) {
        self.initial_inputs
            .entry(node_name.to_string())
            .or_default()
            .insert(port.to_string(), value);
    }

    /// Add a static node. Returns error if name already taken.
    pub fn add_node(&mut self, node: Box<dyn Node>) -> Result<(), String> {
        let name = node.name().to_string();
        if self.nodes.iter().any(|n| n.name() == name) {
            return Err(format!("duplicate node name: {name}"));
        }
        self.nodes.push(NodeSlot::Static(node));
        Ok(())
    }

    /// Add a dynamic node. Returns error if name already taken.
    pub fn add_dynamic_node(&mut self, node: Box<dyn DynamicNode>) -> Result<(), String> {
        let name = node.name().to_string();
        if self.nodes.iter().any(|n| n.name() == name) {
            return Err(format!("duplicate node name: {name}"));
        }
        self.nodes.push(NodeSlot::Dynamic(node));
        Ok(())
    }

    fn find_node(&self, name: &str) -> Option<&NodeSlot> {
        self.nodes.iter().find(|n| n.name() == name)
    }

    fn find_output_port_type(&self, node_name: &str, port_name: &str) -> Option<PortType> {
        let node = self.find_node(node_name)?;
        node.outputs()
            .iter()
            .find(|p| p.name == port_name)
            .map(|p| p.port_type)
    }

    fn find_input_port_type(&self, node_name: &str, port_name: &str) -> Option<PortType> {
        let node = self.find_node(node_name)?;
        node.inputs()
            .iter()
            .find(|p| p.name == port_name)
            .map(|p| p.port_type)
    }

    /// Connect an output port to an input port.
    /// Validates: both nodes exist, ports exist, types compatible.
    pub fn connect(
        &mut self,
        from_node: &str,
        from_port: &str,
        to_node: &str,
        to_port: &str,
    ) -> Result<(), String> {
        let from_type = self
            .find_output_port_type(from_node, from_port)
            .ok_or_else(|| {
                format!("output port '{from_port}' not found on node '{from_node}'")
            })?;
        let to_type = self
            .find_input_port_type(to_node, to_port)
            .ok_or_else(|| {
                format!("input port '{to_port}' not found on node '{to_node}'")
            })?;

        if !from_type.compatible_with(&to_type) {
            return Err(format!(
                "port type mismatch: {from_node}.{from_port} ({from_type:?}) → {to_node}.{to_port} ({to_type:?})"
            ));
        }

        self.edges.push(Edge {
            from_node: from_node.to_string(),
            from_port: from_port.to_string(),
            to_node: to_node.to_string(),
            to_port: to_port.to_string(),
        });
        Ok(())
    }

    /// Validate the graph: check DAG (no cycles) + all required inputs connected.
    pub fn validate(&self) -> Result<(), String> {
        // Check DAG
        self.topological_sort()?;

        // Check required inputs (edges OR initial_inputs count as connected)
        let mut warnings = Vec::new();
        for node in &self.nodes {
            for input in node.inputs() {
                if input.required {
                    let connected_by_edge = self.edges.iter().any(|e| {
                        e.to_node == node.name() && e.to_port == input.name
                    });
                    let connected_by_initial = self.initial_inputs
                        .get(node.name())
                        .map_or(false, |ports| ports.contains_key(input.name));
                    if !connected_by_edge && !connected_by_initial {
                        warnings.push(format!(
                            "required input '{}' on node '{}' is not connected",
                            input.name,
                            node.name()
                        ));
                    }
                }
            }
        }

        if warnings.is_empty() {
            Ok(())
        } else {
            Err(warnings.join("; "))
        }
    }

    /// Topological sort via Kahn's algorithm.
    /// Returns execution order. Errors on cycles.
    pub fn topological_sort(&self) -> Result<Vec<String>, String> {
        let names: Vec<String> = self.nodes.iter().map(|n| n.name().to_string()).collect();
        let name_set: HashSet<&str> = names.iter().map(|s| s.as_str()).collect();

        // Build in-degree map
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for name in &names {
            in_degree.insert(name.as_str(), 0);
        }
        for edge in &self.edges {
            if name_set.contains(edge.to_node.as_str()) {
                *in_degree.entry(edge.to_node.as_str()).or_default() += 1;
            }
        }

        // Start with nodes that have no incoming edges
        let mut queue: VecDeque<&str> = VecDeque::new();
        for name in &names {
            if in_degree[name.as_str()] == 0 {
                queue.push_back(name.as_str());
            }
        }

        let mut order = Vec::new();
        while let Some(node) = queue.pop_front() {
            order.push(node.to_string());
            // Decrease in-degree for all downstream nodes
            for edge in &self.edges {
                if edge.from_node == node && name_set.contains(edge.to_node.as_str()) {
                    let deg = in_degree.get_mut(edge.to_node.as_str()).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(
                            names
                                .iter()
                                .find(|n| *n == &edge.to_node)
                                .unwrap()
                                .as_str(),
                        );
                    }
                }
            }
        }

        if order.len() != names.len() {
            Err("cycle detected in dataflow graph".to_string())
        } else {
            Ok(order)
        }
    }

    /// Merge dynamically emitted nodes and edges into the graph.
    pub(crate) fn merge_dynamic(
        &mut self,
        nodes: Vec<Box<dyn Node>>,
        dynamic_nodes: Vec<Box<dyn DynamicNode>>,
        edges: Vec<Edge>,
    ) -> Result<(), String> {
        for node in nodes {
            self.add_node(node)?;
        }
        for node in dynamic_nodes {
            self.add_dynamic_node(node)?;
        }
        // For dynamically added edges, skip port validation (the nodes
        // were just created and the types should match by construction).
        self.edges.extend(edges);
        Ok(())
    }

    /// List all node names.
    pub fn node_names(&self) -> Vec<&str> {
        self.nodes.iter().map(|n| n.name()).collect()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::port::{PortDef, PortType, PortValue};
    use crate::dataflow::node::NodeContext;

    /// Minimal test node.
    struct TestNode {
        name: String,
        inputs: Vec<PortDef>,
        outputs: Vec<PortDef>,
    }

    impl TestNode {
        fn new(name: &str, inputs: Vec<PortDef>, outputs: Vec<PortDef>) -> Self {
            Self {
                name: name.to_string(),
                inputs,
                outputs,
            }
        }
    }

    #[async_trait::async_trait]
    impl Node for TestNode {
        fn name(&self) -> &str {
            &self.name
        }
        fn inputs(&self) -> &[PortDef] {
            &self.inputs
        }
        fn outputs(&self) -> &[PortDef] {
            &self.outputs
        }
        async fn execute(&mut self, _ctx: &mut NodeContext) -> Result<(), String> {
            Ok(())
        }
    }

    fn source_node(name: &str) -> Box<dyn Node> {
        Box::new(TestNode::new(
            name,
            vec![],
            vec![PortDef {
                name: "out",
                port_type: PortType::Results,
                required: false,
            }],
        ))
    }

    fn sink_node(name: &str) -> Box<dyn Node> {
        Box::new(TestNode::new(
            name,
            vec![PortDef {
                name: "in",
                port_type: PortType::Results,
                required: true,
            }],
            vec![],
        ))
    }

    fn passthrough_node(name: &str) -> Box<dyn Node> {
        Box::new(TestNode::new(
            name,
            vec![PortDef {
                name: "in",
                port_type: PortType::Results,
                required: true,
            }],
            vec![PortDef {
                name: "out",
                port_type: PortType::Results,
                required: false,
            }],
        ))
    }

    #[test]
    fn graph_connect_validates_ports() {
        let mut g = DataflowGraph::new();
        g.add_node(source_node("a")).unwrap();
        g.add_node(sink_node("b")).unwrap();

        // Bad port name
        let err = g.connect("a", "missing", "b", "in").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn graph_connect_type_mismatch() {
        let mut g = DataflowGraph::new();
        g.add_node(source_node("a")).unwrap(); // out: Results
        g.add_node(Box::new(TestNode::new(
            "b",
            vec![PortDef {
                name: "in",
                port_type: PortType::Children,
                required: true,
            }],
            vec![],
        )))
        .unwrap();

        let err = g.connect("a", "out", "b", "in").unwrap_err();
        assert!(err.contains("type mismatch"));
    }

    #[test]
    fn graph_topological_sort_linear() {
        let mut g = DataflowGraph::new();
        g.add_node(source_node("a")).unwrap();
        g.add_node(passthrough_node("b")).unwrap();
        g.add_node(sink_node("c")).unwrap();
        g.connect("a", "out", "b", "in").unwrap();
        g.connect("b", "out", "c", "in").unwrap();

        let order = g.topological_sort().unwrap();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn graph_cycle_detection() {
        let mut g = DataflowGraph::new();

        // Create two nodes that can form a cycle
        g.add_node(Box::new(TestNode::new(
            "a",
            vec![PortDef {
                name: "in",
                port_type: PortType::Results,
                required: false,
            }],
            vec![PortDef {
                name: "out",
                port_type: PortType::Results,
                required: false,
            }],
        )))
        .unwrap();
        g.add_node(Box::new(TestNode::new(
            "b",
            vec![PortDef {
                name: "in",
                port_type: PortType::Results,
                required: false,
            }],
            vec![PortDef {
                name: "out",
                port_type: PortType::Results,
                required: false,
            }],
        )))
        .unwrap();

        g.connect("a", "out", "b", "in").unwrap();
        // Force cycle edge directly
        g.edges.push(Edge {
            from_node: "b".into(),
            from_port: "out".into(),
            to_node: "a".into(),
            to_port: "in".into(),
        });

        let err = g.topological_sort().unwrap_err();
        assert!(err.contains("cycle"));
    }

    #[test]
    fn graph_validate_missing_required() {
        let mut g = DataflowGraph::new();
        g.add_node(sink_node("b")).unwrap(); // required "in" not connected

        let err = g.validate().unwrap_err();
        assert!(err.contains("required input"));
        assert!(err.contains("not connected"));
    }
}

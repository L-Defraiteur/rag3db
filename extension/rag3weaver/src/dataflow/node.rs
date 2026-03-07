//! Node traits and execution context for the dataflow graph.
//!
//! - [`Node`] — static node with fixed inputs/outputs
//! - [`DynamicNode`] — can emit new nodes/edges at runtime via [`GraphEmitter`]
//! - [`NodeContext`] — reads inputs, writes outputs during execution

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use super::port::{PortDef, PortValue};
use super::services::ServiceRegistry;

// ─── Node trait ──────────────────────────────────────────────────────────────

/// A static node in the dataflow graph.
#[async_trait]
pub trait Node: Send + Sync {
    /// Unique name of this node instance.
    fn name(&self) -> &str;

    /// Input port definitions.
    fn inputs(&self) -> &[PortDef];

    /// Output port definitions.
    fn outputs(&self) -> &[PortDef];

    /// Execute the node: read from ctx inputs, write to ctx outputs.
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String>;
}

// ─── DynamicNode trait ───────────────────────────────────────────────────────

/// A node that can emit new nodes and edges at runtime.
///
/// Used by `ExpansionNode` which creates `FetchRelatedNode` + `ComposeNode`
/// dynamically based on search results.
#[async_trait]
pub trait DynamicNode: Send + Sync {
    /// Unique name of this node instance.
    fn name(&self) -> &str;

    /// Input port definitions.
    fn inputs(&self) -> &[PortDef];

    /// Output port definitions.
    fn outputs(&self) -> &[PortDef];

    /// Execute and optionally emit new nodes/edges via the emitter.
    async fn execute_dynamic(
        &mut self,
        ctx: &mut NodeContext,
        emitter: &mut GraphEmitter,
    ) -> Result<(), String>;
}

// ─── NodeContext ─────────────────────────────────────────────────────────────

/// Execution context: typed input/output access + services + metrics for a node.
pub struct NodeContext {
    inputs: HashMap<String, PortValue>,
    outputs: HashMap<String, PortValue>,
    services: Arc<ServiceRegistry>,
    metrics: HashMap<String, serde_json::Value>,
}

impl NodeContext {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            services: Arc::new(ServiceRegistry::new()),
            metrics: HashMap::new(),
        }
    }

    pub fn with_services(services: Arc<ServiceRegistry>) -> Self {
        Self {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            services,
            metrics: HashMap::new(),
        }
    }

    /// Get a service by key, downcast to `T`.
    pub fn service<T: Send + Sync + 'static>(&self, key: &str) -> Option<Arc<T>> {
        self.services.get::<T>(key)
    }

    /// Read an input port value (borrow).
    pub fn input(&self, port: &str) -> Option<&PortValue> {
        self.inputs.get(port)
    }

    /// Take an input port value (moves it out).
    pub fn take_input(&mut self, port: &str) -> Option<PortValue> {
        self.inputs.remove(port)
    }

    /// Write a value to an output port.
    pub fn set_output(&mut self, port: &str, value: PortValue) {
        self.outputs.insert(port.to_string(), value);
    }

    /// Set an input port value (used by the runtime to populate inputs).
    pub(crate) fn set_input(&mut self, port: &str, value: PortValue) {
        self.inputs.insert(port.to_string(), value);
    }

    /// Log a structured metric (available in NodeCompleted event).
    pub fn log_metric(&mut self, key: &str, value: impl serde::Serialize) {
        if let Ok(v) = serde_json::to_value(value) {
            self.metrics.insert(key.to_string(), v);
        }
    }

    /// Drain all output values (used by the runtime after execution).
    pub(crate) fn drain_outputs(&mut self) -> HashMap<String, PortValue> {
        std::mem::take(&mut self.outputs)
    }

    /// Drain all metrics (used by the runtime after execution).
    pub(crate) fn drain_metrics(&mut self) -> HashMap<String, serde_json::Value> {
        std::mem::take(&mut self.metrics)
    }
}

// ─── GraphEmitter ────────────────────────────────────────────────────────────

/// Accumulates graph mutations emitted by a [`DynamicNode`].
pub struct GraphEmitter {
    pub(crate) added_nodes: Vec<Box<dyn Node>>,
    pub(crate) added_dynamic_nodes: Vec<Box<dyn DynamicNode>>,
    pub(crate) added_edges: Vec<super::graph::Edge>,
    pub(crate) initial_inputs: HashMap<String, HashMap<String, PortValue>>,
}

impl GraphEmitter {
    pub fn new() -> Self {
        Self {
            added_nodes: Vec::new(),
            added_dynamic_nodes: Vec::new(),
            added_edges: Vec::new(),
            initial_inputs: HashMap::new(),
        }
    }

    /// Add a static node to the graph.
    pub fn add_node(&mut self, node: Box<dyn Node>) {
        self.added_nodes.push(node);
    }

    /// Add a dynamic node to the graph.
    pub fn add_dynamic_node(&mut self, node: Box<dyn DynamicNode>) {
        self.added_dynamic_nodes.push(node);
    }

    /// Connect two ports.
    pub fn connect(
        &mut self,
        from_node: &str,
        from_port: &str,
        to_node: &str,
        to_port: &str,
    ) {
        self.added_edges.push(super::graph::Edge {
            from_node: from_node.to_string(),
            from_port: from_port.to_string(),
            to_node: to_node.to_string(),
            to_port: to_port.to_string(),
        });
    }

    /// Pre-load an input port on an emitted node with initial data.
    /// Used by DynamicNodes to pass data to newly created nodes.
    pub fn set_initial_input(&mut self, node_name: &str, port: &str, value: PortValue) {
        self.initial_inputs
            .entry(node_name.to_string())
            .or_default()
            .insert(port.to_string(), value);
    }

    /// Check if no mutations were emitted.
    pub fn is_empty(&self) -> bool {
        self.added_nodes.is_empty()
            && self.added_dynamic_nodes.is_empty()
            && self.added_edges.is_empty()
    }

    /// Drain all accumulated mutations.
    pub(crate) fn drain(
        &mut self,
    ) -> (
        Vec<Box<dyn Node>>,
        Vec<Box<dyn DynamicNode>>,
        Vec<super::graph::Edge>,
        HashMap<String, HashMap<String, PortValue>>,
    ) {
        (
            std::mem::take(&mut self.added_nodes),
            std::mem::take(&mut self.added_dynamic_nodes),
            std::mem::take(&mut self.added_edges),
            std::mem::take(&mut self.initial_inputs),
        )
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::port::PortType;
    use crate::search_strategy::UnifiedResult;

    #[test]
    fn node_context_input_output() {
        let mut ctx = NodeContext::new();
        ctx.set_input(
            "results",
            PortValue::Results(vec![UnifiedResult {
                uuid: "u1".into(),
                score: 0.9,
                entity: None,
                data: None,
                chunk: None,
                chunks: None,
                relation: None,
                matched_children: None,
                other_children: None,
                graph: None,
            }]),
        );

        // Read input
        let val = ctx.input("results").unwrap();
        assert!(matches!(val, PortValue::Results(r) if r.len() == 1));

        // Set output
        ctx.set_output("out", PortValue::Empty);
        let outputs = ctx.drain_outputs();
        assert!(outputs.contains_key("out"));

        // Input still there
        assert!(ctx.input("results").is_some());
    }

    #[test]
    fn node_context_take_input() {
        let mut ctx = NodeContext::new();
        ctx.set_input("query", PortValue::Empty);

        let taken = ctx.take_input("query");
        assert!(taken.is_some());

        // Gone after take
        assert!(ctx.input("query").is_none());
    }

    #[test]
    fn graph_emitter_drain() {
        let mut emitter = GraphEmitter::new();
        assert!(emitter.is_empty());

        emitter.connect("a", "out", "b", "in");
        assert!(!emitter.is_empty());

        let (nodes, dyn_nodes, edges, _initials) = emitter.drain();
        assert!(nodes.is_empty());
        assert!(dyn_nodes.is_empty());
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from_node, "a");
        assert_eq!(edges[0].to_node, "b");

        // After drain, empty again
        assert!(emitter.is_empty());
    }
}

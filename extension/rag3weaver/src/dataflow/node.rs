//! Node traits and execution context for the dataflow graph.
//!
//! - [`Node`] — static node with fixed inputs/outputs
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

    /// Type identifier for checkpoint serialization (e.g., "InsertRecordNode").
    /// Must be unique per node implementation and stable across versions.
    fn node_type(&self) -> &'static str {
        "Unknown"
    }

    /// Configuration snapshot for checkpoint (e.g., `{"gpu_batch_size": 32}`).
    /// Used to compute the graph hash for checkpoint identity.
    fn node_config(&self) -> serde_json::Value {
        serde_json::Value::Object(serde_json::Map::new())
    }

    /// Whether this node supports undo (rollback).
    ///
    /// Nodes that perform mutations (INSERT, SET, DELETE) should return `true`
    /// and implement [`undo_context`] + [`undo`]. Read-only nodes return `false`.
    fn can_undo(&self) -> bool {
        false
    }

    /// Undo context captured during execute(), serialized into the checkpoint.
    ///
    /// Called by the runtime AFTER a successful `execute()`. The returned JSON
    /// is persisted and later passed to `undo()` for rollback.
    fn undo_context(&self) -> Option<serde_json::Value> {
        None
    }

    /// Reverse the operation using the previously captured undo context.
    ///
    /// Called during rollback in reverse topological order.
    async fn undo(
        &mut self,
        _ctx: &mut NodeContext,
        _undo_ctx: serde_json::Value,
    ) -> Result<(), String> {
        Err("undo not supported".into())
    }
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

    /// Get the shared service registry (for sub-graph execution).
    pub fn services(&self) -> Arc<ServiceRegistry> {
        self.services.clone()
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
}

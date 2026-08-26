//! Node traits and execution context for the dataflow graph.
//!
//! These types mirror luciole's `Node`, `NodeContext`, `PortDef`, `ServiceRegistry`
//! with identical public APIs. When we swap to `luciole::execute_dag()`, nodes
//! will implement `luciole::Node` directly with zero changes to their execute() bodies.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;

pub use super::port::PortValue;

use super::port::PortDef;
use super::services::ServiceRegistry;

// ─── NodeLog ─────────────────────────────────────────────────────────────────

/// Log level for structured messages emitted by dataflow nodes.
#[derive(Debug, Clone, Serialize)]
pub enum NodeLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// A log entry emitted by a node during execution.
#[derive(Debug, Clone, Serialize)]
pub struct NodeLogEntry {
    pub level: NodeLogLevel,
    pub text: String,
}

// ─── Node trait ─────────────────────────────────────────────────────────────

/// A synchronous DAG node (luciole-compatible API).
///
/// Mirrors `luciole::Node` exactly. When we swap runtimes,
/// nodes implement `luciole::Node` directly (same method signatures).
pub trait Node: Send {
    /// Type identifier for the node (e.g., "InsertRecordNode").
    fn node_type(&self) -> &'static str;

    /// Input port declarations.
    fn inputs(&self) -> Vec<PortDef> {
        vec![]
    }

    /// Output port declarations.
    fn outputs(&self) -> Vec<PortDef> {
        vec![]
    }

    /// Execute the node: read from ctx inputs, write to ctx outputs.
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String>;

    /// Whether this node supports undo (rollback).
    fn can_undo(&self) -> bool {
        false
    }

    /// Undo context captured during execute(), boxed for type erasure.
    fn undo_context(&self) -> Option<Box<dyn Any + Send>> {
        None
    }

    /// Reverse the operation using the previously captured undo context.
    fn undo(&mut self, _ctx: Box<dyn Any + Send>) -> Result<(), String> {
        Err("undo not supported".to_string())
    }

    /// Serializable configuration for checkpoint identity.
    fn node_config(&self) -> Option<Box<dyn Any + Send>> {
        None
    }

    /// Instance name (used by our runtime for graph management).
    /// NOT part of luciole::Node — will be removed when we swap runtimes.
    fn name(&self) -> &str;
}

// ─── NodeContext ────────────────────────────────────────────────────────────

/// Execution context: typed input/output access + services + metrics + logs.
///
/// Public API mirrors `luciole::NodeContext` exactly:
/// - `input(port) -> Option<&PortValue>`
/// - `take_input(port) -> Option<PortValue>`
/// - `set_output(port, PortValue)`
/// - `trigger(port)`
/// - `metric(key, f64)`
/// - `info/warn/error/debug(&str)`
/// - `service::<T>(key) -> Option<&T>`
pub struct NodeContext {
    inputs: HashMap<String, PortValue>,
    outputs: HashMap<String, PortValue>,
    services: Arc<ServiceRegistry>,
    metrics: Vec<(String, f64)>,
    logs: Vec<(NodeLogLevel, String)>,
    /// Le run qui exécute ce nœud — vide hors runtime.
    run_id: String,
}

impl NodeContext {
    pub fn new() -> Self {
        Self {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            services: Arc::new(ServiceRegistry::new()),
            metrics: Vec::new(),
            logs: Vec::new(),
            run_id: String::new(),
        }
    }

    pub fn with_services(services: Arc<ServiceRegistry>) -> Self {
        Self {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            services,
            metrics: Vec::new(),
            logs: Vec::new(),
            run_id: String::new(),
        }
    }

    // ── Public API (luciole-compatible) ─────────────────────────────────

    /// Borrow an input port value.
    pub fn input(&self, port: &str) -> Option<&PortValue> {
        self.inputs.get(port)
    }

    /// Move an input port value out (consumed).
    pub fn take_input(&mut self, port: &str) -> Option<PortValue> {
        self.inputs.remove(port)
    }

    /// Set an output port value.
    pub fn set_output(&mut self, port: &str, value: PortValue) {
        self.outputs.insert(port.to_string(), value);
    }

    /// Emit a trigger signal on an output port.
    pub fn trigger(&mut self, port: &str) {
        self.outputs.insert(port.to_string(), PortValue::Trigger);
    }

    /// Record a numeric metric.
    pub fn metric(&mut self, key: &str, value: f64) {
        self.metrics.push((key.to_string(), value));
    }

    /// Log a debug message.
    pub fn debug(&mut self, msg: &str) {
        self.logs.push((NodeLogLevel::Debug, msg.to_string()));
    }

    /// Log an info message.
    pub fn info(&mut self, msg: &str) {
        self.logs.push((NodeLogLevel::Info, msg.to_string()));
    }

    /// Log a warning message.
    pub fn warn(&mut self, msg: &str) {
        self.logs.push((NodeLogLevel::Warn, msg.to_string()));
    }

    /// Log an error message.
    pub fn error(&mut self, msg: &str) {
        self.logs.push((NodeLogLevel::Error, msg.to_string()));
    }

    /// Access a shared service by key. Returns None if missing or wrong type.
    pub fn service<T: 'static>(&self, key: &str) -> Option<&T> {
        self.services.get::<T>(key)
    }

    // ── Runtime-internal methods ────────────────────────────────────────

    /// Set an input port value (used by the runtime to populate inputs).
    pub(crate) fn set_input(&mut self, port: &str, value: PortValue) {
        self.inputs.insert(port.to_string(), value);
    }

    /// Get the shared service registry (for sub-graph execution).
    pub fn services_arc(&self) -> Arc<ServiceRegistry> {
        self.services.clone()
    }

    /// L'identifiant du run courant — l'adresse de ce graphe sur le bus
    /// (`run.<id>`, `run.<id>.inbox`). Vide hors runtime.
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(crate) fn set_run_id(&mut self, run_id: &str) {
        self.run_id = run_id.to_string();
    }

    /// Drain all output values (used by the runtime after execution).
    pub(crate) fn drain_outputs(&mut self) -> HashMap<String, PortValue> {
        std::mem::take(&mut self.outputs)
    }

    /// Drain all metrics (used by the runtime after execution).
    pub(crate) fn drain_metrics(&mut self) -> HashMap<String, serde_json::Value> {
        self.metrics
            .drain(..)
            .map(|(k, v)| (k, serde_json::json!(v)))
            .collect()
    }

    /// Drain all log entries (used by the runtime after execution).
    pub(crate) fn drain_logs(&mut self) -> Vec<NodeLogEntry> {
        self.logs
            .drain(..)
            .map(|(level, text)| NodeLogEntry { level, text })
            .collect()
    }

    /// Borrow inputs (used by the runtime for snapshotting before execute).
    pub(crate) fn inputs(&self) -> &HashMap<String, PortValue> {
        &self.inputs
    }
}

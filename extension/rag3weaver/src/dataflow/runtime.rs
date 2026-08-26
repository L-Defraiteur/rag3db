//! Dataflow runtime: executes a DataflowGraph.
//!
//! Processes nodes in topological order. Emits [`DataflowEvent`]s via async_broadcast.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use async_broadcast::{InactiveReceiver, Sender};
use serde::Serialize;


use super::checkpoint::{
    CheckpointStore, ExecutionCheckpoint, CheckpointExecutionStatus,
    NodeCheckpoint, NodeCheckpointStatus,
    port_value_to_checkpoint, port_value_from_checkpoint, timestamp_ms,
};
use super::graph::DataflowGraph;
use super::node::{NodeContext, NodeLogLevel};
use super::observe::{TapRegistry, TapSpec, TapEvent};
use super::port::{merge_port_values, PortType, PortValue};
use super::report::ExecutionReport;
use super::services::ServiceRegistry;

// ─── PortSnapshot ────────────────────────────────────────────────────────────

/// Snapshot of a port's data at a point in time.
///
/// Captures the port type, item count, and optional JSON serialization
/// (reusing the checkpoint serialization infrastructure).
#[derive(Debug, Clone, Serialize)]
pub struct PortSnapshot {
    pub name: String,
    pub port_type: PortType,
    pub count: Option<usize>,
    pub data_json: Option<String>,
}

impl PortSnapshot {
    /// Build a snapshot from a port name and value, using checkpoint serialization.
    pub(crate) fn from_port(name: &str, value: &PortValue) -> Self {
        match port_value_to_checkpoint(value) {
            Ok(cpv) => PortSnapshot {
                name: name.to_string(),
                port_type: cpv.port_type,
                count: cpv.record_count,
                data_json: cpv.data_json,
            },
            Err(_) => PortSnapshot {
                name: name.to_string(),
                port_type: PortType::Any,
                count: None,
                data_json: None,
            },
        }
    }
}

// ─── DataflowEvent ───────────────────────────────────────────────────────────

/// Events emitted during graph execution.
#[derive(Debug, Clone, Serialize)]
pub enum DataflowEvent {
    /// A node is about to execute.
    NodeStarted {
        node: String,
        node_type: String,
        inputs: Vec<PortSnapshot>,
    },
    /// A node completed successfully.
    NodeCompleted {
        node: String,
        duration_ms: u64,
        outputs: Vec<PortSnapshot>,
        metrics: HashMap<String, serde_json::Value>,
    },
    /// A structured log message emitted by a node during execution.
    NodeLog {
        node: String,
        node_type: String,
        level: NodeLogLevel,
        text: String,
    },
    /// A node failed.
    NodeFailed { node: String, error: String },
    /// Checkpoint resume: a node was skipped (already completed in a prior run).
    CheckpointResumed {
        node: String,
        output_ports: Vec<String>,
    },
    /// All processing completed.
    Completed {
        total_nodes: usize,
        duration_ms: u64,
    },
    /// Processing failed.
    Failed { error: String },
}

// ─── DataflowOutput ──────────────────────────────────────────────────────────

/// Final outputs of graph execution: all port values keyed by (node, port).
#[derive(Debug)]
pub struct DataflowOutput {
    data: HashMap<String, HashMap<String, PortValue>>,
}

impl DataflowOutput {
    /// Get a specific output value.
    pub fn get(&self, node: &str, port: &str) -> Option<&PortValue> {
        self.data.get(node).and_then(|ports| ports.get(port))
    }

    /// Create an empty output (for tests).
    pub fn empty() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Create from pre-built data (used by luciole bridge).
    pub(crate) fn from_data(data: HashMap<String, HashMap<String, PortValue>>) -> Self {
        Self { data }
    }
}

// ─── DataflowRuntime ─────────────────────────────────────────────────────────

/// Executes a DataflowGraph.
pub struct DataflowRuntime {
    max_iterations: usize,
    event_tx: Sender<DataflowEvent>,
    _inactive_rx: InactiveReceiver<DataflowEvent>,
    taps: TapRegistry,
    services: Arc<ServiceRegistry>,
}

impl DataflowRuntime {
    pub fn new(max_iterations: usize) -> Self {
        let (mut tx, rx) = async_broadcast::broadcast(128);
        tx.set_overflow(true);
        Self {
            max_iterations,
            event_tx: tx,
            _inactive_rx: rx.deactivate(),
            taps: TapRegistry::new(),
            services: Arc::new(ServiceRegistry::new()),
        }
    }

    /// Create a runtime with a service registry.
    pub fn with_services(max_iterations: usize, services: ServiceRegistry) -> Self {
        let (mut tx, rx) = async_broadcast::broadcast(128);
        tx.set_overflow(true);
        Self {
            max_iterations,
            event_tx: tx,
            _inactive_rx: rx.deactivate(),
            taps: TapRegistry::new(),
            services: Arc::new(services),
        }
    }

    /// Create a runtime sharing an existing service registry (for sub-graph execution).
    pub fn with_services_arc(max_iterations: usize, services: Arc<ServiceRegistry>) -> Self {
        let (mut tx, rx) = async_broadcast::broadcast(128);
        tx.set_overflow(true);
        Self {
            max_iterations,
            event_tx: tx,
            _inactive_rx: rx.deactivate(),
            taps: TapRegistry::new(),
            services,
        }
    }

    /// Subscribe to execution events.
    pub fn subscribe(&self) -> async_broadcast::Receiver<DataflowEvent> {
        self._inactive_rx.activate_cloned()
    }

    /// Subscribe to events from a specific set of nodes only.
    /// Global events (Completed, Failed) are always included.
    pub fn subscribe_nodes(&self, nodes: &[&str]) -> NodeEventFilter {
        NodeEventFilter {
            rx: self._inactive_rx.activate_cloned(),
            nodes: nodes.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Tap a specific edge — receive cloned values flowing through it.
    pub fn tap(
        &mut self,
        from_node: &str,
        from_port: &str,
        to_node: &str,
        to_port: &str,
    ) -> async_broadcast::Receiver<TapEvent> {
        self.taps
            .add(TapSpec::new(from_node, from_port, to_node, to_port));
        self.taps.subscribe()
    }

    /// Tap all edges — receive cloned values flowing through every edge.
    pub fn tap_all(&mut self) -> async_broadcast::Receiver<TapEvent> {
        self.taps.set_all();
        self.taps.subscribe()
    }

    fn emit(&self, event: DataflowEvent) {
        let _ = self.event_tx.try_broadcast(event);
    }

    /// Execute the graph and return both the output and an ExecutionReport.
    pub fn execute_with_report(
        &self,
        graph: &mut DataflowGraph,
    ) -> Result<(DataflowOutput, ExecutionReport), String> {
        let mut rx = self.subscribe();
        let output = self.execute(graph)?;

        // Collect all events
        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }

        let report = ExecutionReport::build(&events, graph, &output);
        Ok((output, report))
    }

    /// Execute the graph with checkpoint persistence for crash recovery.
    ///
    /// - If `execution_id` exists in the store with status Running, resumes from
    ///   the last completed node (skips completed, injects saved outputs).
    /// - If new, creates a checkpoint and persists after each node completes.
    /// - On success: marks execution completed and cleans up node data.
    /// - On failure: marks execution failed (node data preserved for resume).
    pub fn execute_with_checkpoint(
        &self,
        graph: &mut DataflowGraph,
        store: &dyn CheckpointStore,
        execution_id: &str,
    ) -> Result<DataflowOutput, String> {
        let graph_def = graph.to_definition();
        let graph_hash = graph_def.hash();

        // Check for existing checkpoint
        let existing = store.load_execution(execution_id)?;

        if let Some(ref cp) = existing {
            // Validate graph hash matches
            if cp.graph_hash != graph_hash {
                return Err(format!(
                    "checkpoint graph_hash mismatch: stored={}, current={} — \
                     the graph structure has changed since the checkpoint was created",
                    cp.graph_hash, graph_hash
                ));
            }
            if cp.status == CheckpointExecutionStatus::Completed {
                // Already completed — no-op
                return Ok(DataflowOutput::empty());
            }
        }

        // Build initial checkpoint if new execution
        let checkpoint = match existing {
            Some(cp) => {
                // Resume: restore initial_inputs from checkpoint into the graph
                for (node_name, ports) in &cp.initial_inputs {
                    for (port_name, cpv) in ports {
                        let port_value = port_value_from_checkpoint(cpv.clone())?;
                        graph
                            .initial_inputs
                            .entry(node_name.clone())
                            .or_default()
                            .insert(port_name.clone(), port_value);
                    }
                }
                cp
            }
            None => {
                let now = timestamp_ms();
                let mut nodes = HashMap::new();
                for slot in &graph.nodes {
                    nodes.insert(
                        slot.name().to_string(),
                        NodeCheckpoint {
                            status: NodeCheckpointStatus::Pending,
                            output_ports: HashMap::new(),
                            undo_context: None,
                            duration_ms: None,
                            error: None,
                            completed_at: None,
                        },
                    );
                }

                // Serialize initial_inputs for resume
                let mut cp_initial_inputs = HashMap::new();
                for (node_name, ports) in &graph.initial_inputs {
                    let mut cp_ports = HashMap::new();
                    for (port_name, value) in ports {
                        let cpv = port_value_to_checkpoint(value)?;
                        cp_ports.insert(port_name.clone(), cpv);
                    }
                    cp_initial_inputs.insert(node_name.clone(), cp_ports);
                }

                let cp = ExecutionCheckpoint {
                    execution_id: execution_id.to_string(),
                    status: CheckpointExecutionStatus::Running,
                    graph_def,
                    graph_hash,
                    nodes,
                    initial_inputs: cp_initial_inputs,
                    created_at: now,
                    updated_at: now,
                };
                store.create_execution(&cp)?;
                cp
            }
        };

        // Run with checkpoint awareness
        let result = self
            .execute_inner_with_checkpoint(graph, store, execution_id, &checkpoint);

        match &result {
            Ok(_) => {
                store.mark_completed(execution_id)?;
            }
            Err(error) => {
                let _ = store.mark_failed(execution_id, error);
            }
        }

        result
    }

    /// Inner execution loop with checkpoint skip/save logic.
    fn execute_inner_with_checkpoint(
        &self,
        graph: &mut DataflowGraph,
        store: &dyn CheckpointStore,
        execution_id: &str,
        checkpoint: &ExecutionCheckpoint,
    ) -> Result<DataflowOutput, String> {
        let start = Instant::now();
        graph.validate()?;

        let mut port_data: HashMap<(String, String), PortValue> = HashMap::new();
        let mut port_data_available: HashSet<(String, String)> = HashSet::new();
        let mut initial_inputs: HashMap<String, HashMap<String, PortValue>> =
            std::mem::take(&mut graph.initial_inputs);
        let mut completed: HashSet<String> = HashSet::new();
        let order = graph.topological_sort()?;

        // Precompute remaining consumer count for each (from_node, from_port) key.
        let mut remaining_consumers: HashMap<(String, String), usize> = HashMap::new();
        for edge in &graph.edges {
            *remaining_consumers
                .entry((edge.from_node.clone(), edge.from_port.clone()))
                .or_insert(0) += 1;
        }

        // Inject saved outputs from completed nodes in the checkpoint
        for (node_name, node_cp) in &checkpoint.nodes {
            if node_cp.status == NodeCheckpointStatus::Completed {
                completed.insert(node_name.clone());

                // Restore output port values into port_data
                for (port_name, cpv) in &node_cp.output_ports {
                    let port_value = port_value_from_checkpoint(cpv.clone())?;
                    let key = (node_name.clone(), port_name.clone());
                    port_data_available.insert(key.clone());
                    port_data.insert(key, port_value);
                }

                self.emit(DataflowEvent::CheckpointResumed {
                    node: node_name.clone(),
                    output_ports: node_cp.output_ports.keys().cloned().collect(),
                });
            }
        }

        // Decrement consumer counts for edges whose downstream nodes were
        // already completed in the checkpoint (they won't consume again).
        for edge in &graph.edges {
            if completed.contains(&edge.to_node) {
                let key = (edge.from_node.clone(), edge.from_port.clone());
                if let Some(count) = remaining_consumers.get_mut(&key) {
                    *count = count.saturating_sub(1);
                }
            }
        }

        // Main execution loop (same structure as execute(), with checkpoint saves)
        for _iteration in 0..self.max_iterations {
            let ready: Vec<String> = order
                .iter()
                .filter(|name| !completed.contains(*name))
                .filter(|name| {
                    let node = graph.nodes.iter().find(|n| n.name() == *name).unwrap();
                    node.inputs().iter().all(|input| {
                        let has_incoming_edge = graph.edges.iter().any(|e| {
                            e.to_node == **name && e.to_port == input.name
                        });
                        // Optional inputs with no connected edge are always satisfied
                        if !input.required && !has_incoming_edge {
                            return true;
                        }
                        // For required inputs, OR optional inputs with connected edges:
                        // wait for upstream data to be available
                        let has_edge_data = graph.edges.iter().any(|e| {
                            e.to_node == **name
                                && e.to_port == input.name
                                && port_data_available.contains(&(
                                    e.from_node.clone(),
                                    e.from_port.clone(),
                                ))
                        });
                        let has_initial = initial_inputs
                            .get(*name)
                            .map_or(false, |ports| ports.contains_key(input.name));
                        has_edge_data || has_initial
                    })
                })
                .cloned()
                .collect();

            if ready.is_empty() {
                if completed.len() == graph.nodes.len() {
                    break;
                }
                let remaining: Vec<String> = order
                    .iter()
                    .filter(|n| !completed.contains(*n))
                    .cloned()
                    .collect();
                let error = format!("deadlock: nodes {:?} cannot execute", remaining);
                self.emit(DataflowEvent::Failed {
                    error: error.clone(),
                });
                return Err(error);
            }

            for node_name in &ready {
                let mut ctx = NodeContext::with_services(self.services.clone());
                let edges_for_node: Vec<_> = graph
                    .edges
                    .iter()
                    .filter(|e| e.to_node == *node_name)
                    .collect();

                let mut port_inputs: HashMap<String, Vec<PortValue>> = HashMap::new();
                for edge in &edges_for_node {
                    let key = (edge.from_node.clone(), edge.from_port.clone());
                    if port_data.contains_key(&key) {
                        let count = remaining_consumers.get_mut(&key).unwrap();
                        *count -= 1;
                        let value = if *count == 0 {
                            port_data.remove(&key).unwrap()
                        } else {
                            port_data.get(&key).unwrap().clone()
                        };
                        self.taps.check_and_emit(edge, &value);
                        port_inputs
                            .entry(edge.to_port.clone())
                            .or_default()
                            .push(value);
                    }
                }

                for (port, values) in port_inputs {
                    let merged = values.into_iter().reduce(|a, b| {
                        match merge_port_values(a, b) {
                            Ok(v) => v,
                            Err(_) => PortValue::Trigger,
                        }
                    });
                    if let Some(v) = merged {
                        ctx.set_input(&port, v);
                    }
                }

                if let Some(node_initials) = initial_inputs.remove(node_name) {
                    for (port, value) in node_initials {
                        ctx.set_input(&port, value);
                    }
                }

                let node_idx = graph
                    .nodes
                    .iter()
                    .position(|n| n.name() == *node_name)
                    .unwrap();
                let node_type = graph.nodes[node_idx].node_type().to_string();

                // Snapshot inputs BEFORE execute
                let input_snapshots: Vec<PortSnapshot> = ctx.inputs().iter()
                    .map(|(name, value)| PortSnapshot::from_port(name, value))
                    .collect();

                self.emit(DataflowEvent::NodeStarted {
                    node: node_name.clone(),
                    node_type: node_type.clone(),
                    inputs: input_snapshots,
                });
                let node_start = Instant::now();

                // Fail injection for testing: register a String service "fail_node"
                // in the ServiceRegistry to make a named node fail on execution.
                let should_fail = self
                    .services
                    .get::<String>("fail_node")
                    .map_or(false, |v| *v == *node_name);

                let exec_result = if should_fail {
                    Err(format!("injected failure for node '{}'", node_name))
                } else {
                    graph.nodes[node_idx].execute(&mut ctx)
                };

                let duration_ms = node_start.elapsed().as_millis() as u64;
                // Vers le bus commun, s'il est là : la trace *sous* un outil.
                if let Some(bus) = self.services.get::<Arc<crate::events::EventBus>>("event_bus") {
                    bus.emit(crate::events::CatalogEvent::NodeRun {
                        node: node_name.clone(),
                        node_type: node_type.clone(),
                        ms: duration_ms,
                        error: exec_result.as_ref().err().cloned(),
                    });
                }

                match exec_result {
                    Ok(()) => {
                        let outputs = ctx.drain_outputs();
                        let metrics = ctx.drain_metrics();
                        let logs = ctx.drain_logs();

                        // Emit NodeLog events
                        for log_entry in &logs {
                            self.emit(DataflowEvent::NodeLog {
                                node: node_name.clone(),
                                node_type: node_type.clone(),
                                level: log_entry.level.clone(),
                                text: log_entry.text.clone(),
                            });
                        }

                        // Capture undo context if the node supports it.
                        // The new trait returns Box<dyn Any + Send>; downcast to
                        // serde_json::Value for checkpoint serialization.
                        let undo_ctx_any = graph.nodes[node_idx].undo_context();
                        let undo_ctx: Option<serde_json::Value> = undo_ctx_any.and_then(|boxed| {
                            boxed.downcast::<serde_json::Value>().ok().map(|v| *v)
                        });

                        // Serialize outputs for checkpoint persistence + snapshots
                        let mut checkpoint_outputs = HashMap::new();
                        let mut output_snapshots: Vec<PortSnapshot> = Vec::new();
                        for (port, value) in &outputs {
                            let cpv = port_value_to_checkpoint(value)?;
                            output_snapshots.push(PortSnapshot {
                                name: port.clone(),
                                port_type: cpv.port_type,
                                count: cpv.record_count,
                                data_json: cpv.data_json.clone(),
                            });
                            checkpoint_outputs.insert(port.clone(), cpv);
                        }

                        // Persist to checkpoint store
                        store
                            .save_node_completed(
                                execution_id,
                                node_name,
                                &checkpoint_outputs,
                                undo_ctx.as_ref(),
                                duration_ms,
                            )?;

                        for (port, value) in outputs {
                            let key = (node_name.clone(), port.clone());
                            port_data_available.insert(key.clone());
                            port_data.insert(key, value);
                        }

                        completed.insert(node_name.clone());
                        self.emit(DataflowEvent::NodeCompleted {
                            node: node_name.clone(),
                            duration_ms,
                            outputs: output_snapshots,
                            metrics,
                        });
                    }
                    Err(error) => {
                        let _ = store
                            .save_node_failed(execution_id, node_name, &error);
                        self.emit(DataflowEvent::NodeFailed {
                            node: node_name.clone(),
                            error: error.clone(),
                        });
                        self.emit(DataflowEvent::Failed {
                            error: error.clone(),
                        });
                        return Err(error);
                    }
                }
            }

            if completed.len() == graph.nodes.len() {
                break;
            }
        }

        if completed.len() != graph.nodes.len() {
            let error = format!(
                "max iterations ({}) exceeded, {} of {} nodes completed",
                self.max_iterations,
                completed.len(),
                graph.nodes.len()
            );
            self.emit(DataflowEvent::Failed {
                error: error.clone(),
            });
            return Err(error);
        }

        let total_ms = start.elapsed().as_millis() as u64;
        self.emit(DataflowEvent::Completed {
            total_nodes: completed.len(),
            duration_ms: total_ms,
        });

        let mut data: HashMap<String, HashMap<String, PortValue>> = HashMap::new();
        for ((node, port), value) in port_data {
            data.entry(node).or_default().insert(port, value);
        }

        Ok(DataflowOutput { data })
    }

    /// Execute the graph. Returns all output values.
    pub fn execute(
        &self,
        graph: &mut DataflowGraph,
    ) -> Result<DataflowOutput, String> {
        let start = Instant::now();
        graph.validate()?;

        // Port data store: (node_name, port_name) → PortValue
        // Values may be removed when consumed by the last downstream edge,
        // so that PortValue::take() works (Arc refcount=1).
        let mut port_data: HashMap<(String, String), PortValue> = HashMap::new();
        // Tracks which output ports have been produced (for readiness checks),
        // independent of whether the value is still in port_data.
        let mut port_data_available: HashSet<(String, String)> = HashSet::new();
        let mut initial_inputs: HashMap<String, HashMap<String, PortValue>> =
            std::mem::take(&mut graph.initial_inputs);
        let mut completed: HashSet<String> = HashSet::new();
        let order = graph.topological_sort()?;

        // Precompute remaining consumer count for each (from_node, from_port) key.
        // Each edge that reads from a port is one consumer. When the last consumer
        // takes the value, we remove it from port_data (giving ownership with
        // Arc refcount=1) so that PortValue::take() works.
        let mut remaining_consumers: HashMap<(String, String), usize> = HashMap::new();
        for edge in &graph.edges {
            *remaining_consumers
                .entry((edge.from_node.clone(), edge.from_port.clone()))
                .or_insert(0) += 1;
        }

        for _iteration in 0..self.max_iterations {
            // Find ready nodes: not completed, all required inputs available
            let ready: Vec<String> = order
                .iter()
                .filter(|name| !completed.contains(*name))
                .filter(|name| {
                    let node = graph.nodes.iter().find(|n| n.name() == *name).unwrap();
                    node.inputs().iter().all(|input| {
                        let has_incoming_edge = graph.edges.iter().any(|e| {
                            e.to_node == **name && e.to_port == input.name
                        });
                        // Optional inputs with no connected edge are always satisfied
                        if !input.required && !has_incoming_edge {
                            return true;
                        }
                        // For required inputs, OR optional inputs with connected edges:
                        // wait for upstream data to be available
                        let has_edge_data = graph.edges.iter().any(|e| {
                            e.to_node == **name
                                && e.to_port == input.name
                                && port_data_available.contains(&(
                                    e.from_node.clone(),
                                    e.from_port.clone(),
                                ))
                        });
                        // Or initial_inputs provides it
                        let has_initial = initial_inputs
                            .get(*name)
                            .map_or(false, |ports| ports.contains_key(input.name));
                        has_edge_data || has_initial
                    })
                })
                .cloned()
                .collect();

            if ready.is_empty() {
                if completed.len() == graph.nodes.len() {
                    break;
                }
                // Check for unfinished nodes
                let remaining: Vec<String> = order
                    .iter()
                    .filter(|n| !completed.contains(*n))
                    .cloned()
                    .collect();
                let error = format!("deadlock: nodes {:?} cannot execute", remaining);
                self.emit(DataflowEvent::Failed {
                    error: error.clone(),
                });
                return Err(error);
            }

            for node_name in &ready {
                // Collect inputs from edges
                let mut ctx = NodeContext::with_services(self.services.clone());
                let edges_for_node: Vec<_> = graph
                    .edges
                    .iter()
                    .filter(|e| e.to_node == *node_name)
                    .collect();

                // Group edges by target port for fan-in, emit taps.
                // Track remaining consumers so the last reader gets ownership
                // (Arc refcount=1) instead of a clone (refcount=2).
                let mut port_inputs: HashMap<String, Vec<PortValue>> = HashMap::new();
                for edge in &edges_for_node {
                    let key = (edge.from_node.clone(), edge.from_port.clone());
                    if port_data.contains_key(&key) {
                        let count = remaining_consumers.get_mut(&key).unwrap();
                        *count -= 1;
                        let value = if *count == 0 {
                            // Last consumer: remove from port_data (ownership transfer)
                            port_data.remove(&key).unwrap()
                        } else {
                            // More consumers remain: clone (read-only ref)
                            port_data.get(&key).unwrap().clone()
                        };
                        self.taps.check_and_emit(edge, &value);
                        port_inputs
                            .entry(edge.to_port.clone())
                            .or_default()
                            .push(value);
                    }
                }

                // Merge fan-in and set inputs
                for (port, values) in port_inputs {
                    let merged = values.into_iter().reduce(|a, b| {
                        match merge_port_values(a, b) {
                            Ok(v) => v,
                            Err(_) => PortValue::Trigger,
                        }
                    });
                    if let Some(v) = merged {
                        ctx.set_input(&port, v);
                    }
                }

                if let Some(node_initials) = initial_inputs.remove(node_name) {
                    for (port, value) in node_initials {
                        ctx.set_input(&port, value);
                    }
                }

                let node_idx = graph
                    .nodes
                    .iter()
                    .position(|n| n.name() == *node_name)
                    .unwrap();
                let node_type = graph.nodes[node_idx].node_type().to_string();

                // Snapshot inputs BEFORE execute (they will be consumed by take_input)
                let input_snapshots: Vec<PortSnapshot> = ctx.inputs().iter()
                    .map(|(name, value)| PortSnapshot::from_port(name, value))
                    .collect();

                self.emit(DataflowEvent::NodeStarted {
                    node: node_name.clone(),
                    node_type: node_type.clone(),
                    inputs: input_snapshots,
                });
                let node_start = Instant::now();

                let exec_result = graph.nodes[node_idx].execute(&mut ctx);

                let duration_ms = node_start.elapsed().as_millis() as u64;
                // Vers le bus commun, s'il est là : la trace *sous* un outil.
                if let Some(bus) = self.services.get::<Arc<crate::events::EventBus>>("event_bus") {
                    bus.emit(crate::events::CatalogEvent::NodeRun {
                        node: node_name.clone(),
                        node_type: node_type.clone(),
                        ms: duration_ms,
                        error: exec_result.as_ref().err().cloned(),
                    });
                }

                match exec_result {
                    Ok(()) => {
                        let outputs = ctx.drain_outputs();
                        let metrics = ctx.drain_metrics();
                        let logs = ctx.drain_logs();

                        // Emit NodeLog events
                        for log_entry in &logs {
                            self.emit(DataflowEvent::NodeLog {
                                node: node_name.clone(),
                                node_type: node_type.clone(),
                                level: log_entry.level.clone(),
                                text: log_entry.text.clone(),
                            });
                        }

                        // Snapshot outputs AFTER execute
                        let output_snapshots: Vec<PortSnapshot> = outputs.iter()
                            .map(|(name, value)| PortSnapshot::from_port(name, value))
                            .collect();

                        for (port, value) in outputs {
                            let key = (node_name.clone(), port.clone());
                            port_data_available.insert(key.clone());
                            port_data.insert(key, value);
                        }

                        completed.insert(node_name.clone());
                        self.emit(DataflowEvent::NodeCompleted {
                            node: node_name.clone(),
                            duration_ms,
                            outputs: output_snapshots,
                            metrics,
                        });
                    }
                    Err(error) => {
                        self.emit(DataflowEvent::NodeFailed {
                            node: node_name.clone(),
                            error: error.clone(),
                        });
                        self.emit(DataflowEvent::Failed {
                            error: error.clone(),
                        });
                        return Err(error);
                    }
                }
            }

            // Check if all done
            if completed.len() == graph.nodes.len() {
                break;
            }
        }

        if completed.len() != graph.nodes.len() {
            let error = format!(
                "max iterations ({}) exceeded, {} of {} nodes completed",
                self.max_iterations,
                completed.len(),
                graph.nodes.len()
            );
            self.emit(DataflowEvent::Failed {
                error: error.clone(),
            });
            return Err(error);
        }

        let total_ms = start.elapsed().as_millis() as u64;
        self.emit(DataflowEvent::Completed {
            total_nodes: completed.len(),
            duration_ms: total_ms,
        });

        // Build output from remaining (unconsumed) port_data entries
        let mut data: HashMap<String, HashMap<String, PortValue>> = HashMap::new();
        for ((node, port), value) in port_data {
            data.entry(node).or_default().insert(port, value);
        }

        Ok(DataflowOutput { data })
    }
}

// ─── NodeEventFilter ────────────────────────────────────────────────────────

/// Filtered event receiver that only yields events for a given set of nodes.
/// Global events (Completed, Failed) always pass through.
pub struct NodeEventFilter {
    rx: async_broadcast::Receiver<DataflowEvent>,
    nodes: HashSet<String>,
}

impl NodeEventFilter {
    /// Try to receive the next matching event (non-blocking).
    pub fn try_recv(&mut self) -> Result<DataflowEvent, async_broadcast::TryRecvError> {
        loop {
            let ev = self.rx.try_recv()?;
            if self.matches(&ev) {
                return Ok(ev);
            }
        }
    }

    fn matches(&self, ev: &DataflowEvent) -> bool {
        match ev {
            DataflowEvent::NodeStarted { node, .. } => self.nodes.contains(node),
            DataflowEvent::NodeCompleted { node, .. } => self.nodes.contains(node),
            DataflowEvent::NodeLog { node, .. } => self.nodes.contains(node),
            DataflowEvent::NodeFailed { node, .. } => self.nodes.contains(node),
            DataflowEvent::CheckpointResumed { node, .. } => self.nodes.contains(node),
            DataflowEvent::Completed { .. } | DataflowEvent::Failed { .. } => true,
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::graph::DataflowGraph;
    use crate::dataflow::node::Node;
    use crate::dataflow::port::{PortDef, PortType, PortValue};
    use crate::search_strategy::UnifiedResult;

    /// Test node that passes Results from "in" to "out".
    struct PassthroughNode {
        name: String,
    }

    
    impl Node for PassthroughNode {
        fn node_type(&self) -> &'static str {
            "PassthroughNode"
        }
        fn name(&self) -> &str {
            &self.name
        }
        fn inputs(&self) -> Vec<PortDef> {
            vec![PortDef {
                name: "in",
                port_type: PortType::Results,
                required: true,
            }]
        }
        fn outputs(&self) -> Vec<PortDef> {
            vec![PortDef {
                name: "out",
                port_type: PortType::Results,
                required: false,
            }]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            if let Some(v) = ctx.take_input("in") {
                ctx.set_output("out", v);
            }
            Ok(())
        }
    }

    /// Source node that emits a fixed set of results.
    struct SourceNode {
        name: String,
        results: Vec<UnifiedResult>,
    }

    
    impl Node for SourceNode {
        fn node_type(&self) -> &'static str {
            "SourceNode"
        }
        fn name(&self) -> &str {
            &self.name
        }
        fn inputs(&self) -> Vec<PortDef> {
            vec![]
        }
        fn outputs(&self) -> Vec<PortDef> {
            vec![PortDef {
                name: "out",
                port_type: PortType::Results,
                required: false,
            }]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            ctx.set_output("out", PortValue::new(self.results.clone()));
            Ok(())
        }
    }

    /// Sink node that accepts results.
    struct SinkNode {
        name: String,
    }

    
    impl Node for SinkNode {
        fn node_type(&self) -> &'static str {
            "SinkNode"
        }
        fn name(&self) -> &str {
            &self.name
        }
        fn inputs(&self) -> Vec<PortDef> {
            vec![PortDef {
                name: "in",
                port_type: PortType::Results,
                required: true,
            }]
        }
        fn outputs(&self) -> Vec<PortDef> {
            vec![]
        }
        fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
            let _ = ctx.take_input("in");
            Ok(())
        }
    }

    fn test_result(uuid: &str) -> UnifiedResult {
        UnifiedResult {
            signal: None,
            uuid: uuid.into(),
            score: 1.0,
            entity: None,
            data: None,
            chunk: None,
            chunks: None,
            relation: None,
            matched_children: None,
            other_children: None,
            graph: None,
        }
    }

    #[test]
    fn runtime_linear_pipeline() {
        let mut graph = DataflowGraph::new();
        graph
            .add_node(Box::new(SourceNode {
                name: "source".into(),
                results: vec![test_result("u1")],
            }))
            .unwrap();
        graph
            .add_node(Box::new(PassthroughNode {
                name: "pass".into(),
            }))
            .unwrap();
        graph
            .add_node(Box::new(SinkNode {
                name: "sink".into(),
            }))
            .unwrap();
        graph.connect("source", "out", "pass", "in").unwrap();
        graph.connect("pass", "out", "sink", "in").unwrap();

        let runtime = DataflowRuntime::new(10);
        let mut rx = runtime.subscribe();
        let _output = runtime.execute(&mut graph).unwrap();

        // Verify execution completed (all nodes ran without errors)
        // Consumed intermediate outputs are removed from DataflowOutput,
        // so we verify via events instead.
        let mut completed_nodes = Vec::new();
        while let Ok(event) = rx.try_recv() {
            if let DataflowEvent::NodeCompleted { node, .. } = event {
                completed_nodes.push(node);
            }
        }
        assert!(completed_nodes.contains(&"source".to_string()));
        assert!(completed_nodes.contains(&"pass".to_string()));
        assert!(completed_nodes.contains(&"sink".to_string()));
    }

    #[test]
    fn runtime_fanout() {
        let mut graph = DataflowGraph::new();
        graph
            .add_node(Box::new(SourceNode {
                name: "source".into(),
                results: vec![test_result("u1")],
            }))
            .unwrap();
        graph
            .add_node(Box::new(SinkNode {
                name: "sink_a".into(),
            }))
            .unwrap();
        graph
            .add_node(Box::new(SinkNode {
                name: "sink_b".into(),
            }))
            .unwrap();
        graph.connect("source", "out", "sink_a", "in").unwrap();
        graph.connect("source", "out", "sink_b", "in").unwrap();

        let runtime = DataflowRuntime::new(10);
        // Fan-out: source.out goes to both sink_a and sink_b.
        // Both consume it, so it's removed from DataflowOutput.
        // Successful execution (no error) confirms fan-out worked.
        let _output = runtime.execute(&mut graph).unwrap();
    }

    #[test]
    fn runtime_fanin() {
        // Two sources feed into same input port of a sink
        struct DualInputSink;

        
        impl Node for DualInputSink {
            fn node_type(&self) -> &'static str {
                "DualInputSink"
            }
            fn name(&self) -> &str {
                "merge_sink"
            }
            fn inputs(&self) -> Vec<PortDef> {
                vec![PortDef {
                    name: "in",
                    port_type: PortType::Results,
                    required: true,
                }]
            }
            fn outputs(&self) -> Vec<PortDef> {
                vec![PortDef {
                    name: "out",
                    port_type: PortType::Results,
                    required: false,
                }]
            }
            fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
                if let Some(v) = ctx.take_input("in") {
                    ctx.set_output("out", v);
                }
                Ok(())
            }
        }

        let mut graph = DataflowGraph::new();
        graph
            .add_node(Box::new(SourceNode {
                name: "src_a".into(),
                results: vec![test_result("a1")],
            }))
            .unwrap();
        graph
            .add_node(Box::new(SourceNode {
                name: "src_b".into(),
                results: vec![test_result("b1"), test_result("b2")],
            }))
            .unwrap();
        graph.add_node(Box::new(DualInputSink)).unwrap();
        graph.connect("src_a", "out", "merge_sink", "in").unwrap();
        graph.connect("src_b", "out", "merge_sink", "in").unwrap();

        let runtime = DataflowRuntime::new(10);
        let output = runtime.execute(&mut graph).unwrap();

        let val = output.get("merge_sink", "out").unwrap();
        if let Some(r) = val.downcast::<Vec<UnifiedResult>>() {
            assert_eq!(r.len(), 3); // 1 from a + 2 from b
        } else {
            panic!("expected Results");
        }
    }

    #[test]
    fn runtime_tap_specific_edge() {
        let mut graph = DataflowGraph::new();
        graph
            .add_node(Box::new(SourceNode {
                name: "source".into(),
                results: vec![test_result("u1"), test_result("u2")],
            }))
            .unwrap();
        graph
            .add_node(Box::new(SinkNode {
                name: "sink".into(),
            }))
            .unwrap();
        graph.connect("source", "out", "sink", "in").unwrap();

        let mut runtime = DataflowRuntime::new(10);
        let mut tap_rx = runtime.tap("source", "out", "sink", "in");
        runtime.execute(&mut graph).unwrap();

        let event = tap_rx.try_recv().unwrap();
        assert_eq!(event.from_node, "source");
        assert_eq!(event.to_node, "sink");
        if let Some(r) = event.value.downcast::<Vec<UnifiedResult>>() {
            assert_eq!(r.len(), 2);
        } else {
            panic!("expected Results in tap event");
        }
    }

    #[test]
    fn runtime_tap_all() {
        let mut graph = DataflowGraph::new();
        graph
            .add_node(Box::new(SourceNode {
                name: "source".into(),
                results: vec![test_result("u1")],
            }))
            .unwrap();
        graph
            .add_node(Box::new(PassthroughNode {
                name: "pass".into(),
            }))
            .unwrap();
        graph
            .add_node(Box::new(SinkNode {
                name: "sink".into(),
            }))
            .unwrap();
        graph.connect("source", "out", "pass", "in").unwrap();
        graph.connect("pass", "out", "sink", "in").unwrap();

        let mut runtime = DataflowRuntime::new(10);
        let mut tap_rx = runtime.tap_all();
        runtime.execute(&mut graph).unwrap();

        // Should have 2 tap events (source→pass, pass→sink)
        let ev1 = tap_rx.try_recv().unwrap();
        let ev2 = tap_rx.try_recv().unwrap();
        assert_eq!(ev1.from_node, "source");
        assert_eq!(ev2.from_node, "pass");
    }

    #[test]
    fn runtime_execute_with_report() {
        let mut graph = DataflowGraph::new();
        graph
            .add_node(Box::new(SourceNode {
                name: "source".into(),
                results: vec![test_result("u1")],
            }))
            .unwrap();
        graph
            .add_node(Box::new(SinkNode {
                name: "sink".into(),
            }))
            .unwrap();
        graph.connect("source", "out", "sink", "in").unwrap();

        let runtime = DataflowRuntime::new(10);
        let (_output, report) = runtime.execute_with_report(&mut graph).unwrap();

        assert_eq!(report.nodes.len(), 2);
        assert!(matches!(
            report.status,
            super::super::report::ExecutionStatus::Completed
        ));
        assert!(report.total_duration_ms < 1000); // sanity check
        assert_eq!(report.edges.len(), 1);
        assert_eq!(report.edges[0].from_node, "source");
    }

    // ── Checkpoint tests ────────────────────────────────────────────────

    mod checkpoint_tests {
        use super::*;
        use crate::dataflow::checkpoint_store::MockCheckpointStore;
        use crate::dataflow::checkpoint::{
            CheckpointExecutionStatus, NodeCheckpointStatus,
        };

        /// Trigger node that emits Empty (checkpointable).
        struct TriggerNode {
            name: String,
        }

        
        impl Node for TriggerNode {
            fn name(&self) -> &str {
                &self.name
            }
            fn node_type(&self) -> &'static str {
                "TriggerNode"
            }
            fn inputs(&self) -> Vec<PortDef> {
                vec![]
            }
            fn outputs(&self) -> Vec<PortDef> {
                vec![PortDef {
                    name: "done",
                    port_type: PortType::Empty,
                    required: false,
                }]
            }
            fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
                ctx.set_output("done", PortValue::Trigger);
                Ok(())
            }
        }

        /// Receiver node that accepts a trigger.
        struct ReceiverNode {
            name: String,
        }

        
        impl Node for ReceiverNode {
            fn name(&self) -> &str {
                &self.name
            }
            fn node_type(&self) -> &'static str {
                "ReceiverNode"
            }
            fn inputs(&self) -> Vec<PortDef> {
                vec![PortDef {
                    name: "trigger",
                    port_type: PortType::Empty,
                    required: true,
                }]
            }
            fn outputs(&self) -> Vec<PortDef> {
                vec![PortDef {
                    name: "done",
                    port_type: PortType::Empty,
                    required: false,
                }]
            }
            fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
                let _ = ctx.take_input("trigger");
                ctx.set_output("done", PortValue::Trigger);
                Ok(())
            }
        }

        /// Node that fails on first call, succeeds on second.
        struct FailOnceNode {
            name: String,
            fail: bool,
        }

        
        impl Node for FailOnceNode {
            fn name(&self) -> &str {
                &self.name
            }
            fn node_type(&self) -> &'static str {
                "FailOnceNode"
            }
            fn inputs(&self) -> Vec<PortDef> {
                vec![PortDef {
                    name: "trigger",
                    port_type: PortType::Empty,
                    required: false,
                }]
            }
            fn outputs(&self) -> Vec<PortDef> {
                vec![PortDef {
                    name: "done",
                    port_type: PortType::Empty,
                    required: false,
                }]
            }
            fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
                if self.fail {
                    self.fail = false;
                    Err("simulated crash".to_string())
                } else {
                    ctx.set_output("done", PortValue::Trigger);
                    Ok(())
                }
            }
        }

        #[test]
        fn checkpoint_full_execution() {
            let store = MockCheckpointStore::new();

            let mut graph = DataflowGraph::new();
            graph.add_node(Box::new(TriggerNode { name: "trigger".into() })).unwrap();
            graph.add_node(Box::new(ReceiverNode { name: "receiver".into() })).unwrap();
            graph.connect("trigger", "done", "receiver", "trigger").unwrap();

            let runtime = DataflowRuntime::new(10);
            let output = runtime
                .execute_with_checkpoint(&mut graph, &store, "exec-full")
                .unwrap();

            // "trigger.done" is consumed by "receiver", so it's removed from output.
            // "receiver.done" is terminal (no downstream consumer).
            assert!(output.get("receiver", "done").is_some());

            let cp = store.load_execution("exec-full").unwrap().unwrap();
            assert_eq!(cp.status, CheckpointExecutionStatus::Completed);
        }

        #[test]
        fn checkpoint_resume_after_failure() {
            let store = MockCheckpointStore::new();

            // First run: trigger succeeds, crasher fails
            {
                let mut graph = DataflowGraph::new();
                graph.add_node(Box::new(TriggerNode { name: "trigger".into() })).unwrap();
                graph
                    .add_node(Box::new(FailOnceNode {
                        name: "crasher".into(),
                        fail: true,
                    }))
                    .unwrap();
                graph.connect("trigger", "done", "crasher", "trigger").unwrap();

                let runtime = DataflowRuntime::new(10);
                let err = runtime
                    .execute_with_checkpoint(&mut graph, &store, "exec-resume")
                    .unwrap_err();
                assert!(err.contains("simulated crash"));
            }

            // Check store: execution failed, trigger completed, crasher failed
            let cp = store.load_execution("exec-resume").unwrap().unwrap();
            assert_eq!(cp.status, CheckpointExecutionStatus::Failed);
            assert_eq!(cp.nodes["trigger"].status, NodeCheckpointStatus::Completed);
            assert_eq!(cp.nodes["crasher"].status, NodeCheckpointStatus::Failed);

            // Reset for resume
            store.mutate("exec-resume", |cp| {
                cp.status = CheckpointExecutionStatus::Running;
                let crasher = cp.nodes.get_mut("crasher").unwrap();
                crasher.status = NodeCheckpointStatus::Pending;
                crasher.error = None;
            });

            // Second run: should skip trigger, re-execute crasher (succeeds)
            {
                let mut graph = DataflowGraph::new();
                graph.add_node(Box::new(TriggerNode { name: "trigger".into() })).unwrap();
                graph
                    .add_node(Box::new(FailOnceNode {
                        name: "crasher".into(),
                        fail: false,
                    }))
                    .unwrap();
                graph.connect("trigger", "done", "crasher", "trigger").unwrap();

                let runtime = DataflowRuntime::new(10);
                let mut rx = runtime.subscribe();
                let output = runtime
                    .execute_with_checkpoint(&mut graph, &store, "exec-resume")
                    .unwrap();

                assert!(output.get("crasher", "done").is_some());

                // Should have emitted CheckpointResumed for trigger
                let mut events = Vec::new();
                while let Ok(ev) = rx.try_recv() {
                    events.push(ev);
                }
                let resumed = events.iter().any(|e| {
                    matches!(e, DataflowEvent::CheckpointResumed { node, .. } if node == "trigger")
                });
                assert!(resumed, "should emit CheckpointResumed for trigger");
            }

            let cp = store.load_execution("exec-resume").unwrap().unwrap();
            assert_eq!(cp.status, CheckpointExecutionStatus::Completed);
        }

        #[test]
        fn checkpoint_graph_hash_mismatch() {
            let store = MockCheckpointStore::new();

            // First run
            {
                let mut graph = DataflowGraph::new();
                graph.add_node(Box::new(TriggerNode { name: "trigger".into() })).unwrap();

                let runtime = DataflowRuntime::new(10);
                runtime
                    .execute_with_checkpoint(&mut graph, &store, "exec-hash")
                    .unwrap();
            }

            // Reset to Running
            store.mutate("exec-hash", |cp| {
                cp.status = CheckpointExecutionStatus::Running;
            });

            // Second run with different graph → hash mismatch
            {
                let mut graph = DataflowGraph::new();
                graph.add_node(Box::new(TriggerNode { name: "trigger".into() })).unwrap();
                graph.add_node(Box::new(ReceiverNode { name: "extra".into() })).unwrap();
                graph.connect("trigger", "done", "extra", "trigger").unwrap();

                let runtime = DataflowRuntime::new(10);
                let err = runtime
                    .execute_with_checkpoint(&mut graph, &store, "exec-hash")
                    .unwrap_err();
                assert!(err.contains("graph_hash mismatch"));
            }
        }

        #[test]
        fn checkpoint_already_completed_is_noop() {
            let store = MockCheckpointStore::new();

            let mut graph = DataflowGraph::new();
            graph.add_node(Box::new(TriggerNode { name: "trigger".into() })).unwrap();

            let runtime = DataflowRuntime::new(10);
            runtime
                .execute_with_checkpoint(&mut graph, &store, "exec-done")
                .unwrap();

            // Second call → no-op
            let mut graph2 = DataflowGraph::new();
            graph2.add_node(Box::new(TriggerNode { name: "trigger".into() })).unwrap();
            let output = runtime
                .execute_with_checkpoint(&mut graph2, &store, "exec-done")
                .unwrap();

            assert!(output.get("trigger", "done").is_none());
        }
    }
}

//! Dataflow runtime: executes a DataflowGraph.
//!
//! Processes nodes in topological order. When a DynamicNode emits new nodes,
//! re-sorts and continues. Emits [`DataflowEvent`]s via async_broadcast.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use async_broadcast::{InactiveReceiver, Sender};
use serde::Serialize;

use super::graph::{DataflowGraph, NodeSlot};
use super::node::{GraphEmitter, NodeContext};
use super::observe::{TapRegistry, TapSpec, TapEvent};
use super::port::{merge_port_values, PortValue};
use super::report::ExecutionReport;

// ─── DataflowEvent ───────────────────────────────────────────────────────────

/// Events emitted during graph execution.
#[derive(Debug, Clone, Serialize)]
pub enum DataflowEvent {
    /// A node is about to execute.
    NodeStarted { node: String },
    /// A node completed successfully.
    NodeCompleted {
        node: String,
        duration_ms: u64,
        output_ports: Vec<String>,
    },
    /// A node failed.
    NodeFailed { node: String, error: String },
    /// A dynamic node expanded the graph.
    GraphExpanded {
        by_node: String,
        added_nodes: Vec<String>,
        added_edges: usize,
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
}

// ─── DataflowRuntime ─────────────────────────────────────────────────────────

/// Executes a DataflowGraph.
pub struct DataflowRuntime {
    max_iterations: usize,
    event_tx: Sender<DataflowEvent>,
    _inactive_rx: InactiveReceiver<DataflowEvent>,
    taps: TapRegistry,
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
        }
    }

    /// Subscribe to execution events.
    pub fn subscribe(&self) -> async_broadcast::Receiver<DataflowEvent> {
        self._inactive_rx.activate_cloned()
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
    pub async fn execute_with_report(
        &self,
        graph: &mut DataflowGraph,
    ) -> Result<(DataflowOutput, ExecutionReport), String> {
        let mut rx = self.subscribe();
        let output = self.execute(graph).await?;

        // Collect all events
        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }

        let report = ExecutionReport::build(&events, graph, &output);
        Ok((output, report))
    }

    /// Execute the graph. Returns all output values.
    pub async fn execute(
        &self,
        graph: &mut DataflowGraph,
    ) -> Result<DataflowOutput, String> {
        let start = Instant::now();
        graph.validate()?;

        // Port data store: (node_name, port_name) → PortValue
        let mut port_data: HashMap<(String, String), PortValue> = HashMap::new();
        let mut completed: HashSet<String> = HashSet::new();
        let mut order = graph.topological_sort()?;

        for iteration in 0..self.max_iterations {
            // Find ready nodes: not completed, all required inputs available
            let ready: Vec<String> = order
                .iter()
                .filter(|name| !completed.contains(*name))
                .filter(|name| {
                    let node = graph.nodes.iter().find(|n| n.name() == *name).unwrap();
                    node.inputs().iter().all(|input| {
                        if !input.required {
                            return true;
                        }
                        // Check if any edge delivers to this port
                        graph.edges.iter().any(|e| {
                            e.to_node == **name
                                && e.to_port == input.name
                                && port_data.contains_key(&(
                                    e.from_node.clone(),
                                    e.from_port.clone(),
                                ))
                        })
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
                let mut ctx = NodeContext::new();
                let edges_for_node: Vec<_> = graph
                    .edges
                    .iter()
                    .filter(|e| e.to_node == *node_name)
                    .collect();

                // Group edges by target port for fan-in, emit taps
                let mut port_inputs: HashMap<String, Vec<PortValue>> = HashMap::new();
                for edge in &edges_for_node {
                    if let Some(value) =
                        port_data.get(&(edge.from_node.clone(), edge.from_port.clone()))
                    {
                        self.taps.check_and_emit(edge, value);
                        port_inputs
                            .entry(edge.to_port.clone())
                            .or_default()
                            .push(value.clone());
                    }
                }

                // Merge fan-in and set inputs
                for (port, values) in port_inputs {
                    let merged = values.into_iter().reduce(|a, b| {
                        match merge_port_values(a, b) {
                            Ok(v) => v,
                            Err(_) => PortValue::Empty,
                        }
                    });
                    if let Some(v) = merged {
                        ctx.set_input(&port, v);
                    }
                }

                self.emit(DataflowEvent::NodeStarted {
                    node: node_name.clone(),
                });
                let node_start = Instant::now();

                // Execute: find the node slot
                let node_idx = graph
                    .nodes
                    .iter()
                    .position(|n| n.name() == *node_name)
                    .unwrap();

                let exec_result = match &graph.nodes[node_idx] {
                    NodeSlot::Static(node) => node.execute(&mut ctx).await,
                    NodeSlot::Dynamic(node) => {
                        let mut emitter = GraphEmitter::new();
                        let result =
                            node.execute_dynamic(&mut ctx, &mut emitter).await;

                        if result.is_ok() && !emitter.is_empty() {
                            let (new_nodes, new_dyn_nodes, new_edges) =
                                emitter.drain();
                            let added_names: Vec<String> = new_nodes
                                .iter()
                                .map(|n| n.name().to_string())
                                .chain(new_dyn_nodes.iter().map(|n| n.name().to_string()))
                                .collect();
                            let added_edge_count = new_edges.len();

                            graph.merge_dynamic(new_nodes, new_dyn_nodes, new_edges)?;
                            order = graph.topological_sort()?;

                            self.emit(DataflowEvent::GraphExpanded {
                                by_node: node_name.clone(),
                                added_nodes: added_names,
                                added_edges: added_edge_count,
                            });
                        }

                        result
                    }
                };

                let duration_ms = node_start.elapsed().as_millis() as u64;

                match exec_result {
                    Ok(()) => {
                        let outputs = ctx.drain_outputs();
                        let output_ports: Vec<String> = outputs.keys().cloned().collect();

                        for (port, value) in outputs {
                            port_data.insert((node_name.clone(), port), value);
                        }

                        completed.insert(node_name.clone());
                        self.emit(DataflowEvent::NodeCompleted {
                            node: node_name.clone(),
                            duration_ms,
                            output_ports,
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

        // Reorganize port_data by node
        let mut data: HashMap<String, HashMap<String, PortValue>> = HashMap::new();
        for ((node, port), value) in port_data {
            data.entry(node).or_default().insert(port, value);
        }

        Ok(DataflowOutput { data })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::graph::DataflowGraph;
    use crate::dataflow::node::{DynamicNode, Node};
    use crate::dataflow::port::{PortDef, PortType, PortValue};
    use crate::search_strategy::UnifiedResult;

    /// Test node that passes Results from "in" to "out".
    struct PassthroughNode {
        name: String,
    }

    #[async_trait::async_trait]
    impl Node for PassthroughNode {
        fn name(&self) -> &str {
            &self.name
        }
        fn inputs(&self) -> &[PortDef] {
            &[PortDef {
                name: "in",
                port_type: PortType::Results,
                required: true,
            }]
        }
        fn outputs(&self) -> &[PortDef] {
            &[PortDef {
                name: "out",
                port_type: PortType::Results,
                required: false,
            }]
        }
        async fn execute(&self, ctx: &mut NodeContext) -> Result<(), String> {
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

    #[async_trait::async_trait]
    impl Node for SourceNode {
        fn name(&self) -> &str {
            &self.name
        }
        fn inputs(&self) -> &[PortDef] {
            &[]
        }
        fn outputs(&self) -> &[PortDef] {
            &[PortDef {
                name: "out",
                port_type: PortType::Results,
                required: false,
            }]
        }
        async fn execute(&self, ctx: &mut NodeContext) -> Result<(), String> {
            ctx.set_output("out", PortValue::Results(self.results.clone()));
            Ok(())
        }
    }

    /// Sink node that accepts results.
    struct SinkNode {
        name: String,
    }

    #[async_trait::async_trait]
    impl Node for SinkNode {
        fn name(&self) -> &str {
            &self.name
        }
        fn inputs(&self) -> &[PortDef] {
            &[PortDef {
                name: "in",
                port_type: PortType::Results,
                required: true,
            }]
        }
        fn outputs(&self) -> &[PortDef] {
            &[]
        }
        async fn execute(&self, ctx: &mut NodeContext) -> Result<(), String> {
            let _ = ctx.take_input("in");
            Ok(())
        }
    }

    fn test_result(uuid: &str) -> UnifiedResult {
        UnifiedResult {
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

    #[tokio::test]
    async fn runtime_linear_pipeline() {
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
        let output = runtime.execute(&mut graph).await.unwrap();

        // Passthrough should have forwarded results
        let val = output.get("pass", "out").unwrap();
        if let PortValue::Results(r) = val {
            assert_eq!(r.len(), 1);
            assert_eq!(r[0].uuid, "u1");
        } else {
            panic!("expected Results");
        }
    }

    #[tokio::test]
    async fn runtime_fanout() {
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
        let output = runtime.execute(&mut graph).await.unwrap();
        assert!(output.get("source", "out").is_some());
    }

    #[tokio::test]
    async fn runtime_fanin() {
        // Two sources feed into same input port of a sink
        struct DualInputSink;

        #[async_trait::async_trait]
        impl Node for DualInputSink {
            fn name(&self) -> &str {
                "merge_sink"
            }
            fn inputs(&self) -> &[PortDef] {
                &[PortDef {
                    name: "in",
                    port_type: PortType::Results,
                    required: true,
                }]
            }
            fn outputs(&self) -> &[PortDef] {
                &[PortDef {
                    name: "out",
                    port_type: PortType::Results,
                    required: false,
                }]
            }
            async fn execute(&self, ctx: &mut NodeContext) -> Result<(), String> {
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
        let output = runtime.execute(&mut graph).await.unwrap();

        let val = output.get("merge_sink", "out").unwrap();
        if let PortValue::Results(r) = val {
            assert_eq!(r.len(), 3); // 1 from a + 2 from b
        } else {
            panic!("expected Results");
        }
    }

    #[tokio::test]
    async fn runtime_dynamic_node() {
        /// DynamicNode that emits a new PassthroughNode.
        struct Expander;

        #[async_trait::async_trait]
        impl DynamicNode for Expander {
            fn name(&self) -> &str {
                "expander"
            }
            fn inputs(&self) -> &[PortDef] {
                &[PortDef {
                    name: "in",
                    port_type: PortType::Results,
                    required: true,
                }]
            }
            fn outputs(&self) -> &[PortDef] {
                &[PortDef {
                    name: "out",
                    port_type: PortType::Results,
                    required: false,
                }]
            }
            async fn execute_dynamic(
                &self,
                ctx: &mut NodeContext,
                emitter: &mut GraphEmitter,
            ) -> Result<(), String> {
                // Pass results through
                if let Some(v) = ctx.take_input("in") {
                    ctx.set_output("out", v);
                }
                // Emit a new passthrough node connected after us
                emitter.add_node(Box::new(PassthroughNode {
                    name: "dynamic_pass".into(),
                }));
                emitter.connect("expander", "out", "dynamic_pass", "in");
                Ok(())
            }
        }

        let mut graph = DataflowGraph::new();
        graph
            .add_node(Box::new(SourceNode {
                name: "source".into(),
                results: vec![test_result("u1")],
            }))
            .unwrap();
        graph.add_dynamic_node(Box::new(Expander)).unwrap();
        graph.connect("source", "out", "expander", "in").unwrap();

        let runtime = DataflowRuntime::new(10);
        let mut rx = runtime.subscribe();
        let output = runtime.execute(&mut graph).await.unwrap();

        // Dynamic node created "dynamic_pass", which should have received results
        let val = output.get("dynamic_pass", "out").unwrap();
        if let PortValue::Results(r) = val {
            assert_eq!(r.len(), 1);
            assert_eq!(r[0].uuid, "u1");
        } else {
            panic!("expected Results");
        }

        // Check for GraphExpanded event
        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }
        let expanded = events
            .iter()
            .any(|e| matches!(e, DataflowEvent::GraphExpanded { .. }));
        assert!(expanded, "should emit GraphExpanded event");
    }

    #[tokio::test]
    async fn runtime_tap_specific_edge() {
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
        runtime.execute(&mut graph).await.unwrap();

        let event = tap_rx.try_recv().unwrap();
        assert_eq!(event.from_node, "source");
        assert_eq!(event.to_node, "sink");
        if let PortValue::Results(r) = &event.value {
            assert_eq!(r.len(), 2);
        } else {
            panic!("expected Results in tap event");
        }
    }

    #[tokio::test]
    async fn runtime_tap_all() {
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
        runtime.execute(&mut graph).await.unwrap();

        // Should have 2 tap events (source→pass, pass→sink)
        let ev1 = tap_rx.try_recv().unwrap();
        let ev2 = tap_rx.try_recv().unwrap();
        assert_eq!(ev1.from_node, "source");
        assert_eq!(ev2.from_node, "pass");
    }

    #[tokio::test]
    async fn runtime_execute_with_report() {
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
        let (_output, report) = runtime.execute_with_report(&mut graph).await.unwrap();

        assert_eq!(report.nodes.len(), 2);
        assert!(matches!(
            report.status,
            super::super::report::ExecutionStatus::Completed
        ));
        assert!(report.total_duration_ms < 1000); // sanity check
        assert_eq!(report.edges.len(), 1);
        assert_eq!(report.edges[0].from_node, "source");
    }

    #[tokio::test]
    async fn runtime_max_iterations() {
        /// Infinite expander: keeps emitting new nodes.
        struct InfiniteExpander {
            name: String,
        }

        #[async_trait::async_trait]
        impl DynamicNode for InfiniteExpander {
            fn name(&self) -> &str {
                &self.name
            }
            fn inputs(&self) -> &[PortDef] {
                &[]
            }
            fn outputs(&self) -> &[PortDef] {
                &[PortDef {
                    name: "out",
                    port_type: PortType::Results,
                    required: false,
                }]
            }
            async fn execute_dynamic(
                &self,
                ctx: &mut NodeContext,
                emitter: &mut GraphEmitter,
            ) -> Result<(), String> {
                ctx.set_output("out", PortValue::Empty);
                // Keep spawning new expanders
                let next_name = format!("{}_child", self.name);
                emitter.add_dynamic_node(Box::new(InfiniteExpander {
                    name: next_name,
                }));
                Ok(())
            }
        }

        let mut graph = DataflowGraph::new();
        graph
            .add_dynamic_node(Box::new(InfiniteExpander {
                name: "root".into(),
            }))
            .unwrap();

        let runtime = DataflowRuntime::new(5);
        let err = runtime.execute(&mut graph).await.unwrap_err();
        assert!(err.contains("max iterations"));
    }
}

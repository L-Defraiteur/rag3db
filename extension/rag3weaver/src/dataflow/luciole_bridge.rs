//! Bridge between our DataflowGraph and luciole's execute_dag().
//!
//! Wraps each of our nodes in a [`LucioleNodeAdapter`] that implements
//! `luciole::Node`, then builds a `luciole::Dag` and delegates execution
//! to `luciole::execute_dag()`.
//!
//! This gives us luciole's level-parallel execution (nodes within the same
//! topological level run in parallel on the scheduler thread pool) while
//! keeping our existing node implementations unchanged.

use std::collections::HashMap;
use std::sync::Arc;

use luciole::port::PortType as LuciolePortType;

use super::graph::DataflowGraph;
use super::node::{Node as OurNode, NodeContext as OurNodeContext};
use super::port::PortValue;
use super::runtime::DataflowOutput;
use super::services::ServiceRegistry;

// ---------------------------------------------------------------------------
// LucioleNodeAdapter — wraps our Node as a luciole::Node
// ---------------------------------------------------------------------------

/// Wraps a rag3weaver Node as a luciole::Node for execute_dag().
///
/// The adapter bridges the two NodeContext types:
/// - On execute: reads inputs from luciole's ctx, populates our ctx, runs
///   our node, copies outputs back to luciole's ctx.
/// - Services are passed through our own ServiceRegistry (not luciole's).
struct LucioleNodeAdapter {
    inner: Box<dyn OurNode>,
    services: Arc<ServiceRegistry>,
}

impl luciole::Node for LucioleNodeAdapter {
    fn node_type(&self) -> &'static str {
        self.inner.node_type()
    }

    fn inputs(&self) -> Vec<luciole::PortDef> {
        self.inner
            .inputs()
            .into_iter()
            .map(|pd| {
                if pd.required {
                    luciole::PortDef::required(pd.name, LuciolePortType::Any)
                } else {
                    luciole::PortDef::optional(pd.name, LuciolePortType::Any)
                }
            })
            .collect()
    }

    fn outputs(&self) -> Vec<luciole::PortDef> {
        self.inner
            .outputs()
            .into_iter()
            .map(|pd| {
                if pd.required {
                    luciole::PortDef::required(pd.name, LuciolePortType::Any)
                } else {
                    luciole::PortDef::optional(pd.name, LuciolePortType::Any)
                }
            })
            .collect()
    }

    fn execute(&mut self, ctx: &mut luciole::NodeContext) -> Result<(), String> {
        // 1. Build our NodeContext with services
        let mut our_ctx = OurNodeContext::with_services(self.services.clone());

        // 2. Transfer inputs from luciole ctx → our ctx
        //    We know which ports to check from the node's declared inputs.
        for pd in self.inner.inputs() {
            if let Some(value) = ctx.take_input(pd.name) {
                our_ctx.set_input(pd.name, value);
            }
        }

        // 3. Execute our node
        self.inner.execute(&mut our_ctx)?;

        // 4. Transfer outputs from our ctx → luciole ctx
        let outputs = our_ctx.drain_outputs();
        for (port, value) in outputs {
            ctx.set_output(&port, value);
        }

        // 5. Transfer metrics
        let metrics = our_ctx.drain_metrics();
        for (key, value) in metrics {
            if let Some(f) = value.as_f64() {
                ctx.metric(&key, f);
            }
        }

        // 6. Transfer logs
        let logs = our_ctx.drain_logs();
        for log_entry in logs {
            match log_entry.level {
                super::node::NodeLogLevel::Debug => ctx.debug(&log_entry.text),
                super::node::NodeLogLevel::Info => ctx.info(&log_entry.text),
                super::node::NodeLogLevel::Warn => ctx.warn(&log_entry.text),
                super::node::NodeLogLevel::Error => ctx.error(&log_entry.text),
            }
        }

        Ok(())
    }

    fn can_undo(&self) -> bool {
        self.inner.can_undo()
    }

    fn undo_context(&self) -> Option<Box<dyn std::any::Any + Send>> {
        self.inner.undo_context()
    }

    fn undo(&mut self, ctx: Box<dyn std::any::Any + Send>) -> Result<(), String> {
        self.inner.undo(ctx)
    }

    fn node_config(&self) -> Option<Box<dyn std::any::Any + Send>> {
        self.inner.node_config()
    }
}

// ---------------------------------------------------------------------------
// execute_via_luciole — main entry point
// ---------------------------------------------------------------------------

/// Execute a DataflowGraph using luciole's DAG engine.
///
/// Converts the graph to a `luciole::Dag`, wrapping each node in a
/// `LucioleNodeAdapter`. This enables level-parallel execution: nodes
/// at the same topological level run concurrently on the thread pool.
///
/// Returns a `DataflowOutput` with the same semantics as
/// `DataflowRuntime::execute()`.
pub fn execute_via_luciole(
    graph: &mut DataflowGraph,
    services: Arc<ServiceRegistry>,
) -> Result<DataflowOutput, String> {
    // 1. Build the luciole Dag
    let mut dag = luciole::Dag::new();

    // Take nodes out of the graph (we'll put adapted versions in the Dag)
    let nodes = std::mem::take(&mut graph.nodes);
    let mut node_names = Vec::with_capacity(nodes.len());

    for node in nodes {
        let name = node.name().to_string();
        let adapter = LucioleNodeAdapter {
            inner: node,
            services: services.clone(),
        };
        dag.add_node(&name, adapter);
        node_names.push(name);
    }

    // 2. Connect edges
    //    Skip validation (already done by DataflowGraph) — use connect which
    //    validates port existence and types. Since we mapped all ports to Any,
    //    type checks will pass.
    for edge in &graph.edges {
        dag.connect(&edge.from_node, &edge.from_port, &edge.to_node, &edge.to_port)?;
    }

    // 3. Set initial inputs
    let initial_inputs = std::mem::take(&mut graph.initial_inputs);
    for (node_name, ports) in initial_inputs {
        for (port_name, value) in ports {
            dag.set_initial_input(&node_name, &port_name, value);
        }
    }

    // 4. Execute via luciole
    let mut result = luciole::execute_dag(&mut dag, None)?;

    // 5. Convert DagResult → DataflowOutput
    //    DagResult.outputs is HashMap<(node, port), PortValue>
    let mut data: HashMap<String, HashMap<String, PortValue>> = HashMap::new();
    for ((node, port), value) in result.outputs.drain() {
        data.entry(node).or_default().insert(port, value);
    }

    Ok(DataflowOutput::from_data(data))
}

/// Execute a DataflowGraph using luciole's DAG engine, returning the
/// full `luciole::DagResult` for inspection (metrics, logs, timing).
pub fn execute_via_luciole_with_result(
    graph: &mut DataflowGraph,
    services: Arc<ServiceRegistry>,
) -> Result<luciole::DagResult, String> {
    let mut dag = luciole::Dag::new();

    let nodes = std::mem::take(&mut graph.nodes);
    for node in nodes {
        let name = node.name().to_string();
        let adapter = LucioleNodeAdapter {
            inner: node,
            services: services.clone(),
        };
        dag.add_node(&name, adapter);
    }

    for edge in &graph.edges {
        dag.connect(&edge.from_node, &edge.from_port, &edge.to_node, &edge.to_port)?;
    }

    let initial_inputs = std::mem::take(&mut graph.initial_inputs);
    for (node_name, ports) in initial_inputs {
        for (port_name, value) in ports {
            dag.set_initial_input(&node_name, &port_name, value);
        }
    }

    luciole::execute_dag(&mut dag, None)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::graph::DataflowGraph;
    use crate::dataflow::node::{Node as OurNode, NodeContext as OurNodeContext};
    use crate::dataflow::port::{PortDef, PortType, PortValue};
    use crate::search_strategy::UnifiedResult;

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

    struct SourceNode {
        name: String,
        results: Vec<UnifiedResult>,
    }

    impl OurNode for SourceNode {
        fn node_type(&self) -> &'static str { "SourceNode" }
        fn name(&self) -> &str { &self.name }
        fn outputs(&self) -> Vec<PortDef> {
            vec![PortDef { name: "out", port_type: PortType::Results, required: false }]
        }
        fn execute(&mut self, ctx: &mut OurNodeContext) -> Result<(), String> {
            ctx.set_output("out", PortValue::new(self.results.clone()));
            ctx.metric("emitted", self.results.len() as f64);
            Ok(())
        }
    }

    struct PassthroughNode { name: String }

    impl OurNode for PassthroughNode {
        fn node_type(&self) -> &'static str { "PassthroughNode" }
        fn name(&self) -> &str { &self.name }
        fn inputs(&self) -> Vec<PortDef> {
            vec![PortDef { name: "in", port_type: PortType::Results, required: true }]
        }
        fn outputs(&self) -> Vec<PortDef> {
            vec![PortDef { name: "out", port_type: PortType::Results, required: false }]
        }
        fn execute(&mut self, ctx: &mut OurNodeContext) -> Result<(), String> {
            if let Some(v) = ctx.take_input("in") {
                ctx.set_output("out", v);
            }
            Ok(())
        }
    }

    struct SinkNode { name: String }

    impl OurNode for SinkNode {
        fn node_type(&self) -> &'static str { "SinkNode" }
        fn name(&self) -> &str { &self.name }
        fn inputs(&self) -> Vec<PortDef> {
            vec![PortDef { name: "in", port_type: PortType::Results, required: true }]
        }
        fn execute(&mut self, ctx: &mut OurNodeContext) -> Result<(), String> {
            let _ = ctx.take_input("in");
            Ok(())
        }
    }

    #[test]
    fn luciole_bridge_linear_pipeline() {
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(SourceNode {
            name: "source".into(),
            results: vec![test_result("u1")],
        })).unwrap();
        graph.add_node(Box::new(PassthroughNode {
            name: "pass".into(),
        })).unwrap();
        graph.add_node(Box::new(SinkNode {
            name: "sink".into(),
        })).unwrap();
        graph.connect("source", "out", "pass", "in").unwrap();
        graph.connect("pass", "out", "sink", "in").unwrap();

        let services = Arc::new(ServiceRegistry::new());
        let _output = execute_via_luciole(&mut graph, services).unwrap();
    }

    #[test]
    fn luciole_bridge_with_initial_inputs() {
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(SinkNode {
            name: "sink".into(),
        })).unwrap();
        graph.set_initial_input(
            "sink",
            "in",
            PortValue::new(vec![test_result("u1")]),
        );

        let services = Arc::new(ServiceRegistry::new());
        let _output = execute_via_luciole(&mut graph, services).unwrap();
    }

    #[test]
    fn luciole_bridge_metrics_and_logs() {
        struct MetricNode { name: String }
        impl OurNode for MetricNode {
            fn node_type(&self) -> &'static str { "MetricNode" }
            fn name(&self) -> &str { &self.name }
            fn execute(&mut self, ctx: &mut OurNodeContext) -> Result<(), String> {
                ctx.metric("docs", 42.0);
                ctx.info("hello from node");
                Ok(())
            }
        }

        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(MetricNode { name: "m".into() })).unwrap();

        let services = Arc::new(ServiceRegistry::new());
        let result = execute_via_luciole_with_result(&mut graph, services).unwrap();
        let nr = result.get("m").unwrap();
        assert_eq!(nr.metrics[0], ("docs".to_string(), 42.0));
        assert_eq!(nr.logs.len(), 1);
    }

    #[test]
    fn luciole_bridge_services_accessible() {
        struct ServiceNode { name: String }
        impl OurNode for ServiceNode {
            fn node_type(&self) -> &'static str { "ServiceNode" }
            fn name(&self) -> &str { &self.name }
            fn execute(&mut self, ctx: &mut OurNodeContext) -> Result<(), String> {
                let val = ctx.service::<String>("test_key")
                    .ok_or("missing service")?;
                ctx.metric("len", val.len() as f64);
                Ok(())
            }
        }

        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(ServiceNode { name: "s".into() })).unwrap();

        let mut services = ServiceRegistry::new();
        services.register("test_key", "hello_world".to_string());
        let services = Arc::new(services);

        let result = execute_via_luciole_with_result(&mut graph, services).unwrap();
        let nr = result.get("s").unwrap();
        assert_eq!(nr.metrics[0].1, 11.0); // "hello_world".len()
    }

    #[test]
    fn luciole_bridge_error_propagation() {
        struct FailNode { name: String }
        impl OurNode for FailNode {
            fn node_type(&self) -> &'static str { "FailNode" }
            fn name(&self) -> &str { &self.name }
            fn execute(&mut self, _ctx: &mut OurNodeContext) -> Result<(), String> {
                Err("boom".into())
            }
        }

        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(FailNode { name: "fail".into() })).unwrap();

        let services = Arc::new(ServiceRegistry::new());
        let err = execute_via_luciole(&mut graph, services).unwrap_err();
        assert!(err.contains("boom"));
    }

    #[test]
    fn luciole_bridge_fanout() {
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(SourceNode {
            name: "source".into(),
            results: vec![test_result("u1")],
        })).unwrap();
        graph.add_node(Box::new(SinkNode { name: "sink_a".into() })).unwrap();
        graph.add_node(Box::new(SinkNode { name: "sink_b".into() })).unwrap();
        graph.connect("source", "out", "sink_a", "in").unwrap();
        graph.connect("source", "out", "sink_b", "in").unwrap();

        let services = Arc::new(ServiceRegistry::new());
        let _output = execute_via_luciole(&mut graph, services).unwrap();
    }
}

//! Execution report: serializable summary of a dataflow execution.
//!
//! Built from [`DataflowEvent`]s collected during runtime + the graph structure.
//! Designed for frontend display, logging, and recording.

use serde::Serialize;

use super::graph::DataflowGraph;
use super::node::NodeLogEntry;
use super::port::PortValue;
use super::runtime::{DataflowEvent, DataflowOutput, PortSnapshot};

// ─── Status enums ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub enum ExecutionStatus {
    Completed,
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize)]
pub enum NodeStatus {
    Completed,
    Failed { error: String },
    Resumed,
}

// ─── NodeReport ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct NodeReport {
    pub name: String,
    pub node_type: String,
    pub status: NodeStatus,
    pub duration_ms: u64,
    pub inputs: Vec<PortSnapshot>,
    pub outputs: Vec<PortSnapshot>,
    pub logs: Vec<NodeLogEntry>,
    pub metrics: std::collections::HashMap<String, serde_json::Value>,
}

// ─── EdgeReport ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct EdgeReport {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
    pub value_summary: String,
}

// ─── ExecutionReport ────────────────────────────────────────────────────────

/// Complete summary of a dataflow execution.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionReport {
    pub nodes: Vec<NodeReport>,
    pub edges: Vec<EdgeReport>,
    pub expanded_nodes: Vec<String>,
    pub total_duration_ms: u64,
    pub status: ExecutionStatus,
}

/// Summarize a PortValue for display (type + count).
pub fn summarize_port_value(value: &PortValue) -> String {
    use crate::search_strategy::{ChildSummary, UnifiedResult};
    use crate::dataflow::port::{BatchPayload, QueryPayload};

    if value.is_trigger() {
        return "Trigger".into();
    }
    if let Some(r) = value.downcast::<Vec<UnifiedResult>>() {
        return format!("Results({})", r.len());
    }
    if let Some(c) = value.downcast::<std::collections::HashMap<String, Vec<ChildSummary>>>() {
        return format!("Children({} parents)", c.len());
    }
    if let Some(u) = value.downcast::<Vec<(String, String)>>() {
        return format!("Uuids({})", u.len());
    }
    if value.is::<crate::search::SearchMeta>() {
        return "Meta".into();
    }
    if let Some(q) = value.downcast::<QueryPayload>() {
        return format!("Query({})", q.target_name);
    }
    if let Some(p) = value.downcast::<BatchPayload>() {
        return format!("{:?}({})", p.batch_type, p.count());
    }
    if value.is::<serde_json::Value>() {
        return "Map".into();
    }
    "Data(?)".into()
}

/// Même résumé que [`summarize_port_value`], depuis un instantané (valeur déjà
/// consommée par son nœud).
pub fn summarize_snapshot(snap: &PortSnapshot) -> String {
    use super::port::PortType;
    let n = snap.count.unwrap_or(0);
    match snap.port_type {
        PortType::Results => format!("Results({n})"),
        PortType::Children => format!("Children({n} parents)"),
        PortType::Uuids => format!("Uuids({n})"),
        PortType::Meta => "Meta".into(),
        PortType::Query => snap
            .data_json
            .as_deref()
            .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
            .and_then(|v| v.get("target_name").and_then(|t| t.as_str()).map(str::to_owned))
            .map(|t| format!("Query({t})"))
            .unwrap_or_else(|| "Query".into()),
        PortType::Empty => "Trigger".into(),
        ref other => format!("{other:?}({n})"),
    }
}

impl ExecutionReport {
    /// Build a report from collected events and the final graph/output.
    pub fn build(
        events: &[DataflowEvent],
        graph: &DataflowGraph,
        output: &DataflowOutput,
    ) -> Self {
        let mut nodes = Vec::new();
        let expanded_nodes = Vec::new();
        let mut total_duration_ms = 0u64;
        let mut status = ExecutionStatus::Completed;

        // Collect per-node data from Start/Log events before building reports
        let mut node_inputs: std::collections::HashMap<String, Vec<PortSnapshot>> =
            std::collections::HashMap::new();
        // (nœud, port d'entrée) → instantané : les valeurs intermédiaires sont
        // déplacées vers leur consommateur, `output` ne garde que les terminales.
        let mut input_snapshots: std::collections::HashMap<(String, String), PortSnapshot> =
            std::collections::HashMap::new();
        let mut node_types: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut node_logs: std::collections::HashMap<String, Vec<NodeLogEntry>> =
            std::collections::HashMap::new();

        for event in events {
            match event {
                DataflowEvent::NodeStarted {
                    node, node_type, inputs,
                } => {
                    for snap in inputs {
                        input_snapshots.insert((node.clone(), snap.name.clone()), snap.clone());
                    }
                    node_inputs.insert(node.clone(), inputs.clone());
                    node_types.insert(node.clone(), node_type.clone());
                }
                DataflowEvent::NodeLog {
                    node, level, text, ..
                } => {
                    node_logs.entry(node.clone()).or_default().push(NodeLogEntry {
                        level: level.clone(),
                        text: text.clone(),
                    });
                }
                DataflowEvent::NodeCompleted {
                    node,
                    duration_ms,
                    outputs,
                    metrics,
                } => {
                    nodes.push(NodeReport {
                        name: node.clone(),
                        node_type: node_types.remove(node).unwrap_or_default(),
                        status: NodeStatus::Completed,
                        duration_ms: *duration_ms,
                        inputs: node_inputs.remove(node).unwrap_or_default(),
                        outputs: outputs.clone(),
                        logs: node_logs.remove(node).unwrap_or_default(),
                        metrics: metrics.clone(),
                    });
                }
                DataflowEvent::NodeFailed { node, error } => {
                    nodes.push(NodeReport {
                        name: node.clone(),
                        node_type: node_types.remove(node).unwrap_or_default(),
                        status: NodeStatus::Failed {
                            error: error.clone(),
                        },
                        duration_ms: 0,
                        inputs: node_inputs.remove(node).unwrap_or_default(),
                        outputs: vec![],
                        logs: node_logs.remove(node).unwrap_or_default(),
                        metrics: std::collections::HashMap::new(),
                    });
                }
                DataflowEvent::CheckpointResumed {
                    node,
                    output_ports,
                } => {
                    // Resumed nodes only have port names, not full snapshots
                    let output_snapshots: Vec<PortSnapshot> = output_ports.iter()
                        .map(|name| PortSnapshot {
                            name: name.clone(),
                            port_type: super::port::PortType::Empty,
                            count: None,
                            data_json: None,
                        })
                        .collect();
                    nodes.push(NodeReport {
                        name: node.clone(),
                        node_type: String::new(),
                        status: NodeStatus::Resumed,
                        duration_ms: 0,
                        inputs: vec![],
                        outputs: output_snapshots,
                        logs: vec![],
                        metrics: std::collections::HashMap::new(),
                    });
                }
                DataflowEvent::Completed { duration_ms, .. } => {
                    total_duration_ms = *duration_ms;
                }
                DataflowEvent::Failed { error } => {
                    status = ExecutionStatus::Failed {
                        error: error.clone(),
                    };
                }
            }
        }

        // Build edge reports from graph structure + output data
        let edges = graph
            .edges
            .iter()
            .map(|e| {
                let value_summary = output
                    .get(&e.from_node, &e.from_port)
                    .map(summarize_port_value)
                    .or_else(|| {
                        input_snapshots
                            .get(&(e.to_node.clone(), e.to_port.clone()))
                            .map(summarize_snapshot)
                    })
                    .unwrap_or_else(|| "—".into());
                EdgeReport {
                    from_node: e.from_node.clone(),
                    from_port: e.from_port.clone(),
                    to_node: e.to_node.clone(),
                    to_port: e.to_port.clone(),
                    value_summary,
                }
            })
            .collect();

        ExecutionReport {
            nodes,
            edges,
            expanded_nodes,
            total_duration_ms,
            status,
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_strategy::UnifiedResult;

    #[test]
    fn summarize_results() {
        let val = PortValue::new(vec![
            UnifiedResult {
                signal: None,
                uuid: "u1".into(),
                score: 1.0,
                entity: None,
                data: None,
                chunk: None,
                chunks: None,
                relation: None,
                matched_children: None,
                other_children: None,
                graph: None,
            },
            UnifiedResult {
                signal: None,
                uuid: "u2".into(),
                score: 0.5,
                entity: None,
                data: None,
                chunk: None,
                chunks: None,
                relation: None,
                matched_children: None,
                other_children: None,
                graph: None,
            },
        ]);
        assert_eq!(summarize_port_value(&val), "Results(2)");
    }

    #[test]
    fn summarize_empty() {
        assert_eq!(summarize_port_value(&PortValue::Trigger), "Trigger");
    }

    #[test]
    fn report_from_events_completed() {
        use crate::dataflow::port::PortType;
        let events = vec![
            DataflowEvent::NodeStarted {
                node: "a".into(),
                node_type: "TestNode".into(),
                inputs: vec![],
            },
            DataflowEvent::NodeCompleted {
                node: "a".into(),
                duration_ms: 42,
                outputs: vec![PortSnapshot {
                    name: "out".into(),
                    port_type: PortType::Empty,
                    count: None,
                    data_json: None,
                }],
                metrics: std::collections::HashMap::new(),
            },
            DataflowEvent::Completed {
                total_nodes: 1,
                duration_ms: 50,
            },
        ];

        let graph = DataflowGraph::new();
        let output = DataflowOutput::empty();

        let report = ExecutionReport::build(&events, &graph, &output);
        assert_eq!(report.nodes.len(), 1);
        assert_eq!(report.nodes[0].name, "a");
        assert_eq!(report.nodes[0].node_type, "TestNode");
        assert_eq!(report.nodes[0].duration_ms, 42);
        assert_eq!(report.nodes[0].outputs.len(), 1);
        assert_eq!(report.nodes[0].outputs[0].name, "out");
        assert_eq!(report.total_duration_ms, 50);
        assert!(matches!(report.status, ExecutionStatus::Completed));
    }

    #[test]
    fn report_from_events_failed() {
        let events = vec![
            DataflowEvent::NodeStarted {
                node: "bad".into(),
                node_type: "BadNode".into(),
                inputs: vec![],
            },
            DataflowEvent::NodeFailed {
                node: "bad".into(),
                error: "boom".into(),
            },
            DataflowEvent::Failed {
                error: "boom".into(),
            },
        ];

        let graph = DataflowGraph::new();
        let output = DataflowOutput::empty();

        let report = ExecutionReport::build(&events, &graph, &output);
        assert_eq!(report.nodes.len(), 1);
        assert!(matches!(
            report.nodes[0].status,
            NodeStatus::Failed { .. }
        ));
        assert!(matches!(
            report.status,
            ExecutionStatus::Failed { .. }
        ));
    }

    #[test]
    fn report_includes_resumed_nodes() {
        use crate::dataflow::port::PortType;
        let events = vec![
            DataflowEvent::CheckpointResumed {
                node: "inserts".into(),
                output_ports: vec!["done".into(), "inserted".into()],
            },
            DataflowEvent::NodeStarted {
                node: "links".into(),
                node_type: "LinkRecordNode".into(),
                inputs: vec![],
            },
            DataflowEvent::NodeCompleted {
                node: "links".into(),
                duration_ms: 15,
                outputs: vec![PortSnapshot {
                    name: "done".into(),
                    port_type: PortType::Empty,
                    count: None,
                    data_json: None,
                }],
                metrics: std::collections::HashMap::new(),
            },
            DataflowEvent::Completed {
                total_nodes: 2,
                duration_ms: 20,
            },
        ];

        let graph = DataflowGraph::new();
        let output = DataflowOutput::empty();
        let report = ExecutionReport::build(&events, &graph, &output);

        assert_eq!(report.nodes.len(), 2);
        assert_eq!(report.nodes[0].name, "inserts");
        assert!(matches!(report.nodes[0].status, NodeStatus::Resumed));
        assert_eq!(report.nodes[0].duration_ms, 0);
        assert_eq!(report.nodes[0].outputs.len(), 2);
        assert_eq!(report.nodes[1].name, "links");
        assert!(matches!(report.nodes[1].status, NodeStatus::Completed));
    }

    #[test]
    fn report_serializes() {
        use crate::dataflow::port::PortType;
        let report = ExecutionReport {
            nodes: vec![NodeReport {
                name: "test".into(),
                node_type: "TestNode".into(),
                status: NodeStatus::Completed,
                duration_ms: 10,
                inputs: vec![],
                outputs: vec![PortSnapshot {
                    name: "out".into(),
                    port_type: PortType::Empty,
                    count: None,
                    data_json: None,
                }],
                logs: vec![],
                metrics: std::collections::HashMap::new(),
            }],
            edges: vec![],
            expanded_nodes: vec![],
            total_duration_ms: 15,
            status: ExecutionStatus::Completed,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Completed"));
    }
}

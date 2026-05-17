//! Dataflow recorder: persists execution reports to rag3db or JSONL.
//!
//! Writes `_DataflowExecution` → `_DataflowNodeRun` / `_DataflowEdgeRun`
//! nodes in rag3db, or falls back to JSONL files for lightweight recording.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;

use crate::connection::{DbConnection, DbError, QueryParam};
use super::report::ExecutionReport;

// ─── RecordSink ─────────────────────────────────────────────────────────────

/// Where to persist execution records.
pub enum RecordSink {
    /// Write to rag3db via DbConnection.
    Database(Arc<dyn DbConnection>),
    /// Write to a JSONL file.
    File(PathBuf),
    /// Write to both.
    Both(Arc<dyn DbConnection>, PathBuf),
    /// Don't record.
    None,
}

// ─── RecordRetention ────────────────────────────────────────────────────────

/// Retention policy for execution records.
#[derive(Debug, Clone, Serialize)]
pub struct RecordRetention {
    /// Maximum records per pipeline name. Oldest are deleted first.
    pub max_per_pipeline: Option<usize>,
    /// Maximum age in days. Older records are deleted.
    pub max_age_days: Option<u32>,
    /// Keep failed executions even if they exceed limits.
    pub keep_errors: bool,
}

impl Default for RecordRetention {
    fn default() -> Self {
        Self {
            max_per_pipeline: Some(100),
            max_age_days: Some(30),
            keep_errors: true,
        }
    }
}

// ─── DataflowRecorder ───────────────────────────────────────────────────────

/// Records execution reports to the configured sink.
pub struct DataflowRecorder {
    sink: RecordSink,
    retention: RecordRetention,
}

impl DataflowRecorder {
    pub fn new(sink: RecordSink) -> Self {
        Self {
            sink,
            retention: RecordRetention::default(),
        }
    }

    pub fn with_retention(mut self, retention: RecordRetention) -> Self {
        self.retention = retention;
        self
    }

    /// Record an execution report.
    pub fn record(
        &self,
        pipeline_name: &str,
        report: &ExecutionReport,
    ) -> Result<(), RecordError> {
        match &self.sink {
            RecordSink::Database(conn) => {
                self.record_to_db(conn, pipeline_name, report)?;
            }
            RecordSink::File(path) => {
                self.record_to_jsonl(path, pipeline_name, report)?;
            }
            RecordSink::Both(conn, path) => {
                // Try DB first, JSONL as fallback on DB error
                if let Err(e) = self.record_to_db(conn, pipeline_name, report) {
                    eprintln!("DB record failed ({}), falling back to JSONL", e);
                    self.record_to_jsonl(path, pipeline_name, report)?;
                } else {
                    self.record_to_jsonl(path, pipeline_name, report)?;
                }
            }
            RecordSink::None => {}
        }
        Ok(())
    }

    /// Ensure the dataflow recording schema exists (node tables + rel tables).
    fn ensure_schema(conn: &Arc<dyn DbConnection>) -> Result<(), RecordError> {
        let stmts = [
            "CREATE NODE TABLE IF NOT EXISTS _DataflowExecution(\
                _uuid STRING, pipeline_name STRING, status STRING, \
                graph_json STRING, graph_hash STRING, \
                node_count INT64, edge_count INT64, expanded_count INT64, \
                inputs_json STRING, error STRING, \
                duration_ms INT64, created_at INT64, updated_at INT64, \
                PRIMARY KEY(_uuid))",
            "CREATE NODE TABLE IF NOT EXISTS _DataflowNodeRun(\
                _uuid STRING, node_name STRING, status STRING, \
                duration_ms INT64, output_ports STRING, PRIMARY KEY(_uuid))",
            "CREATE NODE TABLE IF NOT EXISTS _DataflowEdgeRun(\
                _uuid STRING, from_node STRING, from_port STRING, \
                to_node STRING, to_port STRING, value_summary STRING, \
                PRIMARY KEY(_uuid))",
            "CREATE REL TABLE IF NOT EXISTS _NodeRunOf(\
                FROM _DataflowNodeRun TO _DataflowExecution)",
            "CREATE REL TABLE IF NOT EXISTS _EdgeRunOf(\
                FROM _DataflowEdgeRun TO _DataflowExecution)",
        ];
        for stmt in &stmts {
            conn.execute(stmt).map_err(RecordError::Db)?;
        }
        Ok(())
    }

    /// Write a single Cypher batch that creates the execution + node runs + edge runs.
    fn record_to_db(
        &self,
        conn: &Arc<dyn DbConnection>,
        pipeline_name: &str,
        report: &ExecutionReport,
    ) -> Result<(), RecordError> {
        Self::ensure_schema(conn)?;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let exec_uuid = crate::uuid::hashsafe_uuid(
            "_DataflowExecution",
            &[pipeline_name, &now_ms.to_string()],
        );

        let status = match &report.status {
            super::report::ExecutionStatus::Completed => "completed",
            super::report::ExecutionStatus::Failed { .. } => "failed",
        };

        // Create _DataflowExecution node
        let create_exec = format!(
            "CREATE (e:_DataflowExecution {{_uuid: $uuid, pipeline_name: $pipeline, \
             status: $status, duration_ms: $duration, node_count: $nodes, \
             edge_count: $edges, expanded_count: $expanded, created_at: $created_at}})",
        );
        conn.execute_with_params(
            &create_exec,
            &[
                QueryParam::new("uuid", exec_uuid.as_str()),
                QueryParam::new("pipeline", pipeline_name),
                QueryParam::new("status", status),
                QueryParam::new("duration", report.total_duration_ms as i64),
                QueryParam::new("nodes", report.nodes.len() as i64),
                QueryParam::new("edges", report.edges.len() as i64),
                QueryParam::new("expanded", report.expanded_nodes.len() as i64),
                QueryParam::new("created_at", now_ms),
            ],
        )
        .map_err(RecordError::Db)?;

        // Create _DataflowNodeRun nodes + link to execution
        for (i, node) in report.nodes.iter().enumerate() {
            let node_status = match &node.status {
                super::report::NodeStatus::Completed => "completed".to_string(),
                super::report::NodeStatus::Failed { error } => {
                    format!("failed: {}", error)
                }
                super::report::NodeStatus::Resumed => "resumed".to_string(),
            };
            let node_uuid = format!("{}__node_{}", exec_uuid, i);
            let cypher = "MATCH (e:_DataflowExecution {_uuid: $exec_uuid}) \
                          CREATE (n:_DataflowNodeRun {_uuid: $uuid, node_name: $name, \
                          status: $status, duration_ms: $duration, \
                          output_ports: $ports})-[:_NodeRunOf]->(e)";
            conn.execute_with_params(
                cypher,
                &[
                    QueryParam::new("exec_uuid", exec_uuid.as_str()),
                    QueryParam::new("uuid", node_uuid.as_str()),
                    QueryParam::new("name", node.name.as_str()),
                    QueryParam::new("status", node_status.as_str()),
                    QueryParam::new("duration", node.duration_ms as i64),
                    QueryParam::new("ports", node.outputs.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(",")),
                ],
            )
            .map_err(RecordError::Db)?;
        }

        // Create _DataflowEdgeRun nodes + link to execution
        for (i, edge) in report.edges.iter().enumerate() {
            let edge_uuid = format!("{}__edge_{}", exec_uuid, i);
            let cypher = "MATCH (e:_DataflowExecution {_uuid: $exec_uuid}) \
                          CREATE (r:_DataflowEdgeRun {_uuid: $uuid, from_node: $from_node, \
                          from_port: $from_port, to_node: $to_node, to_port: $to_port, \
                          value_summary: $summary})-[:_EdgeRunOf]->(e)";
            conn.execute_with_params(
                cypher,
                &[
                    QueryParam::new("exec_uuid", exec_uuid.as_str()),
                    QueryParam::new("uuid", edge_uuid.as_str()),
                    QueryParam::new("from_node", edge.from_node.as_str()),
                    QueryParam::new("from_port", edge.from_port.as_str()),
                    QueryParam::new("to_node", edge.to_node.as_str()),
                    QueryParam::new("to_port", edge.to_port.as_str()),
                    QueryParam::new("summary", edge.value_summary.as_str()),
                ],
            )
            .map_err(RecordError::Db)?;
        }

        // Apply retention if configured
        self.apply_retention_db(conn, pipeline_name)?;

        Ok(())
    }

    /// Append a JSONL line to a file.
    fn record_to_jsonl(
        &self,
        path: &PathBuf,
        pipeline_name: &str,
        report: &ExecutionReport,
    ) -> Result<(), RecordError> {
        use std::io::Write;

        #[derive(Serialize)]
        struct JsonlRecord<'a> {
            pipeline: &'a str,
            report: &'a ExecutionReport,
        }

        let record = JsonlRecord {
            pipeline: pipeline_name,
            report,
        };
        let line = serde_json::to_string(&record).map_err(RecordError::Serialize)?;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(RecordError::Io)?;
        writeln!(file, "{}", line).map_err(RecordError::Io)?;

        Ok(())
    }

    /// Delete old records beyond retention limits.
    fn apply_retention_db(
        &self,
        conn: &Arc<dyn DbConnection>,
        pipeline_name: &str,
    ) -> Result<(), RecordError> {
        if let Some(max) = self.retention.max_per_pipeline {
            // Count executions for this pipeline
            let count_cypher =
                "MATCH (e:_DataflowExecution {pipeline_name: $pipeline}) RETURN count(e) AS cnt";
            let result = conn
                .execute_with_params(
                    count_cypher,
                    &[QueryParam::new("pipeline", pipeline_name)],
                )
                .map_err(RecordError::Db)?;

            if let Some(row) = result.rows.first() {
                if let Some(cnt) = row.first().and_then(|v| v.as_i64()) {
                    if cnt as usize > max {
                        let excess = cnt as usize - max;
                        // Delete oldest (keep errors if configured)
                        let delete_cypher = if self.retention.keep_errors {
                            format!(
                                "MATCH (e:_DataflowExecution {{pipeline_name: $pipeline}}) \
                                 WHERE e.status = 'completed' \
                                 WITH e ORDER BY e._uuid LIMIT {} \
                                 DETACH DELETE e",
                                excess
                            )
                        } else {
                            format!(
                                "MATCH (e:_DataflowExecution {{pipeline_name: $pipeline}}) \
                                 WITH e ORDER BY e._uuid LIMIT {} \
                                 DETACH DELETE e",
                                excess
                            )
                        };
                        conn.execute_with_params(
                            &delete_cypher,
                            &[QueryParam::new("pipeline", pipeline_name)],
                        )
                        .map_err(RecordError::Db)?;
                    }
                }
            }
        }
        Ok(())
    }
}

// ─── RecordError ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum RecordError {
    Db(DbError),
    Io(std::io::Error),
    Serialize(serde_json::Error),
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Db(e) => write!(f, "database error: {e}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Serialize(e) => write!(f, "serialization error: {e}"),
        }
    }
}

impl std::error::Error for RecordError {}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::port::PortType;
    use crate::dataflow::report::{
        EdgeReport, ExecutionReport, ExecutionStatus, NodeReport, NodeStatus,
    };
    use crate::dataflow::runtime::PortSnapshot;

    fn make_output_snapshots(names: &[&str]) -> Vec<PortSnapshot> {
        names.iter().map(|n| PortSnapshot {
            name: n.to_string(),
            port_type: PortType::Empty,
            count: None,
            data_json: None,
        }).collect()
    }

    fn sample_report() -> ExecutionReport {
        ExecutionReport {
            nodes: vec![
                NodeReport {
                    name: "primary_search".into(),
                    node_type: "SearchNode".into(),
                    status: NodeStatus::Completed,
                    duration_ms: 42,
                    inputs: vec![],
                    outputs: make_output_snapshots(&["results", "meta"]),
                    logs: vec![],
                    metrics: std::collections::HashMap::new(),
                },
                NodeReport {
                    name: "compose".into(),
                    node_type: "ComposeNode".into(),
                    status: NodeStatus::Completed,
                    duration_ms: 5,
                    inputs: vec![],
                    outputs: make_output_snapshots(&["results"]),
                    logs: vec![],
                    metrics: std::collections::HashMap::new(),
                },
            ],
            edges: vec![EdgeReport {
                from_node: "primary_search".into(),
                from_port: "results".into(),
                to_node: "compose".into(),
                to_port: "results".into(),
                value_summary: "Results(3)".into(),
            }],
            expanded_nodes: vec!["fetch_0".into()],
            total_duration_ms: 50,
            status: ExecutionStatus::Completed,
        }
    }

    #[test]
    fn jsonl_roundtrip() {
        let dir = std::env::temp_dir().join("dataflow_test_jsonl");
        let _ = std::fs::remove_file(&dir);

        let recorder = DataflowRecorder::new(RecordSink::File(dir.clone()));
        let report = sample_report();

        recorder.record("test_pipeline", &report).unwrap();

        let content = std::fs::read_to_string(&dir).unwrap();
        assert!(content.contains("test_pipeline"));
        assert!(content.contains("primary_search"));
        assert!(content.contains("Results(3)"));
        assert!(content.ends_with('\n'));

        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn record_sink_none_is_noop() {
        let recorder = DataflowRecorder::new(RecordSink::None);
        let report = sample_report();

        recorder.record("test", &report).unwrap();
    }

    #[test]
    fn record_to_mock_db() {
        use crate::connection::MockConnection;

        let conn = Arc::new(MockConnection::new());
        let recorder = DataflowRecorder::new(RecordSink::Database(conn));
        let report = sample_report();

        // MockConnection returns empty results, but the recording shouldn't error
        recorder.record("test_pipeline", &report).unwrap();
    }

    #[test]
    fn retention_default() {
        let r = RecordRetention::default();
        assert_eq!(r.max_per_pipeline, Some(100));
        assert_eq!(r.max_age_days, Some(30));
        assert!(r.keep_errors);
    }
}

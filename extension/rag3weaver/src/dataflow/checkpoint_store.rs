//! Cypher-backed checkpoint store for dataflow execution persistence.
//!
//! Stores execution state in two node tables:
//! - `_DataflowExecution` — one row per execution (graph hash, status, timestamps)
//! - `_DataflowNodeState` — one row per node (outputs JSON, status, timing)

use std::collections::HashMap;
use std::sync::Arc;


use crate::connection::{CypherValue, DbConnection, QueryParam};
use super::checkpoint::{
    CheckpointPortValue, CheckpointStore, ExecutionCheckpoint, CheckpointExecutionStatus,
    GraphDefinition, NodeCheckpoint, NodeCheckpointStatus, timestamp_ms,
};

// ─── CypherCheckpointStore ──────────────────────────────────────────────────

/// Checkpoint store backed by Cypher queries against the graph database.
///
/// Uses MERGE for upserts (idempotent) and parameterized queries for JSON data.
pub struct CypherCheckpointStore {
    conn: Arc<dyn DbConnection>,
}

impl CypherCheckpointStore {
    pub fn new(conn: Arc<dyn DbConnection>) -> Self {
        Self { conn }
    }
}


impl CheckpointStore for CypherCheckpointStore {
    fn initialize(&self) -> Result<(), String> {
        self.conn
            .execute(
                "CREATE NODE TABLE IF NOT EXISTS _DataflowExecution(\
                     _uuid STRING, \
                     pipeline_name STRING, \
                     status STRING, \
                     graph_json STRING, \
                     graph_hash STRING, \
                     node_count INT64, \
                     edge_count INT64, \
                     expanded_count INT64, \
                     inputs_json STRING, \
                     error STRING, \
                     duration_ms INT64, \
                     created_at INT64, \
                     updated_at INT64, \
                     PRIMARY KEY(_uuid))",
            )
            .map_err(|e| e.to_string())?;

        self.conn
            .execute(
                "CREATE NODE TABLE IF NOT EXISTS _DataflowNodeState(\
                     _uuid STRING, \
                     execution_id STRING, \
                     node_name STRING, \
                     status STRING, \
                     output_ports STRING, \
                     undo_json STRING, \
                     duration_ms INT64, \
                     error STRING, \
                     completed_at INT64, \
                     PRIMARY KEY(_uuid))",
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn create_execution(&self, checkpoint: &ExecutionCheckpoint) -> Result<(), String> {
        let graph_json =
            serde_json::to_string(&checkpoint.graph_def).map_err(|e| e.to_string())?;
        let inputs_json =
            serde_json::to_string(&checkpoint.initial_inputs).map_err(|e| e.to_string())?;
        let now = timestamp_ms();

        self.conn
            .execute_with_params(
                "MERGE (n:_DataflowExecution {_uuid: $exec_id}) \
                 SET n.status = $status, \
                     n.graph_json = $graph_json, \
                     n.graph_hash = $graph_hash, \
                     n.node_count = $node_count, \
                     n.inputs_json = $inputs_json, \
                     n.error = $error, \
                     n.created_at = $created_at, \
                     n.updated_at = $updated_at",
                &[
                    QueryParam::new("exec_id", CypherValue::String(checkpoint.execution_id.clone())),
                    QueryParam::new("status", CypherValue::String(status_to_string(&checkpoint.status))),
                    QueryParam::new("graph_json", CypherValue::String(graph_json)),
                    QueryParam::new("graph_hash", CypherValue::String(checkpoint.graph_hash.clone())),
                    QueryParam::new("node_count", CypherValue::Int(checkpoint.nodes.len() as i64)),
                    QueryParam::new("inputs_json", CypherValue::String(inputs_json)),
                    QueryParam::new("error", CypherValue::String(String::new())),
                    QueryParam::new("created_at", CypherValue::Int(now as i64)),
                    QueryParam::new("updated_at", CypherValue::Int(now as i64)),
                ],
            )
            .map_err(|e| e.to_string())?;

        // Create pending node state rows for all nodes
        for (node_name, node_cp) in &checkpoint.nodes {
            let uuid = format!("{}:{}", checkpoint.execution_id, node_name);
            self.conn
                .execute_with_params(
                    "MERGE (n:_DataflowNodeState {_uuid: $uuid}) \
                     SET n.execution_id = $exec_id, \
                         n.node_name = $node_name, \
                         n.status = $status, \
                         n.output_ports = $output_ports, \
                         n.duration_ms = $duration_ms, \
                         n.error = $error, \
                         n.completed_at = $completed_at",
                    &[
                        QueryParam::new("uuid", CypherValue::String(uuid)),
                        QueryParam::new("exec_id", CypherValue::String(checkpoint.execution_id.clone())),
                        QueryParam::new("node_name", CypherValue::String(node_name.clone())),
                        QueryParam::new("status", CypherValue::String(
                            node_status_to_string(&node_cp.status),
                        )),
                        QueryParam::new("output_ports", CypherValue::String(String::new())),
                        QueryParam::new("duration_ms", CypherValue::Int(0)),
                        QueryParam::new("error", CypherValue::String(String::new())),
                        QueryParam::new("completed_at", CypherValue::Int(0)),
                    ],
                )
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    fn load_execution(
        &self,
        execution_id: &str,
    ) -> Result<Option<ExecutionCheckpoint>, String> {
        // Load execution header
        let result = self
            .conn
            .execute_with_params(
                "MATCH (n:_DataflowExecution {_uuid: $exec_id}) \
                 RETURN n.status, n.graph_json, n.graph_hash, n.node_count, \
                        n.error, n.created_at, n.updated_at, n.inputs_json",
                &[QueryParam::new("exec_id", CypherValue::String(execution_id.to_string()))],
            )
            .map_err(|e| e.to_string())?;

        if result.rows.is_empty() {
            return Ok(None);
        }

        let row = &result.rows[0];
        let status = string_to_status(row[0].as_str().unwrap_or("running"));
        let graph_json = row[1].as_str().unwrap_or("{}");
        let graph_hash = row[2].as_str().unwrap_or("").to_string();
        let _node_count = row[3].as_i64().unwrap_or(0);
        let _error = row[4].as_str().unwrap_or("");
        let created_at = row[5].as_i64().unwrap_or(0) as u64;
        let updated_at = row[6].as_i64().unwrap_or(0) as u64;

        let graph_def: GraphDefinition =
            serde_json::from_str(graph_json).map_err(|e| e.to_string())?;

        let inputs_json = row.get(7).and_then(|v| v.as_str()).unwrap_or("{}");
        let initial_inputs: HashMap<String, HashMap<String, CheckpointPortValue>> =
            serde_json::from_str(inputs_json).unwrap_or_default();

        // Load node states
        let nodes_result = self
            .conn
            .execute_with_params(
                "MATCH (n:_DataflowNodeState {execution_id: $exec_id}) \
                 RETURN n.node_name, n.status, n.output_ports, n.duration_ms, \
                        n.error, n.completed_at, n.undo_json",
                &[QueryParam::new("exec_id", CypherValue::String(execution_id.to_string()))],
            )
            .map_err(|e| e.to_string())?;

        let mut nodes = HashMap::new();
        for row in &nodes_result.rows {
            let node_name = row[0].as_str().unwrap_or("").to_string();
            let node_status = string_to_node_status(row[1].as_str().unwrap_or("pending"));
            let output_ports_json = row[2].as_str().unwrap_or("{}");
            let duration_ms = row[3].as_i64().unwrap_or(0) as u64;
            let error_str = row[4].as_str().unwrap_or("").to_string();
            let completed_at = row[5].as_i64().unwrap_or(0) as u64;
            let undo_json_str = row.get(6).and_then(|v| v.as_str()).unwrap_or("");

            let output_ports: HashMap<String, CheckpointPortValue> = if output_ports_json.is_empty()
            {
                HashMap::new()
            } else {
                serde_json::from_str(output_ports_json).unwrap_or_default()
            };

            let undo_context = if undo_json_str.is_empty() {
                None
            } else {
                serde_json::from_str(undo_json_str).ok()
            };

            nodes.insert(
                node_name,
                NodeCheckpoint {
                    status: node_status,
                    output_ports,
                    undo_context,
                    duration_ms: if duration_ms > 0 {
                        Some(duration_ms)
                    } else {
                        None
                    },
                    error: if error_str.is_empty() {
                        None
                    } else {
                        Some(error_str)
                    },
                    completed_at: if completed_at > 0 {
                        Some(completed_at)
                    } else {
                        None
                    },
                },
            );
        }

        Ok(Some(ExecutionCheckpoint {
            execution_id: execution_id.to_string(),
            status,
            graph_def,
            graph_hash,
            nodes,
            initial_inputs,
            created_at,
            updated_at,
        }))
    }

    fn find_incomplete(&self) -> Result<Vec<String>, String> {
        let result = self
            .conn
            .execute(
                "MATCH (n:_DataflowExecution) \
                 WHERE n.status = 'running' OR n.status = 'failed' \
                 RETURN n._uuid ORDER BY n.created_at",
            )
            .map_err(|e| e.to_string())?;

        Ok(result
            .rows
            .iter()
            .filter_map(|row| row[0].as_str().map(|s| s.to_string()))
            .collect())
    }

    fn save_node_completed(
        &self,
        execution_id: &str,
        node_name: &str,
        outputs: &HashMap<String, CheckpointPortValue>,
        undo_context: Option<&serde_json::Value>,
        duration_ms: u64,
    ) -> Result<(), String> {
        let uuid = format!("{execution_id}:{node_name}");
        let now = timestamp_ms();
        let outputs_json = serde_json::to_string(outputs).map_err(|e| e.to_string())?;
        let undo_json = match undo_context {
            Some(ctx) => serde_json::to_string(ctx).map_err(|e| e.to_string())?,
            None => String::new(),
        };

        self.conn
            .execute_with_params(
                "MERGE (n:_DataflowNodeState {_uuid: $uuid}) \
                 SET n.status = $status, \
                     n.output_ports = $output_ports, \
                     n.undo_json = $undo_json, \
                     n.duration_ms = $duration_ms, \
                     n.completed_at = $completed_at",
                &[
                    QueryParam::new("uuid", CypherValue::String(uuid)),
                    QueryParam::new("status", CypherValue::String("completed".to_string())),
                    QueryParam::new("output_ports", CypherValue::String(outputs_json)),
                    QueryParam::new("undo_json", CypherValue::String(undo_json)),
                    QueryParam::new("duration_ms", CypherValue::Int(duration_ms as i64)),
                    QueryParam::new("completed_at", CypherValue::Int(now as i64)),
                ],
            )
            .map_err(|e| e.to_string())?;

        // Update execution updated_at
        self.conn
            .execute_with_params(
                "MATCH (n:_DataflowExecution {_uuid: $exec_id}) \
                 SET n.updated_at = $now",
                &[
                    QueryParam::new("exec_id", CypherValue::String(execution_id.to_string())),
                    QueryParam::new("now", CypherValue::Int(now as i64)),
                ],
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn save_node_failed(
        &self,
        execution_id: &str,
        node_name: &str,
        error: &str,
    ) -> Result<(), String> {
        let uuid = format!("{execution_id}:{node_name}");
        let now = timestamp_ms();

        self.conn
            .execute_with_params(
                "MERGE (n:_DataflowNodeState {_uuid: $uuid}) \
                 SET n.status = $status, \
                     n.error = $error, \
                     n.completed_at = $completed_at",
                &[
                    QueryParam::new("uuid", CypherValue::String(uuid)),
                    QueryParam::new("status", CypherValue::String("failed".to_string())),
                    QueryParam::new("error", CypherValue::String(error.to_string())),
                    QueryParam::new("completed_at", CypherValue::Int(now as i64)),
                ],
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn mark_completed(&self, execution_id: &str) -> Result<(), String> {
        let now = timestamp_ms();

        // Update execution status
        self.conn
            .execute_with_params(
                "MATCH (n:_DataflowExecution {_uuid: $exec_id}) \
                 SET n.status = $status, n.updated_at = $now",
                &[
                    QueryParam::new("exec_id", CypherValue::String(execution_id.to_string())),
                    QueryParam::new("status", CypherValue::String("completed".to_string())),
                    QueryParam::new("now", CypherValue::Int(now as i64)),
                ],
            )
            .map_err(|e| e.to_string())?;

        // Clean up node state rows:
        // - Delete rows without undo context (no longer needed)
        // - Keep rows with undo context but clear output_ports (for rollback)
        self.conn
            .execute_with_params(
                "MATCH (n:_DataflowNodeState {execution_id: $exec_id}) \
                 WHERE n.undo_json = '' OR n.undo_json IS NULL \
                 DELETE n",
                &[QueryParam::new("exec_id", CypherValue::String(execution_id.to_string()))],
            )
            .map_err(|e| e.to_string())?;

        // Clear bulky output_ports from surviving undo nodes
        self.conn
            .execute_with_params(
                "MATCH (n:_DataflowNodeState {execution_id: $exec_id}) \
                 SET n.output_ports = ''",
                &[QueryParam::new("exec_id", CypherValue::String(execution_id.to_string()))],
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn mark_failed(&self, execution_id: &str, error: &str) -> Result<(), String> {
        let now = timestamp_ms();

        self.conn
            .execute_with_params(
                "MATCH (n:_DataflowExecution {_uuid: $exec_id}) \
                 SET n.status = $status, n.error = $error, n.updated_at = $now",
                &[
                    QueryParam::new("exec_id", CypherValue::String(execution_id.to_string())),
                    QueryParam::new("status", CypherValue::String("failed".to_string())),
                    QueryParam::new("error", CypherValue::String(error.to_string())),
                    QueryParam::new("now", CypherValue::Int(now as i64)),
                ],
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn delete(&self, execution_id: &str) -> Result<(), String> {
        // Delete node states first
        self.conn
            .execute_with_params(
                "MATCH (n:_DataflowNodeState {execution_id: $exec_id}) DELETE n",
                &[QueryParam::new("exec_id", CypherValue::String(execution_id.to_string()))],
            )
            .map_err(|e| e.to_string())?;

        // Delete execution
        self.conn
            .execute_with_params(
                "MATCH (n:_DataflowExecution {_uuid: $exec_id}) DELETE n",
                &[QueryParam::new("exec_id", CypherValue::String(execution_id.to_string()))],
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

// ─── Status helpers ──────────────────────────────────────────────────────────

fn status_to_string(s: &CheckpointExecutionStatus) -> String {
    match s {
        CheckpointExecutionStatus::Running => "running".to_string(),
        CheckpointExecutionStatus::Completed => "completed".to_string(),
        CheckpointExecutionStatus::Failed => "failed".to_string(),
    }
}

fn string_to_status(s: &str) -> CheckpointExecutionStatus {
    match s {
        "completed" => CheckpointExecutionStatus::Completed,
        "failed" => CheckpointExecutionStatus::Failed,
        _ => CheckpointExecutionStatus::Running,
    }
}

fn node_status_to_string(s: &NodeCheckpointStatus) -> String {
    match s {
        NodeCheckpointStatus::Pending => "pending".to_string(),
        NodeCheckpointStatus::Completed => "completed".to_string(),
        NodeCheckpointStatus::Failed => "failed".to_string(),
    }
}

fn string_to_node_status(s: &str) -> NodeCheckpointStatus {
    match s {
        "completed" => NodeCheckpointStatus::Completed,
        "failed" => NodeCheckpointStatus::Failed,
        _ => NodeCheckpointStatus::Pending,
    }
}

// ─── MockCheckpointStore ─────────────────────────────────────────────────────

/// In-memory checkpoint store for unit testing.
/// Implements the full CheckpointStore trait using HashMaps.
#[cfg(test)]
pub struct MockCheckpointStore {
    executions: std::sync::Mutex<HashMap<String, ExecutionCheckpoint>>,
}

#[cfg(test)]
impl MockCheckpointStore {
    pub fn new() -> Self {
        Self {
            executions: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Mutate a stored checkpoint (for test setup, e.g., resetting status for resume).
    pub fn mutate(&self, execution_id: &str, f: impl FnOnce(&mut ExecutionCheckpoint)) {
        let mut execs = self.executions.lock().unwrap();
        if let Some(cp) = execs.get_mut(execution_id) {
            f(cp);
        }
    }

    /// Access all stored executions (for test inspection).
    pub fn mutate_all(&self, f: impl FnOnce(&HashMap<String, ExecutionCheckpoint>)) {
        let execs = self.executions.lock().unwrap();
        f(&execs);
    }
}

#[cfg(test)]

impl CheckpointStore for MockCheckpointStore {
    fn initialize(&self) -> Result<(), String> {
        Ok(())
    }

    fn create_execution(&self, checkpoint: &ExecutionCheckpoint) -> Result<(), String> {
        let mut execs = self.executions.lock().unwrap();
        execs.insert(checkpoint.execution_id.clone(), checkpoint.clone());
        Ok(())
    }

    fn load_execution(
        &self,
        execution_id: &str,
    ) -> Result<Option<ExecutionCheckpoint>, String> {
        let execs = self.executions.lock().unwrap();
        Ok(execs.get(execution_id).cloned())
    }

    fn find_incomplete(&self) -> Result<Vec<String>, String> {
        let execs = self.executions.lock().unwrap();
        Ok(execs
            .values()
            .filter(|cp| {
                cp.status == CheckpointExecutionStatus::Running
                    || cp.status == CheckpointExecutionStatus::Failed
            })
            .map(|cp| cp.execution_id.clone())
            .collect())
    }

    fn save_node_completed(
        &self,
        execution_id: &str,
        node_name: &str,
        outputs: &HashMap<String, CheckpointPortValue>,
        undo_context: Option<&serde_json::Value>,
        duration_ms: u64,
    ) -> Result<(), String> {
        let mut execs = self.executions.lock().unwrap();
        let cp = execs
            .get_mut(execution_id)
            .ok_or("execution not found")?;
        let node = cp
            .nodes
            .get_mut(node_name)
            .ok_or("node not found")?;
        node.status = NodeCheckpointStatus::Completed;
        node.output_ports = outputs.clone();
        node.undo_context = undo_context.cloned();
        node.duration_ms = Some(duration_ms);
        node.completed_at = Some(timestamp_ms());
        cp.updated_at = timestamp_ms();
        Ok(())
    }

    fn save_node_failed(
        &self,
        execution_id: &str,
        node_name: &str,
        error: &str,
    ) -> Result<(), String> {
        let mut execs = self.executions.lock().unwrap();
        let cp = execs
            .get_mut(execution_id)
            .ok_or("execution not found")?;
        let node = cp
            .nodes
            .get_mut(node_name)
            .ok_or("node not found")?;
        node.status = NodeCheckpointStatus::Failed;
        node.error = Some(error.to_string());
        node.completed_at = Some(timestamp_ms());
        Ok(())
    }

    fn mark_completed(&self, execution_id: &str) -> Result<(), String> {
        let mut execs = self.executions.lock().unwrap();
        let cp = execs
            .get_mut(execution_id)
            .ok_or("execution not found")?;
        cp.status = CheckpointExecutionStatus::Completed;
        cp.updated_at = timestamp_ms();
        // Clean up node data (like the real store)
        for node in cp.nodes.values_mut() {
            node.output_ports.clear();
        }
        Ok(())
    }

    fn mark_failed(&self, execution_id: &str, _error: &str) -> Result<(), String> {
        let mut execs = self.executions.lock().unwrap();
        let cp = execs
            .get_mut(execution_id)
            .ok_or("execution not found")?;
        cp.status = CheckpointExecutionStatus::Failed;
        cp.updated_at = timestamp_ms();
        Ok(())
    }

    fn delete(&self, execution_id: &str) -> Result<(), String> {
        let mut execs = self.executions.lock().unwrap();
        execs.remove(execution_id);
        Ok(())
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper: build a test checkpoint ─────────────────────────────────

    fn make_test_checkpoint(exec_id: &str) -> ExecutionCheckpoint {
        let now = timestamp_ms();
        let mut nodes = HashMap::new();
        nodes.insert(
            "inserts".to_string(),
            NodeCheckpoint {
                status: NodeCheckpointStatus::Pending,
                output_ports: HashMap::new(),
                undo_context: None,
                duration_ms: None,
                error: None,
                completed_at: None,
            },
        );
        nodes.insert(
            "links".to_string(),
            NodeCheckpoint {
                status: NodeCheckpointStatus::Pending,
                output_ports: HashMap::new(),
                undo_context: None,
                duration_ms: None,
                error: None,
                completed_at: None,
            },
        );
        ExecutionCheckpoint {
            execution_id: exec_id.to_string(),
            status: CheckpointExecutionStatus::Running,
            graph_def: GraphDefinition {
                nodes: vec![],
                edges: vec![],
            },
            graph_hash: "abc123".to_string(),
            nodes,
            initial_inputs: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    // ── Tests ───────────────────────────────────────────────────────────

    #[test]
    fn mock_store_create_and_load() {
        let store = MockCheckpointStore::new();
        store.initialize().unwrap();

        let cp = make_test_checkpoint("exec-1");
        store.create_execution(&cp).unwrap();

        let loaded = store.load_execution("exec-1").unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.execution_id, "exec-1");
        assert_eq!(loaded.status, CheckpointExecutionStatus::Running);
        assert_eq!(loaded.nodes.len(), 2);
        assert_eq!(loaded.graph_hash, "abc123");
    }

    #[test]
    fn mock_store_load_missing() {
        let store = MockCheckpointStore::new();
        let loaded = store.load_execution("nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn mock_store_node_completed() {
        let store = MockCheckpointStore::new();
        let cp = make_test_checkpoint("exec-2");
        store.create_execution(&cp).unwrap();

        let mut outputs = HashMap::new();
        outputs.insert(
            "done".to_string(),
            CheckpointPortValue {
                port_type: crate::dataflow::port::PortType::Empty,
                is_batch: false,
                data_json: None,
                record_count: None,
            },
        );
        store
            .save_node_completed("exec-2", "inserts", &outputs, None, 42)
            .unwrap();

        let loaded = store.load_execution("exec-2").unwrap().unwrap();
        let node = &loaded.nodes["inserts"];
        assert_eq!(node.status, NodeCheckpointStatus::Completed);
        assert_eq!(node.duration_ms, Some(42));
        assert!(node.output_ports.contains_key("done"));
    }

    #[test]
    fn mock_store_node_failed() {
        let store = MockCheckpointStore::new();
        let cp = make_test_checkpoint("exec-3");
        store.create_execution(&cp).unwrap();

        store
            .save_node_failed("exec-3", "links", "connection timeout")
            .unwrap();

        let loaded = store.load_execution("exec-3").unwrap().unwrap();
        let node = &loaded.nodes["links"];
        assert_eq!(node.status, NodeCheckpointStatus::Failed);
        assert_eq!(node.error.as_deref(), Some("connection timeout"));
    }

    #[test]
    fn mock_store_find_incomplete() {
        let store = MockCheckpointStore::new();

        store
            .create_execution(&make_test_checkpoint("exec-a"))
            .unwrap();
        store
            .create_execution(&make_test_checkpoint("exec-b"))
            .unwrap();
        store.mark_completed("exec-a").unwrap();

        let incomplete = store.find_incomplete().unwrap();
        assert_eq!(incomplete.len(), 1);
        assert!(incomplete.contains(&"exec-b".to_string()));
    }

    #[test]
    fn mock_store_mark_completed_cleans_outputs() {
        let store = MockCheckpointStore::new();
        let cp = make_test_checkpoint("exec-4");
        store.create_execution(&cp).unwrap();

        let mut outputs = HashMap::new();
        outputs.insert(
            "done".to_string(),
            CheckpointPortValue {
                port_type: crate::dataflow::port::PortType::Empty,
                is_batch: false,
                data_json: None,
                record_count: None,
            },
        );
        store
            .save_node_completed("exec-4", "inserts", &outputs, None, 10)
            .unwrap();

        store.mark_completed("exec-4").unwrap();

        let loaded = store.load_execution("exec-4").unwrap().unwrap();
        assert_eq!(loaded.status, CheckpointExecutionStatus::Completed);
        // Node outputs cleaned on success
        assert!(loaded.nodes["inserts"].output_ports.is_empty());
    }

    #[test]
    fn mock_store_mark_failed() {
        let store = MockCheckpointStore::new();
        let cp = make_test_checkpoint("exec-5");
        store.create_execution(&cp).unwrap();

        store
            .mark_failed("exec-5", "graph execution error")
            .unwrap();

        let loaded = store.load_execution("exec-5").unwrap().unwrap();
        assert_eq!(loaded.status, CheckpointExecutionStatus::Failed);
    }

    #[test]
    fn mock_store_delete() {
        let store = MockCheckpointStore::new();
        let cp = make_test_checkpoint("exec-6");
        store.create_execution(&cp).unwrap();

        store.delete("exec-6").unwrap();

        let loaded = store.load_execution("exec-6").unwrap();
        assert!(loaded.is_none());
    }
}

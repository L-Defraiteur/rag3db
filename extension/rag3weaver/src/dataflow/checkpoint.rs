//! Checkpoint types and serialization for dataflow execution persistence.
//!
//! Provides [`CheckpointPortValue`] — a fully serializable representation of
//! port data (including [`BatchPayload`] contents). Conversion functions handle
//! the bridge between runtime types (with channels, type-erased data) and
//! checkpoint types (plain JSON-serializable structs).
//!
//! Also provides [`GraphDefinition`] — a serializable snapshot of the graph
//! structure (nodes + edges) for checkpoint identity via [`GraphDefinition::hash()`].

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::graph::DataflowGraph;
use super::port::{BatchPayload, PortType, PortValue};
use crate::records::{
    AggregateRecord, CheckpointEntityRecord, CheckpointRelationRecord, DeleteRecord,
    EntityRecord, KBContentRecord, RelationRecord, UpdateRecord,
};

// ─── CheckpointPortValue ────────────────────────────────────────────────────

/// Fully serializable representation of a [`PortValue`].
///
/// For `Batch` payloads, the type-erased `Box<dyn Any>` is downcast to the
/// concrete record type (based on `batch_type`) and serialized to JSON.
/// For search values (`Results`, `Children`, etc.), the existing serde
/// serialization is used directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointPortValue {
    /// The port type (e.g., "Entities", "Empty", "Results").
    pub port_type: PortType,
    /// Whether this is a Batch or a direct value.
    pub is_batch: bool,
    /// JSON-serialized content. `None` for `Empty`.
    pub data_json: Option<String>,
    /// Number of records (for Batch payloads).
    pub record_count: Option<usize>,
}

// ─── PortValue → CheckpointPortValue ────────────────────────────────────────

/// Convert a [`PortValue`] to a serializable checkpoint form.
///
/// For `Batch` payloads, borrows the inner data via `downcast_ref` (without
/// consuming it) and serializes to JSON based on the `batch_type`.
pub fn port_value_to_checkpoint(value: &PortValue) -> Result<CheckpointPortValue, String> {
    use crate::search::SearchMeta;
    use crate::search_strategy::{ChildSummary, ExpansionRule, UnifiedResult};

    // Trigger / Empty
    if value.is_trigger() {
        return Ok(CheckpointPortValue {
            port_type: PortType::Empty,
            is_batch: false,
            data_json: None,
            record_count: None,
        });
    }

    // BatchPayload (ingestion data)
    if let Some(payload) = value.downcast::<BatchPayload>() {
        let json = checkpoint_serialize_batch(payload)?;
        return Ok(CheckpointPortValue {
            port_type: payload.batch_type,
            is_batch: true,
            data_json: Some(json),
            record_count: Some(payload.count()),
        });
    }

    // Search value types (serializable)
    if let Some(v) = value.downcast::<Vec<UnifiedResult>>() {
        let json = serde_json::to_string(v).map_err(|e| e.to_string())?;
        return Ok(CheckpointPortValue { port_type: PortType::Results, is_batch: false, data_json: Some(json), record_count: None });
    }
    if let Some(v) = value.downcast::<HashMap<String, Vec<ChildSummary>>>() {
        let json = serde_json::to_string(v).map_err(|e| e.to_string())?;
        return Ok(CheckpointPortValue { port_type: PortType::Children, is_batch: false, data_json: Some(json), record_count: None });
    }
    if let Some(v) = value.downcast::<Vec<(String, String)>>() {
        let json = serde_json::to_string(v).map_err(|e| e.to_string())?;
        return Ok(CheckpointPortValue { port_type: PortType::Uuids, is_batch: false, data_json: Some(json), record_count: None });
    }
    if let Some(v) = value.downcast::<SearchMeta>() {
        let json = serde_json::to_string(v).map_err(|e| e.to_string())?;
        return Ok(CheckpointPortValue { port_type: PortType::Meta, is_batch: false, data_json: Some(json), record_count: None });
    }
    if let Some(v) = value.downcast::<Vec<ExpansionRule>>() {
        let json = serde_json::to_string(v).map_err(|e| e.to_string())?;
        return Ok(CheckpointPortValue { port_type: PortType::Rules, is_batch: false, data_json: Some(json), record_count: None });
    }
    // Texte brut (markdown d'un outil, réponse d'un modèle) : encodé en
    // chaîne JSON, que `render_port_value` rend telle quelle au modèle.
    if let Some(v) = value.downcast::<String>() {
        let json = serde_json::to_string(v).map_err(|e| e.to_string())?;
        return Ok(CheckpointPortValue { port_type: PortType::Text, is_batch: false, data_json: Some(json), record_count: None });
    }
    if let Some(v) = value.downcast::<serde_json::Value>() {
        let json = serde_json::to_string(v).map_err(|e| e.to_string())?;
        return Ok(CheckpointPortValue { port_type: PortType::Map, is_batch: false, data_json: Some(json), record_count: None });
    }

    // Unknown type — store as empty
    Ok(CheckpointPortValue {
        port_type: PortType::Any,
        is_batch: false,
        data_json: None,
        record_count: None,
    })
}

/// Convert a [`CheckpointPortValue`] back to a runtime [`PortValue`].
pub fn port_value_from_checkpoint(cpv: CheckpointPortValue) -> Result<PortValue, String> {
    if cpv.port_type == PortType::Empty {
        return Ok(PortValue::Trigger);
    }

    let json = cpv
        .data_json
        .ok_or_else(|| format!("missing data_json for {:?}", cpv.port_type))?;

    if cpv.is_batch {
        let payload = checkpoint_deserialize_batch(cpv.port_type, &json)?;
        Ok(PortValue::new(payload))
    } else {
        deserialize_non_batch_port_value(cpv.port_type, &json)
    }
}

// ─── BatchPayload checkpoint serialization ──────────────────────────────────

/// Serialize a [`BatchPayload`]'s contents to JSON without consuming the data.
///
/// Uses `downcast_ref` to borrow the inner `Vec<T>` based on `batch_type`,
/// converts records to their checkpoint form, and serializes.
fn checkpoint_serialize_batch(payload: &BatchPayload) -> Result<String, String> {
    let guard = payload
        .data_lock()
        .map_err(|_| "failed to lock BatchPayload data")?;
    let boxed = guard
        .as_ref()
        .ok_or("BatchPayload data already consumed")?;

    match payload.batch_type {
        PortType::Entities => {
            let records = boxed
                .downcast_ref::<Vec<EntityRecord>>()
                .ok_or("type mismatch: expected Vec<EntityRecord>")?;
            let checkpoint: Vec<CheckpointEntityRecord> =
                records.iter().map(|r| r.to_checkpoint()).collect();
            serde_json::to_string(&checkpoint).map_err(|e| e.to_string())
        }
        PortType::Relations => {
            let records = boxed
                .downcast_ref::<Vec<RelationRecord>>()
                .ok_or("type mismatch: expected Vec<RelationRecord>")?;
            let checkpoint: Vec<CheckpointRelationRecord> =
                records.iter().map(|r| r.to_checkpoint()).collect();
            serde_json::to_string(&checkpoint).map_err(|e| e.to_string())
        }
        PortType::Aggregates => {
            let records = boxed
                .downcast_ref::<Vec<AggregateRecord>>()
                .ok_or("type mismatch: expected Vec<AggregateRecord>")?;
            serde_json::to_string(records).map_err(|e| e.to_string())
        }
        PortType::KBContent => {
            let records = boxed
                .downcast_ref::<Vec<KBContentRecord>>()
                .ok_or("type mismatch: expected Vec<KBContentRecord>")?;
            serde_json::to_string(records).map_err(|e| e.to_string())
        }
        PortType::Updates => {
            let records = boxed
                .downcast_ref::<Vec<UpdateRecord>>()
                .ok_or("type mismatch: expected Vec<UpdateRecord>")?;
            serde_json::to_string(records).map_err(|e| e.to_string())
        }
        PortType::Deletes => {
            let records = boxed
                .downcast_ref::<Vec<DeleteRecord>>()
                .ok_or("type mismatch: expected Vec<DeleteRecord>")?;
            serde_json::to_string(records).map_err(|e| e.to_string())
        }
        other => Err(format!(
            "unsupported batch_type for checkpoint: {other:?}"
        )),
    }
}

/// Deserialize a [`BatchPayload`] from checkpoint JSON.
fn checkpoint_deserialize_batch(port_type: PortType, json: &str) -> Result<BatchPayload, String> {
    match port_type {
        PortType::Entities => {
            let checkpoint: Vec<CheckpointEntityRecord> =
                serde_json::from_str(json).map_err(|e| e.to_string())?;
            let records: Vec<EntityRecord> = checkpoint
                .into_iter()
                .map(|c| c.into_entity_record())
                .collect();
            Ok(BatchPayload::new(PortType::Entities, records))
        }
        PortType::Relations => {
            let checkpoint: Vec<CheckpointRelationRecord> =
                serde_json::from_str(json).map_err(|e| e.to_string())?;
            let records: Vec<RelationRecord> = checkpoint
                .into_iter()
                .map(|c| c.into_relation_record())
                .collect();
            Ok(BatchPayload::new(PortType::Relations, records))
        }
        PortType::Aggregates => {
            let records: Vec<AggregateRecord> =
                serde_json::from_str(json).map_err(|e| e.to_string())?;
            Ok(BatchPayload::new(PortType::Aggregates, records))
        }
        PortType::KBContent => {
            let records: Vec<KBContentRecord> =
                serde_json::from_str(json).map_err(|e| e.to_string())?;
            Ok(BatchPayload::new(PortType::KBContent, records))
        }
        PortType::Updates => {
            let records: Vec<UpdateRecord> =
                serde_json::from_str(json).map_err(|e| e.to_string())?;
            Ok(BatchPayload::new(PortType::Updates, records))
        }
        PortType::Deletes => {
            let records: Vec<DeleteRecord> =
                serde_json::from_str(json).map_err(|e| e.to_string())?;
            Ok(BatchPayload::new(PortType::Deletes, records))
        }
        other => Err(format!(
            "unsupported port_type for checkpoint deserialization: {other:?}"
        )),
    }
}

/// Deserialize a non-batch, non-empty PortValue from checkpoint.
fn deserialize_non_batch_port_value(port_type: PortType, json: &str) -> Result<PortValue, String> {
    use crate::search::SearchMeta;
    use crate::search_strategy::{ChildSummary, ExpansionRule, UnifiedResult};

    match port_type {
        PortType::Results => {
            let v: Vec<UnifiedResult> = serde_json::from_str(json).map_err(|e| format!("Results deser: {e}"))?;
            Ok(PortValue::new(v))
        }
        PortType::Children => {
            let v: HashMap<String, Vec<ChildSummary>> = serde_json::from_str(json).map_err(|e| format!("Children deser: {e}"))?;
            Ok(PortValue::new(v))
        }
        PortType::Uuids => {
            let v: Vec<(String, String)> = serde_json::from_str(json).map_err(|e| format!("Uuids deser: {e}"))?;
            Ok(PortValue::new(v))
        }
        PortType::Meta => {
            let v: SearchMeta = serde_json::from_str(json).map_err(|e| format!("Meta deser: {e}"))?;
            Ok(PortValue::new(v))
        }
        PortType::Rules => {
            let v: Vec<ExpansionRule> = serde_json::from_str(json).map_err(|e| format!("Rules deser: {e}"))?;
            Ok(PortValue::new(v))
        }
        PortType::Map | PortType::Any => {
            let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("Map deser: {e}"))?;
            Ok(PortValue::new(v))
        }
        PortType::Text => {
            let v: String = serde_json::from_str(json).map_err(|e| format!("Text deser: {e}"))?;
            Ok(PortValue::new(v))
        }
        PortType::Empty => Ok(PortValue::Trigger),
        other => Err(format!(
            "checkpoint deserialization not supported for {other:?}"
        )),
    }
}

// ─── Graph Definition ────────────────────────────────────────────────────────

/// Serializable definition of a single node in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDef {
    pub name: String,
    pub node_type: String,
    pub config: serde_json::Value,
}

/// Serializable definition of an edge in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDef {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
}

/// Serializable snapshot of the graph structure.
///
/// Used to compute a deterministic hash for checkpoint identity.
/// Two graphs with the same definition hash are structurally identical.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDefinition {
    pub nodes: Vec<NodeDef>,
    pub edges: Vec<EdgeDef>,
}

impl GraphDefinition {
    /// Compute a deterministic hash of the graph structure.
    ///
    /// Nodes are sorted by name, edges by (from_node, from_port, to_node, to_port).
    /// The hash is a hex-encoded BLAKE3 of the canonical JSON.
    pub fn hash(&self) -> String {
        // Sort nodes by name for deterministic serialization
        let mut sorted_nodes: Vec<&NodeDef> = self.nodes.iter().collect();
        sorted_nodes.sort_by_key(|n| &n.name);

        // Sort edges for deterministic serialization
        let mut sorted_edges: Vec<&EdgeDef> = self.edges.iter().collect();
        sorted_edges.sort_by(|a, b| {
            (&a.from_node, &a.from_port, &a.to_node, &a.to_port)
                .cmp(&(&b.from_node, &b.from_port, &b.to_node, &b.to_port))
        });

        // Build canonical form as BTreeMap for stable JSON key order
        let canonical = serde_json::json!({
            "nodes": sorted_nodes.iter().map(|n| {
                serde_json::json!({
                    "name": n.name,
                    "node_type": n.node_type,
                    "config": n.config,
                })
            }).collect::<Vec<_>>(),
            "edges": sorted_edges.iter().map(|e| {
                serde_json::json!({
                    "from_node": e.from_node,
                    "from_port": e.from_port,
                    "to_node": e.to_node,
                    "to_port": e.to_port,
                })
            }).collect::<Vec<_>>(),
        });

        let json = serde_json::to_string(&canonical).unwrap_or_default();
        crate::hash::content_hash(&json)
    }
}

impl DataflowGraph {
    /// Extract a serializable definition of the graph structure.
    ///
    /// Captures each node's `node_type()` and `node_config()`, plus all edges.
    pub fn to_definition(&self) -> GraphDefinition {
        let nodes = self.nodes.iter().map(|node| {
            NodeDef {
                name: node.name().to_string(),
                node_type: node.node_type().to_string(),
                config: node.node_config()
                    .and_then(|b| b.downcast::<serde_json::Value>().ok())
                    .map(|v| *v)
                    .unwrap_or_default(),
            }
        }).collect();

        let edges = self.edges.iter().map(|e| EdgeDef {
            from_node: e.from_node.clone(),
            from_port: e.from_port.clone(),
            to_node: e.to_node.clone(),
            to_port: e.to_port.clone(),
        }).collect();

        GraphDefinition { nodes, edges }
    }
}


// ─── Execution Checkpoint Types ──────────────────────────────────────────────

/// Status of a single node within a checkpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeCheckpointStatus {
    Pending,
    Completed,
    Failed,
}

/// State of a single node in a checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCheckpoint {
    pub status: NodeCheckpointStatus,
    pub output_ports: HashMap<String, CheckpointPortValue>,
    /// Undo context captured after successful execution (for rollback).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undo_context: Option<serde_json::Value>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    pub completed_at: Option<u64>,
}

/// Overall status of a dataflow execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CheckpointExecutionStatus {
    Running,
    Completed,
    Failed,
}

/// Full checkpoint state for a dataflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCheckpoint {
    pub execution_id: String,
    pub status: CheckpointExecutionStatus,
    pub graph_def: GraphDefinition,
    pub graph_hash: String,
    pub nodes: HashMap<String, NodeCheckpoint>,
    /// Serialized initial_inputs for resume. Keyed by node_name → port_name → value.
    #[serde(default)]
    pub initial_inputs: HashMap<String, HashMap<String, CheckpointPortValue>>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Persistence layer for dataflow checkpoints.
pub trait CheckpointStore: Send + Sync {
    fn initialize(&self) -> Result<(), String>;
    fn create_execution(&self, checkpoint: &ExecutionCheckpoint) -> Result<(), String>;
    fn load_execution(&self, execution_id: &str) -> Result<Option<ExecutionCheckpoint>, String>;
    fn find_incomplete(&self) -> Result<Vec<String>, String>;
    fn save_node_completed(
        &self,
        execution_id: &str,
        node_name: &str,
        outputs: &HashMap<String, CheckpointPortValue>,
        undo_context: Option<&serde_json::Value>,
        duration_ms: u64,
    ) -> Result<(), String>;
    fn save_node_failed(&self, execution_id: &str, node_name: &str, error: &str) -> Result<(), String>;
    fn mark_completed(&self, execution_id: &str) -> Result<(), String>;
    fn mark_failed(&self, execution_id: &str, error: &str) -> Result<(), String>;
    fn delete(&self, execution_id: &str) -> Result<(), String>;
}

// ─── Timestamp helper ────────────────────────────────────────────────────────

/// Current timestamp in milliseconds since UNIX epoch.
pub fn timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::connection::CypherValue;
    use crate::dataflow::node_factories::register_builtins;
    use crate::dataflow::node_registry::NodeRegistry;
    use crate::dataflow::record_nodes::{
        KBEmbedNode, InsertRecordNode, LinkRecordNode,
    };
    use crate::records::{CheckpointRefStatus, EntityRecord};
    use crate::refs::EntityRef;

    fn builtin_registry() -> NodeRegistry {
        let mut registry = NodeRegistry::new();
        register_builtins(&mut registry);
        registry
    }

    #[test]
    fn checkpoint_empty_roundtrip() {
        let cpv = port_value_to_checkpoint(&PortValue::Trigger).unwrap();
        assert_eq!(cpv.port_type, PortType::Empty);
        assert!(!cpv.is_batch);
        assert!(cpv.data_json.is_none());

        let restored = port_value_from_checkpoint(cpv).unwrap();
        assert!(restored.is_trigger());
    }

    #[test]
    fn checkpoint_entity_batch_roundtrip() {
        // Create EntityRecords with resolved refs
        let (entity_ref, resolver) = EntityRef::new("Document");
        resolver.resolve("uuid-abc".to_string());

        let mut data = BTreeMap::new();
        data.insert(
            "_uuid".to_string(),
            CypherValue::String("uuid-abc".to_string()),
        );
        data.insert(
            "title".to_string(),
            CypherValue::String("Hello".to_string()),
        );

        let record = EntityRecord {
            entity_name: "Document".to_string(),
            data,
            entity_ref,
            resolver: None,
        };

        let payload = BatchPayload::new(PortType::Entities, vec![record]);
        let pv = PortValue::new(payload);

        // Serialize to checkpoint
        let cpv = port_value_to_checkpoint(&pv).unwrap();
        assert_eq!(cpv.port_type, PortType::Entities);
        assert!(cpv.is_batch);
        assert_eq!(cpv.record_count, Some(1));
        assert!(cpv.data_json.is_some());

        // Deserialize from checkpoint
        let restored = port_value_from_checkpoint(cpv).unwrap();
        let payload = restored.take::<BatchPayload>().expect("expected BatchPayload");
        assert_eq!(payload.batch_type, PortType::Entities);
        assert_eq!(payload.count(), 1);

        let records = payload.take::<EntityRecord>().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].entity_name, "Document");
        assert_eq!(
            records[0].data.get("title").and_then(|v| v.as_str()),
            Some("Hello")
        );
        // Pre-resolved ref
        assert_eq!(records[0].entity_ref.uuid().unwrap(), "uuid-abc");
        // No resolver (consumed in original execution)
        assert!(records[0].resolver.is_none());
    }

    #[test]
    fn checkpoint_aggregate_batch_roundtrip() {
        let records = vec![
            AggregateRecord {
                index_entry_uuid: "idx-1".into(),
                kb_name: "TreeKB".into(),
                title_entity: "Directory".into(),
                source_uuid: "dir-1".into(),
            },
            AggregateRecord {
                index_entry_uuid: "idx-2".into(),
                kb_name: "TreeKB".into(),
                title_entity: "Directory".into(),
                source_uuid: "dir-2".into(),
            },
        ];
        let pv = PortValue::new(BatchPayload::new(PortType::Aggregates, records));

        let cpv = port_value_to_checkpoint(&pv).unwrap();
        let restored = port_value_from_checkpoint(cpv).unwrap();

        let payload = restored.take::<BatchPayload>().expect("expected BatchPayload");
        let records = payload.take::<AggregateRecord>().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].index_entry_uuid, "idx-1");
        assert_eq!(records[1].index_entry_uuid, "idx-2");
    }

    #[test]
    fn checkpoint_entity_ref_state() {
        // Test that ref state is correctly captured
        let (entity_ref, resolver) = EntityRef::new("File");
        resolver.resolve("file-uuid".to_string());

        let record = EntityRecord {
            entity_name: "File".to_string(),
            data: BTreeMap::new(),
            entity_ref,
            resolver: None,
        };

        let cp = record.to_checkpoint();
        assert_eq!(cp.ref_state.type_name, "File");
        assert!(matches!(
            cp.ref_state.status,
            CheckpointRefStatus::Ready { .. }
        ));

        if let CheckpointRefStatus::Ready { uuid } = &cp.ref_state.status {
            assert_eq!(uuid, "file-uuid");
        }
    }

    // ── Graph definition tests ──────────────────────────────────────────

    #[test]
    fn graph_definition_from_ingestion_graph() {
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(InsertRecordNode::new("inserts"))).unwrap();
        graph.add_node(Box::new(LinkRecordNode::new("links"))).unwrap();
        graph.connect("inserts", "done", "links", "trigger").unwrap();

        let def = graph.to_definition();
        assert_eq!(def.nodes.len(), 2);
        assert_eq!(def.edges.len(), 1);

        assert_eq!(def.nodes[0].name, "inserts");
        assert_eq!(def.nodes[0].node_type, "InsertRecordNode");
        assert_eq!(def.nodes[1].name, "links");
        assert_eq!(def.nodes[1].node_type, "LinkRecordNode");

        assert_eq!(def.edges[0].from_node, "inserts");
        assert_eq!(def.edges[0].from_port, "done");
        assert_eq!(def.edges[0].to_node, "links");
        assert_eq!(def.edges[0].to_port, "trigger");
    }

    #[test]
    fn graph_definition_hash_deterministic() {
        let mut g1 = DataflowGraph::new();
        g1.add_node(Box::new(InsertRecordNode::new("inserts"))).unwrap();
        g1.add_node(Box::new(LinkRecordNode::new("links"))).unwrap();
        g1.connect("inserts", "done", "links", "trigger").unwrap();

        let mut g2 = DataflowGraph::new();
        g2.add_node(Box::new(InsertRecordNode::new("inserts"))).unwrap();
        g2.add_node(Box::new(LinkRecordNode::new("links"))).unwrap();
        g2.connect("inserts", "done", "links", "trigger").unwrap();

        assert_eq!(g1.to_definition().hash(), g2.to_definition().hash());
    }

    #[test]
    fn graph_definition_hash_changes_with_structure() {
        let mut g1 = DataflowGraph::new();
        g1.add_node(Box::new(InsertRecordNode::new("inserts"))).unwrap();

        let mut g2 = DataflowGraph::new();
        g2.add_node(Box::new(InsertRecordNode::new("inserts"))).unwrap();
        g2.add_node(Box::new(LinkRecordNode::new("links"))).unwrap();
        g2.connect("inserts", "done", "links", "trigger").unwrap();

        assert_ne!(g1.to_definition().hash(), g2.to_definition().hash());
    }

    #[test]
    fn graph_definition_hash_changes_with_config() {
        let mut g1 = DataflowGraph::new();
        g1.add_node(Box::new(KBEmbedNode::new("embeds", 32))).unwrap();

        let mut g2 = DataflowGraph::new();
        g2.add_node(Box::new(KBEmbedNode::new("embeds", 64))).unwrap();

        assert_ne!(g1.to_definition().hash(), g2.to_definition().hash());
    }

    #[test]
    fn graph_definition_serializable() {
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(KBEmbedNode::new("embeds", 32))).unwrap();

        let def = graph.to_definition();
        let json = serde_json::to_string(&def).unwrap();
        let restored: GraphDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.nodes.len(), 1);
        assert_eq!(restored.nodes[0].node_type, "KBEmbedNode");
        assert_eq!(
            restored.nodes[0].config.get("gpu_batch_size").unwrap().as_u64(),
            Some(32)
        );
        assert_eq!(def.hash(), restored.hash());
    }

    #[test]
    fn registry_creates_all_checkpoint_types() {
        let registry = builtin_registry();
        let cases = vec![
            ("InsertRecordNode", serde_json::json!({})),
            ("LinkRecordNode", serde_json::json!({})),
            ("KBEmbedNode", serde_json::json!({"gpu_batch_size": 16})),
            ("ChunkRecordNode", serde_json::json!({})),
            ("EmbedNode", serde_json::json!({})),
            ("KBChunkRecordNode", serde_json::json!({})),
            ("KBGatherNode", serde_json::json!({})),
            ("KBUpdateNode", serde_json::json!({})),
            ("KBChunkNode", serde_json::json!({})),
            ("FlushNode", serde_json::json!({"table": "Test_Index"})),
            ("SparseCommitNode", serde_json::json!({"table": "Test_Chunk"})),
        ];

        for (node_type, config) in &cases {
            let node = registry.create(node_type, "test", config);
            assert!(node.is_ok(), "failed to create {node_type}");
            let node = node.unwrap();
            assert_eq!(node.name(), "test");
            assert_eq!(node.node_type(), *node_type);
        }

        // Unknown type fails
        let err = registry.create("Bogus", "x", &serde_json::json!({}));
        assert!(err.is_err());
    }

    #[test]
    fn registry_embed_node_preserves_config() {
        let registry = builtin_registry();
        let config = serde_json::json!({"gpu_batch_size": 64});
        let node = registry.create("KBEmbedNode", "embeds", &config).unwrap();
        let restored_config = node.node_config()
            .and_then(|b| b.downcast::<serde_json::Value>().ok())
            .expect("expected serde_json::Value config");
        assert_eq!(restored_config.get("gpu_batch_size").unwrap().as_u64(), Some(64));
    }
}

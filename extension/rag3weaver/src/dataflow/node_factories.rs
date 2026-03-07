//! Built-in node factories for the NodeRegistry.
//!
//! - Macro-generated factories for simple/named nodes
//! - Manual factories for nodes with config params
//! - `register_builtins()` populates a registry with all 13 node types

use crate::named_factory;

use super::node_registry::{
    ConfigParam, ConfigParamType, NodeFactory, NodeRegistry, NodeSchema,
};
use super::port::{PortDef, PortType};
use super::search_nodes::{
    ComposeNode, FetchRelatedNode, PrimarySearchNode, QuerySourceNode,
};
use super::record_nodes::{
    ChunkKBNode, ChunkRecordNode, EmbedRecordNode, FlushFTSNode, GatherKBNode,
    InsertRecordNode, LinkRecordNode, UpdateKBNode,
};
use super::migration_nodes::{CypherNodeFactory, ValidateNodeFactory};

use crate::search_strategy::ExpansionDirection;

// ─── Macro-generated factories (simple_factory!) ────────────────────────────

named_factory!(
    ComposeNodeFactory,
    ComposeNode,
    "ComposeNode",
    "Attaches fetched children to root results",
    &[
        PortDef { name: "results", port_type: PortType::Results, required: true },
        PortDef { name: "children", port_type: PortType::Children, required: false },
    ],
    &[PortDef { name: "results", port_type: PortType::Results, required: false }],
);

named_factory!(
    PrimarySearchNodeFactory,
    PrimarySearchNode,
    "PrimarySearchNode",
    "Runs Catalog::search() via service registry",
    &[PortDef { name: "query", port_type: PortType::Query, required: true }],
    &[
        PortDef { name: "results", port_type: PortType::Results, required: false },
        PortDef { name: "meta", port_type: PortType::Meta, required: false },
    ],
);

// ─── Macro-generated factories (named_factory!) ─────────────────────────────

named_factory!(
    InsertRecordNodeFactory,
    InsertRecordNode,
    "InsertRecordNode",
    "UNWIND MERGE on _uuid from Vec<EntityRecord>",
    &[
        PortDef { name: "entities", port_type: PortType::Entities, required: true },
        PortDef { name: "trigger", port_type: PortType::Empty, required: false },
    ],
    &[
        PortDef { name: "done", port_type: PortType::Empty, required: false },
        PortDef { name: "inserted", port_type: PortType::Entities, required: false },
    ],
);

named_factory!(
    LinkRecordNodeFactory,
    LinkRecordNode,
    "LinkRecordNode",
    "UNWIND MATCH+MERGE from Vec<RelationRecord>",
    &[
        PortDef { name: "relations", port_type: PortType::Relations, required: true },
        PortDef { name: "trigger", port_type: PortType::Empty, required: false },
    ],
    &[PortDef { name: "done", port_type: PortType::Empty, required: false }],
);

named_factory!(
    ChunkRecordNodeFactory,
    ChunkRecordNode,
    "ChunkRecordNode",
    "Parallel chunking, outputs chunk entities + links",
    &[
        PortDef { name: "entities", port_type: PortType::Entities, required: true },
        PortDef { name: "trigger", port_type: PortType::Empty, required: false },
    ],
    &[
        PortDef { name: "done", port_type: PortType::Empty, required: false },
        PortDef { name: "chunks", port_type: PortType::Entities, required: false },
        PortDef { name: "chunk_links", port_type: PortType::Relations, required: false },
    ],
);

named_factory!(
    GatherKBNodeFactory,
    GatherKBNode,
    "GatherKBNode",
    "Read DB, detect content changes, output changed KBContentRecords",
    &[
        PortDef { name: "aggregates", port_type: PortType::Aggregates, required: true },
        PortDef { name: "trigger", port_type: PortType::Empty, required: false },
    ],
    &[
        PortDef { name: "kb_content", port_type: PortType::KBContent, required: false },
        PortDef { name: "done", port_type: PortType::Empty, required: false },
    ],
);

named_factory!(
    UpdateKBNodeFactory,
    UpdateKBNode,
    "UpdateKBNode",
    "Update KB_Index entries + delete stale chunks",
    &[PortDef { name: "kb_content", port_type: PortType::KBContent, required: true }],
    &[
        PortDef { name: "kb_content", port_type: PortType::KBContent, required: false },
        PortDef { name: "done", port_type: PortType::Empty, required: false },
    ],
);

named_factory!(
    ChunkKBNodeFactory,
    ChunkKBNode,
    "ChunkKBNode",
    "Generate chunk entities + relations from aggregated content",
    &[PortDef { name: "kb_content", port_type: PortType::KBContent, required: true }],
    &[
        PortDef { name: "entities", port_type: PortType::Entities, required: false },
        PortDef { name: "relations", port_type: PortType::Relations, required: false },
        PortDef { name: "done", port_type: PortType::Empty, required: false },
    ],
);

named_factory!(
    FlushFTSNodeFactory,
    FlushFTSNode,
    "FlushFTSNode",
    "Flush full-text search indexes for KB names",
    &[PortDef { name: "trigger", port_type: PortType::Empty, required: false }],
    &[PortDef { name: "done", port_type: PortType::Empty, required: false }],
);

// ─── Manual factories ───────────────────────────────────────────────────────

/// Factory for QuerySourceNode (config: kb_name, query, options).
///
/// Note: QuerySourceNode is typically created directly by catalog code,
/// not from registry config. This factory exists for completeness/introspection.
pub struct QuerySourceNodeFactory;

impl NodeFactory for QuerySourceNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let kb_name = config
            .get("kb_name")
            .and_then(|v| v.as_str())
            .ok_or("QuerySourceNode: missing 'kb_name' config")?;
        let query = config
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or("QuerySourceNode: missing 'query' config")?;
        let options: crate::search::SearchOptions = if let Some(opts) = config.get("options") {
            serde_json::from_value(opts.clone())
                .map_err(|e| format!("QuerySourceNode: invalid 'options': {e}"))?
        } else {
            crate::search::SearchOptions::default()
        };
        Ok(Box::new(QuerySourceNode::named(name, kb_name, query, &options)))
    }

    fn node_type(&self) -> &'static str {
        "QuerySourceNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "QuerySourceNode",
            description: "Emits search query + options",
            inputs: vec![],
            outputs: vec![PortDef {
                name: "query",
                port_type: PortType::Query,
                required: false,
            }],
            config_params: vec![
                ConfigParam {
                    name: "kb_name",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Knowledge base name",
                },
                ConfigParam {
                    name: "query",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Search query text",
                },
                ConfigParam {
                    name: "options",
                    param_type: ConfigParamType::Json,
                    required: false,
                    default: None,
                    description: "SearchOptions as JSON",
                },
            ],
        }
    }
}

/// Factory for FetchRelatedNode (config: relation, direction, limit, source_entity).
pub struct FetchRelatedNodeFactory;

impl NodeFactory for FetchRelatedNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let relation = config
            .get("relation")
            .and_then(|v| v.as_str())
            .ok_or("FetchRelatedNode: missing 'relation' config")?
            .to_string();
        let direction = match config
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("Outgoing")
        {
            "Incoming" => ExpansionDirection::Incoming,
            _ => ExpansionDirection::Outgoing,
        };
        let limit = config
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        let source_entity = config
            .get("source_entity")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Box::new(FetchRelatedNode::new(
            name,
            relation,
            direction,
            limit,
            source_entity,
        )))
    }

    fn node_type(&self) -> &'static str {
        "FetchRelatedNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "FetchRelatedNode",
            description: "Cypher graph traversal from parent results",
            inputs: vec![PortDef {
                name: "results",
                port_type: PortType::Results,
                required: true,
            }],
            outputs: vec![PortDef {
                name: "children",
                port_type: PortType::Children,
                required: false,
            }],
            config_params: vec![
                ConfigParam {
                    name: "relation",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Relationship type (e.g. HAS_FILE)",
                },
                ConfigParam {
                    name: "direction",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("Outgoing")),
                    description: "Outgoing or Incoming",
                },
                ConfigParam {
                    name: "limit",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(10)),
                    description: "Max children per parent (0 = unlimited)",
                },
                ConfigParam {
                    name: "source_entity",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Filter results by source entity type",
                },
            ],
        }
    }
}

/// Factory for EmbedRecordNode (config: gpu_batch_size).
pub struct EmbedRecordNodeFactory;

impl NodeFactory for EmbedRecordNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let gpu_batch_size = config
            .get("gpu_batch_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(64) as usize;
        Ok(Box::new(EmbedRecordNode::new(name, gpu_batch_size)))
    }

    fn node_type(&self) -> &'static str {
        "EmbedRecordNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "EmbedRecordNode",
            description: "Unified embedding with _embed_hash skip",
            inputs: vec![
                PortDef {
                    name: "entities",
                    port_type: PortType::Entities,
                    required: true,
                },
                PortDef {
                    name: "trigger",
                    port_type: PortType::Empty,
                    required: false,
                },
            ],
            outputs: vec![PortDef {
                name: "done",
                port_type: PortType::Empty,
                required: false,
            }],
            config_params: vec![ConfigParam {
                name: "gpu_batch_size",
                param_type: ConfigParamType::Int,
                required: false,
                default: Some(serde_json::json!(64)),
                description: "GPU batch size for embedding calls",
            }],
        }
    }
}

// ─── register_builtins ──────────────────────────────────────────────────────

/// Populate a NodeRegistry with all 14 built-in node types.
pub fn register_builtins(registry: &mut NodeRegistry) {
    // Search nodes
    registry.register(Box::new(ComposeNodeFactory));
    registry.register(Box::new(PrimarySearchNodeFactory));
    registry.register(Box::new(QuerySourceNodeFactory));
    registry.register(Box::new(FetchRelatedNodeFactory));
    // Record nodes
    registry.register(Box::new(InsertRecordNodeFactory));
    registry.register(Box::new(LinkRecordNodeFactory));
    registry.register(Box::new(EmbedRecordNodeFactory));
    registry.register(Box::new(ChunkRecordNodeFactory));
    registry.register(Box::new(GatherKBNodeFactory));
    registry.register(Box::new(UpdateKBNodeFactory));
    registry.register(Box::new(ChunkKBNodeFactory));
    registry.register(Box::new(FlushFTSNodeFactory));
    // Migration nodes
    registry.register(Box::new(CypherNodeFactory));
    registry.register(Box::new(ValidateNodeFactory));
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin_registry() -> NodeRegistry {
        let mut registry = NodeRegistry::new();
        register_builtins(&mut registry);
        registry
    }

    #[test]
    fn register_builtins_has_all_14_types() {
        let registry = builtin_registry();
        assert_eq!(registry.types().len(), 14);
    }

    #[test]
    fn all_factories_have_schemas() {
        let registry = builtin_registry();
        for node_type in registry.types() {
            let schema = registry.schema(node_type);
            assert!(schema.is_some(), "missing schema for {node_type}");
            let schema = schema.unwrap();
            assert_eq!(schema.node_type, node_type);
            assert!(!schema.description.is_empty(), "empty description for {node_type}");
        }
    }

    #[test]
    fn simple_factory_creates_compose_node() {
        let registry = builtin_registry();
        let node = registry
            .create("ComposeNode", "my_compose", &serde_json::json!({}))
            .unwrap();
        assert_eq!(node.node_type(), "ComposeNode");
        assert_eq!(node.name(), "my_compose");
    }

    #[test]
    fn named_factory_creates_insert_record_node() {
        let registry = builtin_registry();
        let node = registry
            .create("InsertRecordNode", "insert_0", &serde_json::json!({}))
            .unwrap();
        assert_eq!(node.node_type(), "InsertRecordNode");
        assert_eq!(node.name(), "insert_0");
    }

    #[test]
    fn fetch_related_factory_with_config() {
        let registry = builtin_registry();
        let config = serde_json::json!({
            "relation": "HAS_FILE",
            "direction": "Outgoing",
            "limit": 5,
            "source_entity": "Directory",
        });
        let node = registry
            .create("FetchRelatedNode", "fetch_0", &config)
            .unwrap();
        assert_eq!(node.node_type(), "FetchRelatedNode");
        assert_eq!(node.name(), "fetch_0");
    }

    #[test]
    fn fetch_related_factory_defaults() {
        let registry = builtin_registry();
        let config = serde_json::json!({ "relation": "HAS_FILE" });
        let node = registry
            .create("FetchRelatedNode", "fetch_1", &config)
            .unwrap();
        assert_eq!(node.node_type(), "FetchRelatedNode");
    }

    #[test]
    fn fetch_related_factory_missing_relation_errors() {
        let registry = builtin_registry();
        let result = registry.create("FetchRelatedNode", "fetch_2", &serde_json::json!({}));
        match result {
            Err(e) => assert!(e.contains("missing 'relation'"), "got: {e}"),
            Ok(_) => panic!("expected error"),
        }
    }

    #[test]
    fn embed_record_factory_with_config() {
        let registry = builtin_registry();
        let config = serde_json::json!({ "gpu_batch_size": 128 });
        let node = registry
            .create("EmbedRecordNode", "embed_0", &config)
            .unwrap();
        assert_eq!(node.node_type(), "EmbedRecordNode");
        assert_eq!(node.name(), "embed_0");
    }

    #[test]
    fn embed_record_factory_default_batch_size() {
        let registry = builtin_registry();
        let node = registry
            .create("EmbedRecordNode", "embed_1", &serde_json::json!({}))
            .unwrap();
        assert_eq!(node.node_type(), "EmbedRecordNode");
    }

    #[test]
    fn query_source_factory_with_config() {
        let registry = builtin_registry();
        let config = serde_json::json!({
            "kb_name": "test_kb",
            "query": "hello world",
        });
        let node = registry
            .create("QuerySourceNode", "qs", &config)
            .unwrap();
        assert_eq!(node.node_type(), "QuerySourceNode");
    }

    #[test]
    fn query_source_factory_missing_kb_name_errors() {
        let registry = builtin_registry();
        let result = registry.create(
            "QuerySourceNode",
            "qs",
            &serde_json::json!({ "query": "hello" }),
        );
        match result {
            Err(e) => assert!(e.contains("missing 'kb_name'"), "got: {e}"),
            Ok(_) => panic!("expected error"),
        }
    }
}

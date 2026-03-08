//! Built-in node factories for the NodeRegistry.
//!
//! - Macro-generated factories for simple/named nodes
//! - Manual factories for nodes with config params
//! - `register_builtins()` populates a registry with all 22 node types

use crate::named_factory;

use super::node_registry::{
    ConfigParam, ConfigParamType, NodeFactory, NodeRegistry, NodeSchema,
};
use super::port::{PortDef, PortType};
use super::search_nodes::{
    ComposeNode, FetchRelatedNode, KBSearchNode, KBQuerySourceNode,
};
use super::generic_search_nodes::{
    SearchSourceNode, VectorSearchNode, BM25SearchNode,
    SparseSearchNode, FuseResultsNode, ResolveParentNode,
};
use super::record_nodes::{
    ChunkRecordNode, EmbedNode, KBChunkNode, KBChunkRecordNode, KBEmbedNode, FlushNode, KBGatherNode,
    InsertRecordNode, LinkRecordNode, KBUpdateNode,
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
    KBSearchNodeFactory,
    KBSearchNode,
    "KBSearchNode",
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
    KBChunkRecordNodeFactory,
    KBChunkRecordNode,
    "KBChunkRecordNode",
    "Parallel chunking for KB entities, outputs chunk entities + links",
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
    ChunkRecordNodeFactory,
    ChunkRecordNode,
    "ChunkRecordNode",
    "Parallel chunking for simple entities, outputs chunk entities + links",
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
    KBGatherNodeFactory,
    KBGatherNode,
    "KBGatherNode",
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
    KBUpdateNodeFactory,
    KBUpdateNode,
    "KBUpdateNode",
    "Update KB_Index entries + delete stale chunks",
    &[PortDef { name: "kb_content", port_type: PortType::KBContent, required: true }],
    &[
        PortDef { name: "kb_content", port_type: PortType::KBContent, required: false },
        PortDef { name: "done", port_type: PortType::Empty, required: false },
    ],
);

named_factory!(
    KBChunkNodeFactory,
    KBChunkNode,
    "KBChunkNode",
    "Generate chunk entities + relations from aggregated content",
    &[PortDef { name: "kb_content", port_type: PortType::KBContent, required: true }],
    &[
        PortDef { name: "entities", port_type: PortType::Entities, required: false },
        PortDef { name: "relations", port_type: PortType::Relations, required: false },
        PortDef { name: "done", port_type: PortType::Empty, required: false },
    ],
);

/// Factory for FlushNode (config: table or tables).
pub struct FlushNodeFactory;

impl NodeFactory for FlushNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let tables = if let Some(arr) = config.get("tables").and_then(|v| v.as_array()) {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else if let Some(t) = config.get("table").and_then(|v| v.as_str()) {
            vec![t.to_string()]
        } else {
            return Err("FlushNode: missing 'table' or 'tables' config".to_string());
        };
        Ok(Box::new(FlushNode::new(name, tables)))
    }

    fn node_type(&self) -> &'static str {
        "FlushNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "FlushNode",
            description: "Flush Lucivy FTS indexes for configured tables",
            config_params: vec![
                ConfigParam {
                    name: "table",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Single table name to flush",
                },
            ],
            inputs: vec![PortDef { name: "trigger", port_type: PortType::Empty, required: false }],
            outputs: vec![PortDef { name: "done", port_type: PortType::Empty, required: false }],
        }
    }
}

// ─── Manual factories ───────────────────────────────────────────────────────

/// Factory for KBQuerySourceNode (config: kb_name, query, options).
///
/// Note: KBQuerySourceNode is typically created directly by catalog code,
/// not from registry config. This factory exists for completeness/introspection.
pub struct KBQuerySourceNodeFactory;

impl NodeFactory for KBQuerySourceNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let kb_name = config
            .get("kb_name")
            .and_then(|v| v.as_str())
            .ok_or("KBQuerySourceNode: missing 'kb_name' config")?;
        let query = config
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or("KBQuerySourceNode: missing 'query' config")?;
        let options: crate::search::SearchOptions = if let Some(opts) = config.get("options") {
            serde_json::from_value(opts.clone())
                .map_err(|e| format!("KBQuerySourceNode: invalid 'options': {e}"))?
        } else {
            crate::search::SearchOptions::default()
        };
        Ok(Box::new(KBQuerySourceNode::named(name, kb_name, query, &options)))
    }

    fn node_type(&self) -> &'static str {
        "KBQuerySourceNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "KBQuerySourceNode",
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

/// Factory for KBEmbedNode (config: gpu_batch_size).
pub struct KBEmbedNodeFactory;

impl NodeFactory for KBEmbedNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let gpu_batch_size = config
            .get("gpu_batch_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(64) as usize;
        Ok(Box::new(KBEmbedNode::new(name, gpu_batch_size)))
    }

    fn node_type(&self) -> &'static str {
        "KBEmbedNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "KBEmbedNode",
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

/// Factory for EmbedNode (config: gpu_batch_size, text_field, embedding_col, sparse_col).
pub struct EmbedNodeFactory;

impl NodeFactory for EmbedNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let gpu_batch_size = config
            .get("gpu_batch_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(64) as usize;

        let signals: crate::search::SearchSignals = config
            .get("signals")
            .map(|v| serde_json::from_value(v.clone()).unwrap_or(crate::search::SearchSignals::HYBRID))
            .unwrap_or(crate::search::SearchSignals::HYBRID);

        let mut node = EmbedNode::new(name, signals, gpu_batch_size);

        let text_field = config.get("text_field").and_then(|v| v.as_str()).unwrap_or("_text");
        let embedding_col = config.get("embedding_col").and_then(|v| v.as_str()).unwrap_or("embedding");
        let sparse_col = config.get("sparse_col").and_then(|v| v.as_str()).unwrap_or("sparse");
        node = node.with_columns(text_field, embedding_col, sparse_col);

        Ok(Box::new(node))
    }

    fn node_type(&self) -> &'static str {
        "EmbedNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "EmbedNode",
            description: "Embedding for simple entities (configurable columns)",
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
            config_params: vec![
                ConfigParam {
                    name: "gpu_batch_size",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(64)),
                    description: "GPU batch size for embedding calls",
                },
                ConfigParam {
                    name: "signals",
                    param_type: ConfigParamType::Json,
                    required: false,
                    default: Some(serde_json::json!(["bm25", "vector"])),
                    description: "Search signals array (bm25, vector, sparse)",
                },
                ConfigParam {
                    name: "text_field",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("_text")),
                    description: "Field name containing text to embed",
                },
                ConfigParam {
                    name: "embedding_col",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("embedding")),
                    description: "Column name for dense embeddings",
                },
                ConfigParam {
                    name: "sparse_col",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("sparse")),
                    description: "Prefix for sparse columns ({prefix}_indices, {prefix}_weights)",
                },
            ],
        }
    }
}

// ─── Generic search node factories ──────────────────────────────────────────

/// Factory for SearchSourceNode (config: target_name, query, options).
pub struct SearchSourceNodeFactory;

impl NodeFactory for SearchSourceNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let target_name = config
            .get("target_name")
            .and_then(|v| v.as_str())
            .ok_or("SearchSourceNode: missing 'target_name' config")?;
        let query = config
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or("SearchSourceNode: missing 'query' config")?;
        let options: crate::search::SearchOptions = if let Some(opts) = config.get("options") {
            serde_json::from_value(opts.clone())
                .map_err(|e| format!("SearchSourceNode: invalid 'options': {e}"))?
        } else {
            crate::search::SearchOptions::default()
        };
        Ok(Box::new(SearchSourceNode::new(name, target_name, query, options)))
    }

    fn node_type(&self) -> &'static str {
        "SearchSourceNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "SearchSourceNode",
            description: "Resolves SearchTarget and emits query",
            inputs: vec![],
            outputs: vec![PortDef {
                name: "query",
                port_type: PortType::Query,
                required: false,
            }],
            config_params: vec![
                ConfigParam {
                    name: "target_name",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Target name (KB or entity)",
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

/// Factory for VectorSearchNode (config: limit).
pub struct VectorSearchNodeFactory;

impl NodeFactory for VectorSearchNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let limit = config
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        Ok(Box::new(VectorSearchNode::new(name, limit)))
    }

    fn node_type(&self) -> &'static str {
        "VectorSearchNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "VectorSearchNode",
            description: "Vector similarity search on chunk embeddings",
            inputs: vec![PortDef {
                name: "query",
                port_type: PortType::Query,
                required: true,
            }],
            outputs: vec![PortDef {
                name: "results",
                port_type: PortType::Results,
                required: false,
            }],
            config_params: vec![ConfigParam {
                name: "limit",
                param_type: ConfigParamType::Int,
                required: false,
                default: Some(serde_json::json!(10)),
                description: "Max results to return",
            }],
        }
    }
}

/// Factory for BM25SearchNode (config: limit, fuzzy_distance, result_mode).
pub struct BM25SearchNodeFactory;

impl NodeFactory for BM25SearchNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let limit = config
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        let fuzzy_distance = config
            .get("fuzzy_distance")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u8;
        let result_mode = match config
            .get("result_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("Aggregated")
        {
            "Detailed" => crate::search::ResultMode::Detailed,
            "SourceResolved" => crate::search::ResultMode::SourceResolved,
            _ => crate::search::ResultMode::Aggregated,
        };
        let mut node = BM25SearchNode::new(name, limit);
        node = node.with_fuzzy(fuzzy_distance).with_result_mode(result_mode);
        Ok(Box::new(node))
    }

    fn node_type(&self) -> &'static str {
        "BM25SearchNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "BM25SearchNode",
            description: "BM25 full-text search with highlight→chunk resolution",
            inputs: vec![PortDef {
                name: "query",
                port_type: PortType::Query,
                required: true,
            }],
            outputs: vec![PortDef {
                name: "results",
                port_type: PortType::Results,
                required: false,
            }],
            config_params: vec![
                ConfigParam {
                    name: "limit",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(10)),
                    description: "Max results to return",
                },
                ConfigParam {
                    name: "fuzzy_distance",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(0)),
                    description: "Levenshtein distance for fuzzy matching",
                },
                ConfigParam {
                    name: "result_mode",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("Aggregated")),
                    description: "Result mode: Aggregated, Detailed, or SourceResolved",
                },
            ],
        }
    }
}

/// Factory for SparseSearchNode (config: limit).
pub struct SparseSearchNodeFactory;

impl NodeFactory for SparseSearchNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let limit = config
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        Ok(Box::new(SparseSearchNode::new(name, limit)))
    }

    fn node_type(&self) -> &'static str {
        "SparseSearchNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "SparseSearchNode",
            description: "Sparse vector search (SPLADE/BGE-M3)",
            inputs: vec![PortDef {
                name: "query",
                port_type: PortType::Query,
                required: true,
            }],
            outputs: vec![PortDef {
                name: "results",
                port_type: PortType::Results,
                required: false,
            }],
            config_params: vec![ConfigParam {
                name: "limit",
                param_type: ConfigParamType::Int,
                required: false,
                default: Some(serde_json::json!(10)),
                description: "Max results to return",
            }],
        }
    }
}

/// Factory for FuseResultsNode (no config).
pub struct FuseResultsNodeFactory;

impl NodeFactory for FuseResultsNodeFactory {
    fn create(
        &self,
        name: &str,
        _config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        Ok(Box::new(FuseResultsNode::new(name)))
    }

    fn node_type(&self) -> &'static str {
        "FuseResultsNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "FuseResultsNode",
            description: "RRF fusion of multi-signal search results",
            inputs: vec![
                PortDef { name: "vector", port_type: PortType::Results, required: false },
                PortDef { name: "bm25", port_type: PortType::Results, required: false },
                PortDef { name: "sparse", port_type: PortType::Results, required: false },
            ],
            outputs: vec![PortDef {
                name: "results",
                port_type: PortType::Results,
                required: false,
            }],
            config_params: vec![],
        }
    }
}

/// Factory for ResolveParentNode (config: return_fields).
pub struct ResolveParentNodeFactory;

impl NodeFactory for ResolveParentNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let mut node = ResolveParentNode::new(name);
        if let Some(fields) = config.get("return_fields").and_then(|v| v.as_array()) {
            let field_strs: Vec<String> = fields
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            node = node.with_return_fields(field_strs);
        }
        Ok(Box::new(node))
    }

    fn node_type(&self) -> &'static str {
        "ResolveParentNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "ResolveParentNode",
            description: "Resolve chunks to parent entities with data enrichment",
            inputs: vec![
                PortDef { name: "results", port_type: PortType::Results, required: true },
                PortDef { name: "query", port_type: PortType::Query, required: false },
            ],
            outputs: vec![PortDef {
                name: "results",
                port_type: PortType::Results,
                required: false,
            }],
            config_params: vec![ConfigParam {
                name: "return_fields",
                param_type: ConfigParamType::Json,
                required: false,
                default: None,
                description: "Fields to return from parent entity (JSON array of strings)",
            }],
        }
    }
}

// ─── register_builtins ──────────────────────────────────────────────────────

/// Populate a NodeRegistry with all 22 built-in node types.
pub fn register_builtins(registry: &mut NodeRegistry) {
    // Search nodes (KB)
    registry.register(Box::new(ComposeNodeFactory));
    registry.register(Box::new(KBSearchNodeFactory));
    registry.register(Box::new(KBQuerySourceNodeFactory));
    registry.register(Box::new(FetchRelatedNodeFactory));
    // Search nodes (generic)
    registry.register(Box::new(SearchSourceNodeFactory));
    registry.register(Box::new(VectorSearchNodeFactory));
    registry.register(Box::new(BM25SearchNodeFactory));
    registry.register(Box::new(SparseSearchNodeFactory));
    registry.register(Box::new(FuseResultsNodeFactory));
    registry.register(Box::new(ResolveParentNodeFactory));
    // Record nodes
    registry.register(Box::new(InsertRecordNodeFactory));
    registry.register(Box::new(LinkRecordNodeFactory));
    registry.register(Box::new(KBEmbedNodeFactory));
    registry.register(Box::new(EmbedNodeFactory));
    registry.register(Box::new(ChunkRecordNodeFactory));
    registry.register(Box::new(KBChunkRecordNodeFactory));
    registry.register(Box::new(KBGatherNodeFactory));
    registry.register(Box::new(KBUpdateNodeFactory));
    registry.register(Box::new(KBChunkNodeFactory));
    registry.register(Box::new(FlushNodeFactory));
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
    fn register_builtins_has_all_22_types() {
        let registry = builtin_registry();
        assert_eq!(registry.types().len(), 22);
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
            .create("KBEmbedNode", "embed_0", &config)
            .unwrap();
        assert_eq!(node.node_type(), "KBEmbedNode");
        assert_eq!(node.name(), "embed_0");
    }

    #[test]
    fn embed_record_factory_default_batch_size() {
        let registry = builtin_registry();
        let node = registry
            .create("KBEmbedNode", "embed_1", &serde_json::json!({}))
            .unwrap();
        assert_eq!(node.node_type(), "KBEmbedNode");
    }

    #[test]
    fn query_source_factory_with_config() {
        let registry = builtin_registry();
        let config = serde_json::json!({
            "kb_name": "test_kb",
            "query": "hello world",
        });
        let node = registry
            .create("KBQuerySourceNode", "qs", &config)
            .unwrap();
        assert_eq!(node.node_type(), "KBQuerySourceNode");
    }

    #[test]
    fn query_source_factory_missing_kb_name_errors() {
        let registry = builtin_registry();
        let result = registry.create(
            "KBQuerySourceNode",
            "qs",
            &serde_json::json!({ "query": "hello" }),
        );
        match result {
            Err(e) => assert!(e.contains("missing 'kb_name'"), "got: {e}"),
            Ok(_) => panic!("expected error"),
        }
    }
}

//! Built-in node factories for the NodeRegistry.
//!
//! - Macro-generated factories for simple/named nodes
//! - Manual factories for nodes with config params
//! - `register_builtins()` populates a registry with all 28 node types

use crate::named_factory;

use super::node_registry::{
    Choices,
    ConfigParam, ConfigParamType, NodeFactory, NodeRegistry, NodeSchema,
};
use super::port::{PortDef, PortType};
use super::search_nodes::{
    ComposeNode, FetchRelatedNode, KBSearchNode, KBQuerySourceNode,
};
use super::generic_search_nodes::{
    SearchSourceNode, VectorSearchNode, BM25SearchNode,
    SparseSearchNode, FuseResultsNode, RerankNode, ResolveParentNode,
};
use super::record_nodes::{
    ChunkRecordNode, DeleteRecordNode, EmbedNode, KBChunkNode, KBChunkRecordNode, KBEmbedNode,
    FlushNode, SparseCommitNode, KBGatherNode, InsertRecordNode, LinkRecordNode, KBUpdateNode,
    RechunkDeleteNode, UpdateRecordNode,
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

named_factory!(
    DeleteRecordNodeFactory,
    DeleteRecordNode,
    "DeleteRecordNode",
    "Batch cascade-delete entities + chunks from Vec<DeleteRecord>",
    &[
        PortDef { name: "deletes", port_type: PortType::Deletes, required: true },
        PortDef { name: "trigger", port_type: PortType::Empty, required: false },
    ],
    &[
        PortDef { name: "done", port_type: PortType::Empty, required: false },
    ],
);

named_factory!(
    UpdateRecordNodeFactory,
    UpdateRecordNode,
    "UpdateRecordNode",
    "Batch field update + change detection from Vec<UpdateRecord>",
    &[
        PortDef { name: "updates", port_type: PortType::Updates, required: true },
        PortDef { name: "trigger", port_type: PortType::Empty, required: false },
    ],
    &[
        PortDef { name: "done", port_type: PortType::Empty, required: false },
        PortDef { name: "rechunk_entities", port_type: PortType::Entities, required: false },
    ],
);

named_factory!(
    RechunkDeleteNodeFactory,
    RechunkDeleteNode,
    "RechunkDeleteNode",
    "Delete old chunks before re-chunking",
    &[
        PortDef { name: "entities", port_type: PortType::Entities, required: true },
    ],
    &[
        PortDef { name: "entities", port_type: PortType::Entities, required: false },
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
                    choices: None,
                    json_schema: None,
                },
            ],
            inputs: vec![PortDef { name: "trigger", port_type: PortType::Empty, required: false }],
            outputs: vec![PortDef { name: "done", port_type: PortType::Empty, required: false }],
        }
    }
}

/// Factory for SparseCommitNode (config: table or tables).
pub struct SparseCommitNodeFactory;

impl NodeFactory for SparseCommitNodeFactory {
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
            return Err("SparseCommitNode: missing 'table' or 'tables' config".to_string());
        };
        Ok(Box::new(SparseCommitNode::new(name, tables)))
    }

    fn node_type(&self) -> &'static str {
        "SparseCommitNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "SparseCommitNode",
            description: "Commit dirty sparse vector indexes for configured tables",
            config_params: vec![
                ConfigParam {
                    name: "table",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Single table name to commit",
                    choices: None,
                    json_schema: None,
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
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "query",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Search query text",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "options",
                    param_type: ConfigParamType::Json,
                    required: false,
                    default: None,
                    description: "SearchOptions as JSON",
                    choices: None,
                    json_schema: None,
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
            "Outgoing" => ExpansionDirection::Outgoing,
            other => return Err(format!("FetchRelatedNode: unknown direction '{other}' (Outgoing | Incoming)")),
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
                    choices: Some(Choices::Relations),
                    json_schema: None,
                },
                ConfigParam {
                    name: "direction",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("Outgoing")),
                    description: "Outgoing or Incoming",
                    choices: Some(Choices::fixed(["Outgoing", "Incoming"])),
                    json_schema: None,
                },
                ConfigParam {
                    name: "limit",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(10)),
                    description: "Max children per parent (0 = unlimited)",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "source_entity",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Filter results by source entity type",
                    choices: None,
                    json_schema: None,
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
                choices: None,
                json_schema: None,
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
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "signals",
                    param_type: ConfigParamType::Json,
                    required: false,
                    default: Some(serde_json::json!(["bm25", "vector"])),
                    description: "Search signals array (bm25, vector, sparse)",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "text_field",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("_text")),
                    description: "Field name containing text to embed",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "embedding_col",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("embedding")),
                    description: "Column name for dense embeddings",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "sparse_col",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("sparse")),
                    description: "Prefix for sparse columns ({prefix}_indices, {prefix}_weights)",
                    choices: None,
                    json_schema: None,
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
                    choices: Some(Choices::Targets),
                    json_schema: None,
                },
                ConfigParam {
                    name: "query",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Search query text",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "options",
                    param_type: ConfigParamType::Json,
                    required: false,
                    default: None,
                    description: "SearchOptions as JSON",
                    choices: None,
                    json_schema: None,
                },
            ],
        }
    }
}

// ─── Paramètres communs aux nœuds de signal ─────────────────────────────────

fn parse_result_mode(config: &serde_json::Value, node: &str) -> Result<crate::search::ResultMode, String> {
    use crate::search::ResultMode;
    Ok(match config.get("result_mode").and_then(|v| v.as_str()) {
        // La forme serde, et elle seule : c'est celle que les nœuds réémettent
        // et celle que l'`enum` de la fiche annonce. Les alias PascalCase
        // d'autrefois n'avaient plus aucun appelant.
        None | Some("aggregated") => ResultMode::Aggregated,
        Some("detailed") => ResultMode::Detailed,
        Some("source_resolved") => ResultMode::SourceResolved,
        Some(other) => return Err(format!("{node}: unknown result_mode '{other}' (aggregated | detailed | source_resolved)")),
    })
}

/// Liste de chaînes, soit un tableau JSON, soit `"a,b,c"`.
fn parse_str_list(v: &serde_json::Value, node: &str, key: &str) -> Result<Vec<String>, String> {
    match v {
        serde_json::Value::Array(a) => a
            .iter()
            .map(|x| x.as_str().map(String::from).ok_or_else(|| format!("{node}: '{key}' items must be strings")))
            .collect(),
        serde_json::Value::String(s) => Ok(s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()),
        _ => Err(format!("{node}: '{key}' must be a JSON array or a comma-separated string")),
    }
}

fn signal_param() -> ConfigParam {
    ConfigParam {
        name: "signal",
        param_type: ConfigParamType::String,
        required: false,
        default: None,
        description: "Étiquette des résultats (défaut : nom du nœud) ; sert à la fusion par le port 'signals'",
        choices: None,
        json_schema: None,
    }
}

fn result_mode_param() -> ConfigParam {
    ConfigParam {
        name: "result_mode",
        param_type: ConfigParamType::String,
        required: false,
        default: Some(serde_json::json!("aggregated")),
        description: "aggregated | detailed | source_resolved (KB → entité source, pour fusionner plusieurs KB)",
        choices: Some(Choices::fixed(["aggregated", "detailed", "source_resolved"])),
        json_schema: None,
    }
}

fn limit_param() -> ConfigParam {
    ConfigParam {
        name: "limit",
        param_type: ConfigParamType::Int,
        required: false,
        default: Some(serde_json::json!(10)),
        description: "Max results to return",
        choices: None,
        json_schema: None,
    }
}

fn query_in() -> PortDef {
    PortDef { name: "query", port_type: PortType::Query, required: true }
}

fn results_out() -> PortDef {
    PortDef { name: "results", port_type: PortType::Results, required: false }
}

/// Factory for VectorSearchNode (config: limit, result_mode, signal).
pub struct VectorSearchNodeFactory;

impl NodeFactory for VectorSearchNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let limit = config.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let mut node = VectorSearchNode::new(name, limit)
            .with_result_mode(parse_result_mode(config, "VectorSearchNode")?);
        if let Some(sig) = config.get("signal").and_then(|v| v.as_str()) {
            node = node.with_signal(sig);
        }
        Ok(Box::new(node))
    }

    fn node_type(&self) -> &'static str {
        "VectorSearchNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "VectorSearchNode",
            description: "Vector similarity search on chunk embeddings",
            inputs: vec![query_in()],
            outputs: vec![results_out()],
            config_params: vec![limit_param(), result_mode_param(), signal_param()],
        }
    }
}

/// Factory for BM25SearchNode (config: limit, fuzzy_distance, result_mode, mode, fields, signal).
pub struct BM25SearchNodeFactory;

impl NodeFactory for BM25SearchNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let limit = config.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let fuzzy_distance = config.get("fuzzy_distance").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let mut node = BM25SearchNode::new(name, limit)
            .with_fuzzy(fuzzy_distance)
            .with_result_mode(parse_result_mode(config, "BM25SearchNode")?);
        if let Some(mode) = config.get("mode") {
            let mode: crate::search::BM25Mode = serde_json::from_value(mode.clone())
                .map_err(|e| format!("BM25SearchNode: invalid 'mode': {e}"))?;
            node = node.with_mode(mode);
        }
        if let Some(fields) = config.get("fields") {
            let fields = parse_str_list(fields, "BM25SearchNode", "fields")?;
            if fields.is_empty() {
                return Err("BM25SearchNode: 'fields' must name at least one field".into());
            }
            node = node.with_fields(fields);
        }
        if let Some(sig) = config.get("signal").and_then(|v| v.as_str()) {
            node = node.with_signal(sig);
        }
        Ok(Box::new(node))
    }

    fn node_type(&self) -> &'static str {
        "BM25SearchNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "BM25SearchNode",
            description: "BM25 full-text search with highlight→chunk resolution",
            inputs: vec![query_in()],
            outputs: vec![results_out()],
            config_params: vec![
                limit_param(),
                ConfigParam {
                    name: "fuzzy_distance",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(0)),
                    description: "Levenshtein distance for fuzzy matching (0 = exact)",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "mode",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("contains")),
                    description: "contains | contains_split | regex | parse | symbol (exact, séparateurs compris)",
                    choices: Some(Choices::fixed(["contains", "contains_split", "regex", "parse", "symbol"])),
                    json_schema: None,
                },
                ConfigParam {
                    name: "fields",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Champs indexés à interroger, 'a,b' (défaut : tous ceux de la cible) — une branche par champ pour les peser à la fusion",
                    choices: None,
                    json_schema: None,
                },
                result_mode_param(),
                signal_param(),
            ],
        }
    }
}

/// Factory for SparseSearchNode (config: limit, result_mode, signal).
pub struct SparseSearchNodeFactory;

impl NodeFactory for SparseSearchNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let limit = config.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let mut node = SparseSearchNode::new(name, limit)
            .with_result_mode(parse_result_mode(config, "SparseSearchNode")?);
        if let Some(sig) = config.get("signal").and_then(|v| v.as_str()) {
            node = node.with_signal(sig);
        }
        Ok(Box::new(node))
    }

    fn node_type(&self) -> &'static str {
        "SparseSearchNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "SparseSearchNode",
            description: "Sparse vector search (SPLADE/BGE-M3)",
            inputs: vec![query_in()],
            outputs: vec![results_out()],
            config_params: vec![limit_param(), result_mode_param(), signal_param()],
        }
    }
}

/// Factory for FuseResultsNode (config: strategy, rrf_k, weights, boost, top_k, signal).
pub struct FuseResultsNodeFactory;

impl NodeFactory for FuseResultsNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let mut node = FuseResultsNode::new(name);
        if let Some(strategy) = config.get("strategy") {
            let strategy: crate::search::FusionStrategy = serde_json::from_value(strategy.clone())
                .map_err(|e| format!("FuseResultsNode: invalid 'strategy': {e}"))?;
            node = node.with_strategy(strategy);
        }
        if let Some(k) = config.get("rrf_k") {
            let k = k.as_f64().ok_or("FuseResultsNode: 'rrf_k' must be a number")?;
            if k <= 0.0 {
                return Err("FuseResultsNode: 'rrf_k' must be positive".into());
            }
            node = node.with_rrf_k(k);
        }
        // `weights` : objet JSON {"label": w} ou chaîne "label:w,label:w".
        if let Some(w) = config.get("weights") {
            match w {
                serde_json::Value::Object(m) => {
                    for (label, v) in m {
                        let v = v.as_f64().ok_or_else(|| format!("FuseResultsNode: weight of '{label}' must be a number"))?;
                        node = node.with_weight(label.clone(), v);
                    }
                }
                serde_json::Value::String(s) => {
                    for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                        let (label, v) = part
                            .split_once(':')
                            .ok_or_else(|| format!("FuseResultsNode: weights entry '{part}' must be 'label:weight'"))?;
                        let v: f64 = v.trim().parse()
                            .map_err(|_| format!("FuseResultsNode: weight of '{label}' must be a number"))?;
                        node = node.with_weight(label.trim(), v);
                    }
                }
                _ => return Err("FuseResultsNode: 'weights' must be an object or 'label:weight,…'".into()),
            }
        }
        if let Some(b) = config.get("boost") {
            for label in parse_str_list(b, "FuseResultsNode", "boost")? {
                node = node.with_boost(label);
            }
        }
        if let Some(k) = config.get("top_k") {
            let k = k.as_u64().ok_or("FuseResultsNode: 'top_k' must be an integer")? as usize;
            node = node.with_top_k(k);
        }
        if let Some(sig) = config.get("signal").and_then(|v| v.as_str()) {
            node = node.with_signal(sig);
        }
        Ok(Box::new(node))
    }

    fn node_type(&self) -> &'static str {
        "FuseResultsNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "FuseResultsNode",
            description: "Fusion N-aire de signaux étiquetés (RRF ou pondérée) ; ports nommés + port 'signals' en fan-in",
            inputs: vec![
                PortDef { name: "vector", port_type: PortType::Results, required: false },
                PortDef { name: "bm25", port_type: PortType::Results, required: false },
                PortDef { name: "sparse", port_type: PortType::Results, required: false },
                PortDef { name: "signals", port_type: PortType::Results, required: false },
            ],
            outputs: vec![results_out()],
            config_params: vec![
                ConfigParam {
                    name: "strategy",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("rrf")),
                    description: "rrf | weighted",
                    choices: Some(Choices::fixed(["rrf", "weighted"])),
                    json_schema: None,
                },
                ConfigParam {
                    name: "rrf_k",
                    param_type: ConfigParamType::Float,
                    required: false,
                    default: Some(serde_json::json!(crate::search::DEFAULT_RRF_K)),
                    description: "Constante RRF",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "weights",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Poids par étiquette, 'label:w,label:w' (défauts : vector 0.7, bm25 0.3, sparse 0.2, autres 1.0)",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "boost",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Étiquettes en rôle boost, 'a,b' : elles modulent le score fusionné au lieu d'y entrer",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "top_k",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: None,
                    description: "Troncature de chaque liste avant fusion",
                    choices: None,
                    json_schema: None,
                },
                signal_param(),
            ],
        }
    }
}

/// Factory for RerankNode (config: candidates, service, signal).
pub struct RerankNodeFactory;

impl NodeFactory for RerankNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn super::node::Node>, String> {
        let mut node = RerankNode::new(name);
        if let Some(n) = config.get("candidates") {
            let n = n.as_u64().ok_or("RerankNode: 'candidates' must be an integer")? as usize;
            if n == 0 {
                return Err("RerankNode: 'candidates' must be at least 1".into());
            }
            node = node.with_candidates(n);
        }
        if let Some(svc) = config.get("service").and_then(|v| v.as_str()) {
            node = node.with_service(svc);
        }
        if let Some(sig) = config.get("signal").and_then(|v| v.as_str()) {
            node = node.with_signal(sig);
        }
        Ok(Box::new(node))
    }

    fn node_type(&self) -> &'static str {
        "RerankNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "RerankNode",
            description: "Cross-encoder sur la tête des résultats ; la queue passe inchangée",
            inputs: vec![
                PortDef { name: "results", port_type: PortType::Results, required: true },
                query_in(),
            ],
            outputs: vec![results_out()],
            config_params: vec![
                ConfigParam {
                    name: "candidates",
                    param_type: ConfigParamType::Int,
                    required: false,
                    default: Some(serde_json::json!(RerankNode::DEFAULT_CANDIDATES)),
                    description: "Taille du pool re-scoré",
                    choices: None,
                    json_schema: None,
                },
                ConfigParam {
                    name: "service",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("reranker")),
                    description: "Clé du service Arc<dyn Reranker> (plusieurs cross-encoders possibles dans un graphe)",
                    choices: None,
                    json_schema: None,
                },
                signal_param(),
            ],
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
                choices: None,
                json_schema: None,
            }],
        }
    }
}

// ─── register_builtins ──────────────────────────────────────────────────────

/// Populate a NodeRegistry with all 28 built-in node types.
pub fn register_builtins(registry: &mut NodeRegistry) {
    // Search nodes (KB)
    registry.register(Box::new(ComposeNodeFactory));
    registry.register(Box::new(KBSearchNodeFactory));
    registry.register(Box::new(KBQuerySourceNodeFactory));
    registry.register(Box::new(FetchRelatedNodeFactory));
    // Trace : le consommateur du bus d'événements, en graphe.
    registry.register(Box::new(super::trace_nodes::EventSourceNodeFactory));
    registry.register(Box::new(super::trace_nodes::TraceSinkNodeFactory));
    registry.register(Box::new(super::trace_nodes::SendMessageNodeFactory));
    // Search nodes (generic)
    registry.register(Box::new(SearchSourceNodeFactory));
    registry.register(Box::new(VectorSearchNodeFactory));
    registry.register(Box::new(BM25SearchNodeFactory));
    registry.register(Box::new(SparseSearchNodeFactory));
    registry.register(Box::new(FuseResultsNodeFactory));
    registry.register(Box::new(RerankNodeFactory));
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
    registry.register(Box::new(DeleteRecordNodeFactory));
    registry.register(Box::new(UpdateRecordNodeFactory));
    registry.register(Box::new(RechunkDeleteNodeFactory));
    registry.register(Box::new(FlushNodeFactory));
    registry.register(Box::new(SparseCommitNodeFactory));
    // Migration nodes
    registry.register(Box::new(CypherNodeFactory));
    registry.register(Box::new(ValidateNodeFactory));
    // Média
    registry.register(Box::new(super::ocr_nodes::OcrNodeFactory));
    // Génération
    registry.register(Box::new(super::llm_nodes::LlmNodeFactory));
    // Code
    #[cfg(feature = "code")]
    {
        registry.register(Box::new(super::code_nodes::ParseCodeNodeFactory));
        registry.register(Box::new(super::code_nodes::CodeIngestNodeFactory));
        registry.register(Box::new(super::code_nodes::ReadFileNodeFactory));
        registry.register(Box::new(super::code_nodes::GrepNodeFactory));
        registry.register(Box::new(super::code_nodes::ListFilesNodeFactory));
        registry.register(Box::new(super::code_nodes::EditFileNodeFactory));
    }
}

/// Nombre de types de nœuds enregistrés par [`register_builtins`] — les tests
/// de comptage le lisent ici pour suivre les features.
pub const BUILTIN_NODE_COUNT: usize = 32 + if cfg!(feature = "code") { 6 } else { 0 };

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerated_params_are_exact_lists_without_aliases() {
        let mut r = NodeRegistry::new();
        register_builtins(&mut r);
        let refused = |node_type: &str, config: serde_json::Value| -> String {
            r.create(node_type, "n", &config).err().map(|e| e.to_string()).unwrap_or_default()
        };
        // Les alias d'autrefois ne passent plus, et l'erreur dit la liste.
        let e = refused("BM25SearchNode", serde_json::json!({"result_mode": "Aggregated"}));
        assert!(e.contains("'Aggregated'") && e.contains("aggregated, detailed, source_resolved"), "{e}");
        let e = refused("BM25SearchNode", serde_json::json!({"mode": "containsSplit"}));
        assert!(e.contains("contains, contains_split, regex, parse, symbol"), "{e}");
        let e = refused("FuseResultsNode", serde_json::json!({"strategy": "RRF"}));
        assert!(e.contains("rrf, weighted"), "{e}");
        // Le parseur lui-même est strict aussi, pas seulement le registre.
        let e = FetchRelatedNodeFactory
            .create("f", &serde_json::json!({"relation": "R", "direction": "Sideways"}))
            .err()
            .unwrap_or_default();
        assert!(e.contains("unknown direction 'Sideways'"), "{e}");
        assert!(parse_result_mode(&serde_json::json!({"result_mode": "SourceResolved"}), "n").is_err());
        // Les schémas publient les listes.
        let modes = r.schema("BM25SearchNode").unwrap().config_params.into_iter().find(|p| p.name == "mode").unwrap();
        assert_eq!(modes.choices, Some(Choices::fixed(["contains", "contains_split", "regex", "parse", "symbol"])));
    }


    fn builtin_registry() -> NodeRegistry {
        let mut registry = NodeRegistry::new();
        register_builtins(&mut registry);
        registry
    }

    #[test]
    fn register_builtins_has_all_29_types() {
        let registry = builtin_registry();
        assert_eq!(registry.types().len(), BUILTIN_NODE_COUNT);
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

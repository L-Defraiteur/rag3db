//! Dataflow graph framework.
//!
//! Replaces the round-based `SearchQueue` with a typed DAG of nodes.
//! Each node has typed input/output ports, and data flows through edges.
//!
//! ## Observability (Phase 2)
//!
//! - [`observe`] — Tap per-edge for debug (zero cost when inactive)
//! - [`report`] — Serializable [`ExecutionReport`] built from events
//! - [`record`] — Persist reports to rag3db or JSONL

pub mod checkpoint;
pub mod checkpoint_store;
pub mod graph;
pub mod luciole_bridge;
pub mod node;
pub mod observe;
pub mod port;
pub mod record;
pub mod record_nodes;
pub mod report;
pub mod runtime;
pub mod graph_node;
pub mod graph_tool;
pub mod mermaid;
pub mod node_factories;
pub mod node_registry;
pub mod search_nodes;
pub mod generic_search_nodes;
pub mod migration_nodes;
pub mod llm_nodes;
pub mod ocr_nodes;
pub mod migrations;
pub mod services;

pub use graph::{DataflowGraph, Edge};
pub use node::{Node, NodeContext, NodeLogLevel, NodeLogEntry};
pub use observe::{TapEvent, TapSpec};
pub use port::{merge_port_values, BatchPayload, PortDef, PortType, PortValue, QueryPayload};
pub use record::{DataflowRecorder, RecordRetention, RecordSink};
pub use report::{ExecutionReport, ExecutionStatus, NodeReport, EdgeReport, NodeStatus};
pub use luciole_bridge::{execute_via_luciole, execute_via_luciole_with_result};
pub use runtime::{DataflowEvent, DataflowOutput, DataflowRuntime, NodeEventFilter};
pub use search_nodes::{
    ComposeNode, FetchRelatedNode, KBSearchNode, KBQuerySourceNode,
};
pub use generic_search_nodes::{
    SearchSourceNode, VectorSearchNode, BM25SearchNode,
    SparseSearchNode, FuseResultsNode, RerankNode, ResolveParentNode,
};
pub use services::{ConnService, ServiceRegistry};
pub use node_registry::{NodeSchema, NodeFactory, NodeRegistry, ConfigParam, ConfigParamType};
pub use graph_node::{GraphNode, GraphNodeFactory};
pub use graph_tool::{
    build_definition, builtin_graph_tools, execute_definition, param_type_name, resolve_params,
    run_definition_as_tool_content, substitute_definition, template_vars, validate_node_types,
    GraphTool, GraphToolError, GraphToolRegistry, NodeTypePolicy, SEARCH_EXPAND_TOOL_MERMAID,
    SEARCH_TOOL_MERMAID, SEARCH_TOOL_NODE_TYPE,
};
pub use migration_nodes::{CypherNode, CypherNodeFactory, ValidateNode, ValidateNodeFactory, Assertion};
pub use migrations::{MigrationRunner, MigrationFile, MigrationStatus, MigrationState, MigrationResult, MigrationError};
pub use mermaid::{parse_mermaid, parse_mermaid_template, to_mermaid, MermaidError};
pub use node_factories::register_builtins;
pub use ocr_nodes::{OcrNode, OcrNodeFactory, OCR_SERVICE};
pub use llm_nodes::{LlmNode, LlmNodeFactory, LLM_SERVICE, NODE_REGISTRY_SERVICE};
pub use checkpoint::{
    CheckpointPortValue, port_value_to_checkpoint, port_value_from_checkpoint,
    GraphDefinition, NodeDef, EdgeDef,
    CheckpointStore, ExecutionCheckpoint, CheckpointExecutionStatus,
    NodeCheckpoint, NodeCheckpointStatus, timestamp_ms,
};
pub use checkpoint_store::CypherCheckpointStore;
pub use record_nodes::{
    InsertRecordNode, LinkRecordNode, KBEmbedNode,
    ChunkRecordNode, EmbedNode, KBChunkRecordNode, KBGatherNode, KBUpdateNode, KBChunkNode, FlushNode,
    SparseCommitNode, DeleteRecordNode, UpdateRecordNode, RechunkDeleteNode,
};

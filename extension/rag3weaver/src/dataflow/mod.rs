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
pub mod node;
pub mod observe;
pub mod port;
pub mod record;
pub mod record_nodes;
pub mod report;
pub mod runtime;
pub mod graph_node;
pub mod mermaid;
pub mod node_factories;
pub mod node_registry;
pub mod search_nodes;
pub mod migration_nodes;
pub mod migrations;
pub mod services;

pub use graph::{DataflowGraph, Edge};
pub use node::{Node, NodeContext};
pub use observe::{TapEvent, TapSpec};
pub use port::{merge_port_values, BatchPayload, PortDef, PortType, PortValue};
pub use record::{DataflowRecorder, RecordRetention, RecordSink};
pub use report::{ExecutionReport, ExecutionStatus, NodeReport, EdgeReport, NodeStatus};
pub use runtime::{DataflowEvent, DataflowOutput, DataflowRuntime, NodeEventFilter};
pub use search_nodes::{
    ComposeNode, FetchRelatedNode, PrimarySearchNode, QuerySourceNode,
};
pub use services::{ConnService, ServiceRegistry};
pub use node_registry::{NodeSchema, NodeFactory, NodeRegistry, ConfigParam, ConfigParamType};
pub use graph_node::{GraphNode, GraphNodeFactory};
pub use migration_nodes::{CypherNode, CypherNodeFactory, ValidateNode, ValidateNodeFactory, Assertion};
pub use migrations::{MigrationRunner, MigrationFile, MigrationStatus, MigrationState, MigrationResult, MigrationError};
pub use mermaid::{parse_mermaid, parse_mermaid_template, to_mermaid, MermaidError};
pub use node_factories::register_builtins;
pub use checkpoint::{
    CheckpointPortValue, port_value_to_checkpoint, port_value_from_checkpoint,
    GraphDefinition, NodeDef, EdgeDef,
    CheckpointStore, ExecutionCheckpoint, CheckpointExecutionStatus,
    NodeCheckpoint, NodeCheckpointStatus, timestamp_ms,
};
pub use checkpoint_store::CypherCheckpointStore;
pub use record_nodes::{
    InsertRecordNode, LinkRecordNode, EmbedRecordNode,
    ChunkRecordNode, GatherKBNode, UpdateKBNode, ChunkKBNode, FlushFTSNode,
};

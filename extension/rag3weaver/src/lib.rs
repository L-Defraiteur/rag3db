//! rag3weaver: RAG pipeline orchestrator for rag3db.
//!
//! Provides typed events, config parsing, and async traits for embedding
//! and database access. Designed to be runtime-agnostic (WASM-compatible).

#[cfg(feature = "bge-m3")]
pub mod bge_m3_embedder;
#[cfg(any(feature = "candle-embedder", feature = "candle-wasm"))]
pub mod bm42_model;
#[cfg(any(feature = "candle-embedder", feature = "candle-wasm"))]
pub mod bm42_embedder;
#[cfg(any(feature = "candle-embedder", feature = "candle-wasm"))]
pub mod candle_embedder;
pub mod catalog;
#[cfg(feature = "rag3db-native")]
pub mod rag3db_connection;
#[cfg(feature = "wasm-emscripten")]
pub mod wasm_ffi;
pub mod chunker;
pub mod config;
pub mod connection;
pub mod embedder;
pub mod events;
pub mod filter;
pub mod fusion;
pub mod hash;
pub mod node_id_cache;
pub mod records;
pub mod dataflow;
pub mod query;
pub mod refs;
pub mod schema;
pub mod search;
pub mod search_strategy;
pub mod sparse_index;
pub mod uuid;
pub mod validator;

pub use chunker::{Chunk, Chunker, ChunkerConfig};
pub use config::{CatalogConfig, EntityConfig, SimpleFieldDef};
pub use connection::{CallbackConnection, DbConnection};
pub use embedder::{CallbackDualEmbedder, CallbackEmbedder, CallbackSparseEmbedder, DualEmbedFn, DualEmbedder, EmbedError, EmbedFn, Embedder, SparseEmbedder};
pub use events::{CatalogEvent, EventBus};
pub use filter::{FilterBuilder, FilterCondition, FilterOp, FilterParser, FilterValue, ParsedFilter};
pub use hash::content_hash;
pub use node_id_cache::{InternalNodeId, NodeIdCache};
pub use records::{EntityRecord, RelationRecord, AggregateRecord, PendingWork, RefOrUuid, FlushResult, DrainStats};
pub use query::{PreparedQuery, QueryBuilder};
pub use refs::{EntityRef, EntityRefResolver, RefError, RelResolved, RelationRef, RelationRefResolver};
pub use schema::{generate_full_schema, FullSchema};
pub use sparse_index::SparseVector;
pub use uuid::{chunk_uuid, hashsafe_uuid};
pub use catalog::{Catalog, CatalogError, DeleteResult, KBMetadata, UpdateResult, UpdateStatus};
pub use search::{
    AttributedChunk, BM25HitDiagnostic, BM25Mode, BoostType, ChunkInfo, ChunkOverlapDiag,
    Consistency, ExploreGraph, ExploreOptions, ExploreResult, FusionConfig, FusionStrategy,
    GraphEdge, GraphNode, NormalizeMode, ResultMode, SearchDiagnostics, SearchMeta,
    SearchOptions, SearchResponse, SearchResult, SearchSignals, SignalConfig, SignalRole,
};
pub use search_strategy::{
    UnifiedResult, ChildSummary, SearchStrategy, SearchStrategyResponse,
    ExpansionRule, ExpansionDirection,
};
pub use validator::{validate_schema, KBFieldRef};
#[cfg(feature = "rag3db-native")]
pub use rag3db_connection::Rag3dbConnection;
#[cfg(feature = "wasm-emscripten")]
pub use wasm_ffi::WasmDbConnection;

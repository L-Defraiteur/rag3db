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
pub mod cypher_persistence;
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
pub mod ops;
pub mod persistence;
pub mod query;
pub mod queue;
pub mod refs;
pub mod schema;
pub mod search;
pub mod sparse_index;
pub mod uuid;
pub mod validator;

pub use chunker::{Chunk, Chunker, ChunkerConfig};
pub use config::CatalogConfig;
pub use connection::{CallbackConnection, DbConnection};
pub use embedder::{CallbackEmbedder, CallbackSparseEmbedder, EmbedError, EmbedFn, Embedder, SparseEmbedder};
pub use events::{CatalogEvent, EventBus};
pub use filter::{FilterBuilder, FilterCompiler, FilterCondition, FilterOp, FilterParser, FilterValue, ParsedFilter, SplitResult};
pub use hash::content_hash;
pub use node_id_cache::{InternalNodeId, NodeIdCache};
pub use ops::{CatalogOp, EmbedOp, InsertOp, LinkOp, SparseEmbedOp, OperationConfig, RefOrUuid, OP_EMBED, OP_INSERT, OP_LINK, OP_SPARSE_EMBED};
pub use persistence::{OperationPersistence, PersistedOp};
pub use query::{PreparedQuery, QueryBuilder};
pub use queue::{FlushConfig, FlushResult, ItemState, OperationQueue, Processor, QueueEvent, QueueStats};
pub use refs::{EntityRef, EntityRefResolver, RefError, RelResolved, RelationRef, RelationRefResolver};
pub use schema::{generate_full_schema, FullSchema};
pub use sparse_index::SparseVector;
pub use uuid::{chunk_uuid, hashsafe_uuid};
pub use catalog::{Catalog, CatalogError, DeleteResult, KBMetadata, UpdateResult, UpdateStatus};
pub use search::{
    BM25Mode, Consistency, ExploreGraph, ExploreOptions, ExploreResult, GraphEdge, GraphNode,
    HybridStrategy, SearchMeta, SearchOptions, SearchResponse, SearchResult, SearchType,
};
pub use validator::{validate_schema, KBFieldRef};
pub use cypher_persistence::CypherPersistence;
#[cfg(feature = "rag3db-native")]
pub use rag3db_connection::Rag3dbConnection;
#[cfg(feature = "wasm-emscripten")]
pub use wasm_ffi::WasmDbConnection;

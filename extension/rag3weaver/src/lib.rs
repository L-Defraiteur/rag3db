//! rag3weaver: RAG pipeline orchestrator for rag3db.
//!
//! Provides typed events, config parsing, and async traits for embedding
//! and database access. Designed to be runtime-agnostic (WASM-compatible).

pub mod chunker;
pub mod config;
pub mod connection;
pub mod embedder;
pub mod events;
pub mod filter;
pub mod fusion;
pub mod hash;
pub mod ops;
pub mod persistence;
pub mod query;
pub mod queue;
pub mod refs;
pub mod schema;
pub mod uuid;
pub mod validator;

pub use chunker::{Chunk, Chunker, ChunkerConfig};
pub use config::CatalogConfig;
pub use connection::DbConnection;
pub use embedder::Embedder;
pub use events::{CatalogEvent, EventBus};
pub use filter::{FilterParser, FilterValue, ParsedFilter};
pub use hash::content_hash;
pub use ops::{CatalogOp, EmbedOp, InsertOp, LinkOp, OperationConfig, RefOrUuid, OP_EMBED, OP_INSERT, OP_LINK};
pub use persistence::{OperationPersistence, PersistedOp};
pub use query::{PreparedQuery, QueryBuilder};
pub use queue::{FlushConfig, FlushResult, ItemState, OperationQueue, Processor, QueueStats};
pub use refs::{EntityRef, EntityRefResolver, RefError, RelResolved, RelationRef, RelationRefResolver};
pub use schema::{generate_full_schema, FullSchema};
pub use uuid::{chunk_uuid, hashsafe_uuid};
pub use validator::validate_schema;

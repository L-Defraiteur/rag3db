//! rag3weaver: RAG pipeline orchestrator for rag3db.
//!
//! Provides typed events, config parsing, and async traits for embedding
//! and database access. Designed to be runtime-agnostic (WASM-compatible).

// `candle-wasm` est la variante sans hf-hub : elle donne accès à
// `BgeM3Embedder::from_local_dir` sans tirer le téléchargement runtime.
#[cfg(any(feature = "bge-m3", feature = "candle-wasm"))]
pub mod bge_m3_embedder;
// Retiré le 24 août 2026 : `bm42_model` / `bm42_embedder` (sparse par poids
// d'attention, « un hack » de l'aveu même des docs de février) et le
// `CandleDualEmbedder` qui en dépendait. Zéro usage, et la seule brique qui
// aurait exigé un export ONNX côté PyTorch pour passer sur burn. Le sparse
// vient de BGE-M3 (tête apprise), sur candle ou sur burn.
#[cfg(any(feature = "candle-embedder", feature = "candle-wasm"))]
pub mod candle_embedder;
/// Modèle BGE-M3 généré par burn-onnx depuis l'ONNX de BAAI — code machine, non édité.
/// Voir `generated/README.md` pour la provenance et la régénération.
/// Périphérique burn partagé (embedders, rerankers, OCR).
#[cfg(any(feature = "burn-embedder", feature = "burn-ocr"))]
pub mod burn_device;
#[cfg(feature = "burn-embedder")]
#[path = "../generated/bge_m3_onnx.rs"]
#[allow(clippy::all, dead_code, unused_imports)]
pub mod bge_m3_onnx;
#[cfg(feature = "burn-embedder")]
pub mod burn_bge_m3_embedder;
/// all-MiniLM-L6-v2 généré par burn-onnx depuis l'ONNX de sentence-transformers —
/// code machine, non édité. Voir `generated/README.md`.
#[cfg(feature = "burn-embedder")]
#[path = "../generated/minilm_onnx.rs"]
#[allow(clippy::all, dead_code, unused_imports)]
pub mod minilm_onnx;
#[cfg(feature = "burn-embedder")]
pub mod burn_minilm_embedder;
/// paraphrase-multilingual-MiniLM-L12-v2 (BERT 12 couches sur le vocabulaire XLM-R,
/// multilingue) généré par burn-onnx depuis l'ONNX de sentence-transformers — code
/// machine, non édité. Voir `generated/README.md`.
#[cfg(feature = "burn-embedder")]
#[path = "../generated/multilingual_minilm_onnx.rs"]
#[allow(clippy::all, dead_code, unused_imports)]
pub mod multilingual_minilm_onnx;
#[cfg(feature = "burn-embedder")]
pub mod burn_multilingual_minilm_embedder;
#[cfg(feature = "burn-embedder")]
pub use burn_multilingual_minilm_embedder::BurnMultilingualMiniLmEmbedder;
/// cross-encoder/ms-marco-MiniLM-L-6-v2 généré par burn-onnx depuis l'ONNX du
/// modèle — code machine, non édité. Voir `generated/README.md`.
#[cfg(feature = "burn-embedder")]
#[path = "../generated/msmarco_minilm_onnx.rs"]
#[allow(clippy::all, dead_code, unused_imports)]
pub mod msmarco_minilm_onnx;
#[cfg(feature = "burn-embedder")]
pub mod burn_reranker;
#[cfg(feature = "burn-embedder")]
pub use burn_reranker::BurnMiniLmReranker;
/// cross-encoder/mmarco-mMiniLMv2-L12-H384-v1 (XLM-RoBERTa, multilingue) généré par
/// burn-onnx depuis l'ONNX du modèle — code machine, non édité. Voir `generated/README.md`.
#[cfg(feature = "burn-embedder")]
#[path = "../generated/mmarco_mminilm_onnx.rs"]
#[allow(clippy::all, dead_code, unused_imports)]
pub mod mmarco_mminilm_onnx;
/// BAAI/bge-reranker-v2-m3 (XLM-RoBERTa, multilingue) généré par burn-onnx depuis
/// l'ONNX d'onnx-community — code machine, non édité. Voir `generated/README.md`.
#[cfg(feature = "burn-embedder")]
#[path = "../generated/bge_reranker_v2_m3_onnx.rs"]
#[allow(clippy::all, dead_code, unused_imports)]
pub mod bge_reranker_v2_m3_onnx;
#[cfg(feature = "burn-embedder")]
pub mod burn_xlmr_reranker;
#[cfg(feature = "burn-embedder")]
pub use burn_xlmr_reranker::{BurnBgeRerankerV2M3, BurnMMiniLmReranker};
/// PP-OCRv6_tiny_det (DBNet, PPLCNetV4 + RepLKFPN) généré par burn-onnx depuis
/// l'ONNX officiel de PaddlePaddle — code machine, non édité. Voir `generated/README.md`.
#[cfg(feature = "burn-ocr")]
#[path = "../generated/ppocrv6_tiny_det_onnx.rs"]
#[allow(clippy::all, dead_code, unused_imports)]
pub mod ppocrv6_tiny_det_onnx;
/// PP-OCRv6_tiny_rec (PPLCNetV4 + tête CTC, 6904 caractères) généré par burn-onnx
/// depuis l'ONNX officiel de PaddlePaddle — code machine, non édité. Voir `generated/README.md`.
#[cfg(feature = "burn-ocr")]
#[path = "../generated/ppocrv6_tiny_rec_onnx.rs"]
#[allow(clippy::all, dead_code, unused_imports)]
pub mod ppocrv6_tiny_rec_onnx;
#[cfg(feature = "burn-ocr")]
pub mod burn_ppocr;
#[cfg(feature = "burn-ocr")]
pub use burn_ppocr::BurnPpOcr;
pub mod catalog;
pub mod cypher_blob_store;
pub mod buffered_blob_store;
#[cfg(feature = "rag3db-native")]
pub mod rag3db_connection;
pub mod rag3db_search_backend;
#[cfg(feature = "postgres")]
pub mod postgres_connection;
#[cfg(feature = "postgres")]
pub mod postgres_blob_store;
#[cfg(feature = "postgres")]
pub mod postgres_search_backend;
#[cfg(feature = "wasm-emscripten")]
pub mod wasm_ffi;
pub mod chunker;
pub mod config;
pub mod connection;
pub mod dialect;
pub mod embedder;
pub mod events;
pub mod filter;
pub mod fts_handle;
pub mod fusion;
pub mod hash;
pub mod node_id_cache;
pub mod ocr;
pub mod records;
pub mod dataflow;
pub mod query;
pub mod refs;
pub mod schema;
pub mod scope;
pub mod tools;
pub mod reranker;
pub use reranker::{CallbackReranker, MockReranker, Reranker};
pub mod search;
pub mod search_backend;
pub mod search_strategy;
pub mod sparse_index;
pub mod uuid;
pub mod validator;

pub use chunker::{Chunk, Chunker, ChunkerConfig};
pub use config::{CatalogConfig, EntityConfig, SimpleFieldDef};
pub use connection::{CallbackConnection, DbConnection, SyncDbConnection};
pub use embedder::{CallbackDualEmbedder, CallbackEmbedder, CallbackSparseEmbedder, DualEmbedFn, DualEmbedder, EmbedError, EmbedFn, Embedder, SparseEmbedder};
pub use events::{CatalogEvent, EventBus};
pub use filter::{FilterBuilder, FilterCondition, FilterOp, FilterParser, FilterValue, ParsedFilter};
pub use hash::content_hash;
pub use node_id_cache::{InternalNodeId, NodeIdCache};
pub use records::{EntityRecord, RelationRecord, AggregateRecord, UpdateRecord, DeleteRecord, PendingWork, RefOrUuid, FlushResult, DrainStats};
pub use query::{PreparedQuery, QueryBuilder};
pub use refs::{EntityRef, EntityRefResolver, RefError, RelResolved, RelationRef, RelationRefResolver};
pub use schema::{generate_full_schema, FullSchema};
pub use sparse_index::SparseVector;
pub use uuid::{chunk_uuid, hashsafe_uuid};
pub use catalog::{Catalog, CatalogError, KBMetadata, ReindexStats};
pub use records::{DeleteResult, UpdateResult, UpdateStatus};
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

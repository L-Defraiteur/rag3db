//! rag3weaver: RAG pipeline orchestrator for rag3db.
//!
//! Provides typed events, config parsing, and the traits for embedding
//! and database access. Natif seulement : le wasm a été abandonné pour
//! cette crate le 26 août 2026 (lucivy garde le sien) — fils et async sont
//! libres ici.

#[cfg(feature = "bge-m3")]
pub mod bge_m3_embedder;
// Retiré le 24 août 2026 : `bm42_model` / `bm42_embedder` (sparse par poids
// d'attention, « un hack » de l'aveu même des docs de février) et le
// `CandleDualEmbedder` qui en dépendait. Zéro usage, et la seule brique qui
// aurait exigé un export ONNX côté PyTorch pour passer sur burn. Le sparse
// vient de BGE-M3 (tête apprise), sur candle ou sur burn.
#[cfg(feature = "candle-embedder")]
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
/// Le code comme graphe : `File` / `Scope` / `Library` via `codeparsers`.
#[cfg(feature = "code")]
pub mod code;
/// `read` et `grep` sur une `FileSource`, annotés par le graphe.
#[cfg(feature = "code")]
pub mod code_tools;
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
pub mod chunker;
pub mod config;
pub mod connection;
pub mod dialect;
pub mod embedder;
// Fournisseur LLM distant (endpoint compatible OpenAI) et l'authentification
// Google qui va avec. Le trait `Llm` de `llm.rs` existe sans cette feature.
#[cfg(feature = "openai-llm")]
pub mod gcp_auth;
#[cfg(feature = "openai-llm")]
pub mod openai_llm;
pub mod events;
pub mod filter;
pub mod fts_handle;
pub mod fusion;
pub mod hash;
pub mod node_id_cache;
pub mod ocr;
pub mod origin;
pub mod records;
pub mod dataflow;
pub mod query;
pub mod refs;
pub mod schema;
/// **Le régime de travail** : un nom pour la composition « carte, rythme,
/// rafale » qui décide si le poste reste utilisable pendant qu'on travaille.
/// **Exécuter une commande sans donner la machine** : la porte, les faits, le
/// verdict et ce que la session a acquis. Voir
/// `docs/30-aout-2026-04h00/03-donner-des-commandes-a-un-agent.md`.
pub mod commande;
pub mod regime;
pub use regime::Regime;
pub mod postures;
pub mod meter;
pub use meter::{Consumption, Meter, Unit};
pub mod session;
pub use session::{Absorb, Compaction, Session, SessionTools};
pub mod scope;
/// **Le catalogue de gabarits** : ce qu'on pose au lieu de l'écrire — des
/// entités, des graphes, des composants, et des motifs qui s'appliquent aux
/// trois. Voir `docs/vision_roadmap_08_2026/08-des-catalogues-de-gabarits.md`.
pub mod template;
pub mod work_domain;
pub mod tools;
pub mod reranker;
pub use reranker::{CallbackReranker, MockReranker, Reranker};
pub mod llm;
pub use llm::{
    emit, first_stop, holdback,
    CallbackLlm, ChannelSink, CountingSink, Finish, Flow, GenOptions, Llm, LlmError, LlmOutput,
    MockLlm, StringSink, TokenSink, Turn, Usage,
};
/// La boucle d'agent : générer, exécuter les outils, réinjecter, recommencer.
pub mod agent;
pub use agent::{
    Agent, AgentLimits, AgentRun, CallbackToolBox, GraphToolBox, StopReason, ToolBox,
};
/// **Des processus qui tiennent une ressource rare, et la servent** : un modèle
/// chargé une fois (`daemon::embeddings`), et la base qu'un seul processus peut
/// ouvrir (`daemon::db`, alias rag3daemon). Leurs clients se font passer pour un
/// [`Embedder`] et une [`connection::DbConnection`] ordinaires.
#[cfg(feature = "daemon")]
pub mod daemon;
#[cfg(feature = "daemon")]
pub use daemon::{DaemonConnection, DaemonEmbedder, DbDaemon, EmbedDaemon};
/// **Démarrer un serveur, et savoir s'il est déjà là.** Le démon d'embedding,
/// le terminal à plusieurs, et le backend que la boucle étrange devra faire
/// tourner passent tous par là.
pub mod serveur;
pub use serveur::{Attache, Etat, Fin, Serveur, ServeurError, Sonde};
pub mod search;
pub mod search_backend;
/// Jaro et Jaro-Winkler : ordonner un sommet rapporté large par un index.
pub mod jaro;
pub mod search_strategy;
pub mod sparse_index;
pub mod uuid;
pub mod validator;

pub use chunker::{Chunk, Chunker, ChunkerConfig};
pub use config::{CatalogConfig, EntityConfig, SimpleFieldDef};
pub use connection::{CallbackConnection, DbConnection, SyncDbConnection};
pub use embedder::{CallbackDualEmbedder, CallbackEmbedder, CallbackSparseEmbedder, DualEmbedFn, DualEmbedder, EmbedError, EmbedFn, Embedder, SparseEmbedder};
pub use events::{topic, CatalogEvent, Event, EventBus};
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

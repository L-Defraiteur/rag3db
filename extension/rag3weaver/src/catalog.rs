//! Catalog: CRUD facade assembling all rag3weaver pipeline components.
//!
//! The `Catalog` struct is the main entry point. It owns the database connection,
//! embedder, operation queue, and event bus. After `initialize()`, it provides
//! synchronous `create()`/`link()` methods that enqueue operations, and async
//! `drain()` to process them.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use crate::config::{CatalogConfig, ChunkingConfig, EntityDef, FieldType, RelationDef};
use crate::connection::{CypherValue, DbConnection, QueryParam, SyncDbConnection};
use crate::embedder::{DualEmbedder, Embedder, SparseEmbedder};
use crate::events::{CatalogEvent, EventBus};
use crate::filter::{FilterCondition, FilterParser};
use crate::search;
use crate::hash::content_hash;
use crate::node_id_cache::NodeIdCache;
use crate::records::{AggregateRecord, DrainStats, EntityRecord, FlushResult, PendingWork, RefOrUuid, RelationRecord};
use crate::refs::{EntityRef, RelationRef};
use crate::schema::{generate_full_schema_with_dialect, resolve_entity_kbs};
use crate::chunker::{Chunker, ChunkerConfig};
use crate::uuid::hashsafe_uuid;
use crate::validator::{validate_schema, KBFieldRef};
use crate::cypher_blob_store::CypherBlobStore;
use sparse_vector::blob_store::BlobStore;
use crate::dataflow::checkpoint::CheckpointStore;
use crate::dataflow::node_factories::register_builtins;
use crate::dataflow::node_registry::NodeRegistry;
use crate::dataflow::checkpoint_store::CypherCheckpointStore;
use crate::dataflow::graph::DataflowGraph;
use crate::dataflow::port::{BatchPayload, PortType, PortValue};
use crate::dataflow::record_nodes::{
    ChunkRecordNode, DeleteRecordNode, EmbedNode, KBChunkNode, KBEmbedNode, FlushNode,
    KBGatherNode, InsertRecordNode, LinkRecordNode, KBUpdateNode, RechunkDeleteNode,
    UpdateRecordNode,
};
use crate::dataflow::runtime::DataflowRuntime;
use crate::dataflow::services::ServiceRegistry;

// ─── KBMetadata ────────────────────────────────────────────────────────────

/// Resolved metadata for a Knowledge Base, built at `Catalog::initialize()`.
#[derive(Debug, Clone)]
pub struct KBMetadata {
    pub name: String,
    pub title: KBFieldRef,
    pub content: Vec<KBFieldRef>,
    pub entities: HashSet<String>,
    pub signals: search::SearchSignals,
    pub keyword_weight: f64,
    pub title_boost: f64,
    pub content_boost: f64,
    pub chunking: ChunkingConfig,
}

// ─── CatalogError ──────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("not initialized")]
    NotInitialized,
    #[error("unknown entity: {0}")]
    UnknownEntity(String),
    #[error("unknown relation: {0}")]
    UnknownRelation(String),
    #[error("unknown knowledge base: {0}")]
    UnknownKB(String),
    #[error("entity not found: {entity}:{uuid}")]
    NotFound { entity: String, uuid: String },
    #[error("schema validation failed: {0}")]
    ValidationFailed(String),
    #[error("schema error: {0}")]
    SchemaError(String),
    #[error("db error: {0}")]
    DbError(String),
    #[error("embed error: {0}")]
    EmbedError(String),
    #[error("filter error: {0}")]
    FilterError(String),
}

// Re-export result types (defined in records.rs, used widely)
pub use crate::records::{DeleteResult, UpdateResult, UpdateStatus};

/// Stats returned by [`Catalog::reindex()`].
#[derive(Debug, Clone)]
pub struct ReindexStats {
    pub entity: String,
    pub records_processed: usize,
}

// ─── Catalog ───────────────────────────────────────────────────────────────

/// Internal cumulative drain counters (not reset on clear).
#[derive(Debug, Default)]
struct DrainCounters {
    total_queued: usize,
    total_processed: usize,
    total_failed: usize,
    flush_count: usize,
}

pub struct Catalog {
    conn: Arc<dyn DbConnection>,
    embedder: Arc<dyn Embedder>,
    sparse_embedder: Option<Arc<dyn SparseEmbedder>>,
    dual_embedder: Option<Arc<dyn DualEmbedder>>,
    config: CatalogConfig,
    /// Typed pending work queue. Populated by create()/link()/update()/delete(),
    /// consumed by build_ingestion_graph() → drain().
    pending: PendingWork,
    drain_counters: DrainCounters,
    event_bus: EventBus,
    kb_metadata: HashMap<String, KBMetadata>,
    /// Simple entity configs (registerEntity API). Separate from KB metadata.
    entity_configs: HashMap<String, crate::config::EntityConfig>,
    initialized: bool,
    embedding_cache: HashMap<String, Vec<f32>>,
    /// Cache mapping entity UUIDs to rag3db internal node IDs.
    /// Populated by InsertRecordNode on each INSERT via RETURN ID(n).
    node_id_cache: Arc<RwLock<NodeIdCache>>,
    /// Cached chunkers keyed by config to avoid re-instantiation.
    chunker_cache: HashMap<ChunkerConfig, Chunker>,
    /// Checkpoint store for crash-recovery of drain executions.
    checkpoint_store: Option<Arc<dyn CheckpointStore>>,
    /// BlobStore backed by rag3db for lucivy/sparse index persistence.
    /// CypherBlobStore when sync_conn is set, MemBlobStore fallback for in-memory DBs.
    blob_store: Option<Arc<dyn BlobStore>>,
    /// Sparse vector index handles, keyed by table name (e.g. "Product_Chunk").
    sparse_handles: HashMap<String, Arc<sparse_vector::handle::SparseHandle>>,
    /// Index FTS lucivy v3, un par table. Ouverture **paresseuse** : ouvrir un
    /// index Blob télécharge tout l'index, donc on ne le fait qu'au premier
    /// usage réel de la table (cf doc 04 de la passation lucivy).
    fts_handles: HashMap<String, Arc<lucivy_core::sharded_handle::ShardedHandle>>,
    /// Topologie de stockage des index FTS. Voir [`crate::fts_handle::FtsStorage`] :
    /// (a) blob-backed rematérialise tout à chaque ouverture, (b) copie locale
    /// durable + deltas ne le fait jamais. Décision d'archi, pas un réglage.
    fts_storage: crate::fts_handle::FtsStorage,
    /// Base directory for sparse/FTS mmap caches.
    cache_base: PathBuf,
    /// Sync connection for BlobStore (avoids async→sync bridge).
    sync_conn: Option<Arc<dyn SyncDbConnection>>,
    /// Fail injection for testing: if set, the named node will fail during checkpoint execution.
    fail_node: Option<String>,
    /// Schema dialect for multi-backend DDL/DML generation.
    dialect: Arc<dyn crate::dialect::SchemaDialect>,
    /// Search backend for multi-backend search operations.
    search_backend: Option<Arc<dyn crate::search_backend::SearchBackend>>,
}

impl Catalog {
    // ── Lifecycle ───────────────────────────────────────────────────────

    pub fn new(
        conn: Box<dyn DbConnection>,
        embedder: Box<dyn Embedder>,
        config: CatalogConfig,
    ) -> Self {
        Self {
            conn: Arc::from(conn),
            embedder: Arc::from(embedder),
            sparse_embedder: None,
            dual_embedder: None,
            config,
            pending: PendingWork::new(),
            drain_counters: DrainCounters::default(),
            event_bus: EventBus::new(64),
            kb_metadata: HashMap::new(),
            entity_configs: HashMap::new(),
            initialized: false,
            embedding_cache: HashMap::new(),
            node_id_cache: Arc::new(RwLock::new(NodeIdCache::new())),
            chunker_cache: HashMap::new(),
            checkpoint_store: None,
            blob_store: None,
            sparse_handles: HashMap::new(),
            fts_handles: HashMap::new(),
            fts_storage: Default::default(),
            cache_base: std::env::temp_dir().join("rag3weaver_cache"),
            sync_conn: None,
            fail_node: None,
            dialect: Arc::new(crate::dialect::Rag3dbDialect),
            search_backend: None,
        }
    }

    /// Set the schema dialect for multi-backend support.
    /// Must be called before `initialize()`. Defaults to `Rag3dbDialect`.
    pub fn set_dialect(&mut self, dialect: Arc<dyn crate::dialect::SchemaDialect>) {
        self.dialect = dialect;
    }

    /// Set the search backend for multi-backend search operations.
    /// If not set, a `Rag3dbSearchBackend` is created automatically in `initialize()`.
    pub fn set_search_backend(&mut self, backend: Arc<dyn crate::search_backend::SearchBackend>) {
        self.search_backend = Some(backend);
    }

    /// Replace the dense embedder with a shared Arc.
    /// Use this to share a single model instance between dense and sparse roles.
    pub fn set_embedder(&mut self, embedder: Arc<dyn Embedder>) {
        self.embedder = embedder;
    }

    /// Set the sparse embedder (optional). Must be called before `initialize()`.
    /// Accepts `Arc<dyn SparseEmbedder>` to allow sharing with the dense embedder.
    pub fn set_sparse_embedder(&mut self, embedder: Arc<dyn SparseEmbedder>) {
        self.sparse_embedder = Some(embedder);
    }

    /// Set the dual embedder (optional). When set, both dense and sparse embeddings
    /// are computed in a single forward pass via `DualEmbedProcessor`.
    /// Must be called before `initialize()`.
    pub fn set_dual_embedder(&mut self, embedder: Arc<dyn DualEmbedder>) {
        self.dual_embedder = Some(embedder);
    }

    /// Set a custom checkpoint store. Must be called before `initialize()`.
    /// If set, `initialize()` will skip creating the default `CypherCheckpointStore`.
    pub fn set_checkpoint_store(&mut self, store: Arc<dyn CheckpointStore>) {
        self.checkpoint_store = Some(store);
    }

    /// Set the sync database connection for BlobStore operations.
    /// Must be called before `initialize()` for BlobStore to work.
    pub fn set_sync_connection(&mut self, conn: Arc<dyn SyncDbConnection>) {
        self.sync_conn = Some(conn);
    }

    /// Set the base directory for sparse/FTS mmap caches.
    /// Defaults to `$TMPDIR/rag3weaver_cache`.
    pub fn set_cache_base(&mut self, path: PathBuf) {
        self.cache_base = path;
    }

    /// Set a node name that should fail during checkpoint execution (testing only).
    /// The named node will return an injected error instead of executing.
    pub fn set_fail_node(&mut self, node_name: Option<&str>) {
        self.fail_node = node_name.map(|s| s.to_string());
    }

    /// Get the blob store for index persistence. Available after `initialize()`.
    pub fn blob_store(&self) -> Option<Arc<dyn BlobStore>> {
        self.blob_store.clone()
    }

    /// Get a sparse vector index handle by table name.
    pub fn sparse_handle(&self, table: &str) -> Option<Arc<sparse_vector::handle::SparseHandle>> {
        self.sparse_handles.get(table).cloned()
    }

    /// Create or open a sparse handle for a table, storing it in sparse_handles.
    /// No-op if blob_store is not configured or handle already exists.
    fn ensure_sparse_handle(&mut self, table: &str) {
        if self.sparse_handles.contains_key(table) {
            return;
        }
        let Some(ref blob_store) = self.blob_store else { return };
        // Try open first (index may already exist in BlobStore), fall back to create.
        let handle = match sparse_vector::handle::SparseHandle::open_with_store(
            blob_store.clone(), table, &self.cache_base,
        ) {
            Ok(h) => h,
            Err(_) => match sparse_vector::handle::SparseHandle::create_with_store(
                blob_store.clone(), table, &self.cache_base,
            ) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("[rag3weaver] failed to create sparse handle for {table}: {e}");
                    return;
                }
            },
        };
        self.sparse_handles.insert(table.to_string(), Arc::new(handle));
    }

    /// Choisit la topologie de stockage des index FTS.
    ///
    /// À appeler **avant** le premier `ensure_fts_handle` : les handles déjà
    /// ouverts gardent leur stockage d'origine.
    pub fn set_fts_storage(&mut self, storage: crate::fts_handle::FtsStorage) {
        self.fts_storage = storage;
    }

    /// Handle FTS d'une table, s'il est déjà ouvert.
    pub fn fts_handle(
        &self,
        table: &str,
    ) -> Option<Arc<lucivy_core::sharded_handle::ShardedHandle>> {
        self.fts_handles.get(table).cloned()
    }

    /// Ouvre (ou crée) l'index FTS d'une table, en le mémorisant.
    ///
    /// **Paresseux par conception** : ouvrir un index adossé au BlobStore
    /// télécharge l'intégralité de ses blobs. On ne paie donc ce coût qu'au
    /// premier usage effectif de la table, pas à `initialize()`.
    ///
    /// Même contrat que [`Self::ensure_sparse_handle`] : on tente l'ouverture,
    /// et on ne crée que si l'index n'existe pas encore.
    pub fn ensure_fts_handle(
        &mut self,
        table: &str,
        text_fields: &[String],
        filter_fields: &[(String, String)],
    ) -> Option<Arc<lucivy_core::sharded_handle::ShardedHandle>> {
        if let Some(h) = self.fts_handles.get(table) {
            return Some(h.clone());
        }
        if text_fields.is_empty() {
            return None;
        }
        let blob_store = self.blob_store.clone()?;

        use lucivy_core::sharded_handle::{
            BlobShardStorage, FsShardStorage, ShardStorage, ShardedHandle,
        };
        let index_name = crate::fts_handle::fts_index_name(table);

        let storage = || -> Option<Box<dyn ShardStorage>> {
            match &self.fts_storage {
                crate::fts_handle::FtsStorage::BlobBacked => Some(Box::new(
                    BlobShardStorage::new(
                        Arc::new(crate::fts_handle::DynBlobStore(blob_store.clone())),
                        index_name.clone(),
                        &self.cache_base,
                    ),
                )),
                crate::fts_handle::FtsStorage::LocalFs { base_path } => {
                    let dir = std::path::Path::new(base_path).join(&index_name);
                    match FsShardStorage::new(&dir.to_string_lossy()) {
                        Ok(s) => Some(Box::new(s)),
                        Err(e) => {
                            eprintln!("[rag3weaver] FsShardStorage {table}: {e}");
                            None
                        }
                    }
                }
            }
        };

        let handle = match ShardedHandle::open_with_storage(storage()?) {
            Ok(h) => h,
            Err(_) => {
                let config = match crate::fts_handle::build_schema_config(
                    text_fields,
                    filter_fields,
                    crate::fts_handle::DEFAULT_SHARDS,
                ) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("[rag3weaver] schéma FTS invalide pour {table}: {e}");
                        return None;
                    }
                };
                match ShardedHandle::create_with_storage(storage()?, &config) {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("[rag3weaver] création de l'index FTS {table} échouée: {e}");
                        return None;
                    }
                }
            }
        };

        let handle = Arc::new(handle);
        self.fts_handles.insert(table.to_string(), handle.clone());
        Some(handle)
    }

    /// Gracefully close all lucivy FTS indexes to release file locks.
    /// Must be called before dropping the Catalog when the DB will be reopened
    /// in the same process (e.g. tests, hot reload).
    pub fn shutdown(&mut self) -> Result<(), CatalogError> {
        // Collect table names for FTS and sparse.
        let mut fts_tables: Vec<String> = Vec::new();
        for name in self.entity_configs.keys() {
            fts_tables.push(name.clone());
        }
        for kb_name in self.kb_metadata.keys() {
            fts_tables.push(format!("{kb_name}_Index"));
        }
        let sparse_tables: Vec<String> = self.sparse_handles.keys().cloned().collect();

        self.event_bus.emit(CatalogEvent::ShutdownStarted {
            fts_tables: fts_tables.clone(),
            sparse_tables: sparse_tables.clone(),
        });

        // 1. Close lucivy FTS indexes (release writer locks).
        let mut fts_closed: usize = 0;
        let mut fts_failed: Vec<String> = Vec::new();
        for table in &fts_tables {
            let query = format!("CALL CLOSE_LUCIVY_INDEX('{table}')");
            match self.conn.execute(&query) {
                Ok(_) => fts_closed += 1,
                Err(e) => {
                    eprintln!("[rag3weaver] shutdown: failed to close lucivy index on {table}: {e}");
                    fts_failed.push(format!("{table}: {e}"));
                }
            }
        }

        // 2. Commit and drop sparse handles (release writer locks).
        let mut sparse_committed: usize = 0;
        let mut sparse_failed: Vec<String> = Vec::new();
        for (table, handle) in self.sparse_handles.drain() {
            match handle.commit_inner() {
                Ok(_) => sparse_committed += 1,
                Err(e) => {
                    eprintln!("[rag3weaver] shutdown: failed to commit sparse handle {table}: {e}");
                    sparse_failed.push(format!("{table}: {e}"));
                }
            }
        }

        self.event_bus.emit(CatalogEvent::ShutdownCompleted {
            fts_closed,
            fts_failed,
            sparse_committed,
            sparse_failed,
        });

        Ok(())
    }

    pub fn initialize(&mut self) -> Result<(), CatalogError> {
        // 0. Backend setup (CREATE EXTENSION, CREATE SCHEMA, etc.)
        for stmt in self.dialect.setup_statements() {
            self.conn.execute(&stmt)
                .map_err(|e| CatalogError::DbError(e.to_string()))?;
        }

        // 1. Validate schema
        let validation = validate_schema(&self.config);
        if !validation.valid {
            return Err(CatalogError::ValidationFailed(
                validation.errors.join("; "),
            ));
        }

        // 2. Generate DDL (using dialect for backend-specific statements)
        let schema = generate_full_schema_with_dialect(&self.config, self.dialect.as_ref())
            .map_err(|e| CatalogError::SchemaError(e.to_string()))?;

        // 3. Execute DDL statements (tables first)
        for ddl in &schema.ddl {
            self.conn
                .execute(ddl)
                .map_err(|e| CatalogError::DbError(e.to_string()))?;
        }

        // 4. Execute index statements
        for idx in &schema.indexes {
            self.conn
                .execute(idx)
                .map_err(|e| CatalogError::DbError(e.to_string()))?;
        }

        // 5. Build KB metadata from validation result + config
        for (kb_name, kb_validation) in &validation.knowledge_bases {
            let kb_config = self
                .config
                .knowledge_bases
                .get(kb_name)
                .cloned()
                .unwrap_or_default();

            let title = match &kb_validation.title {
                Some(t) => KBFieldRef {
                    entity: t.entity.clone(),
                    field: t.field.clone(),
                },
                None => continue,
            };

            let content: Vec<KBFieldRef> = kb_validation
                .content
                .iter()
                .map(|c| KBFieldRef {
                    entity: c.entity.clone(),
                    field: c.field.clone(),
                })
                .collect();

            self.kb_metadata.insert(
                kb_name.clone(),
                KBMetadata {
                    name: kb_name.clone(),
                    title,
                    content,
                    entities: kb_validation.entities.clone(),
                    signals: kb_config.signals,
                    keyword_weight: kb_config.keyword_weight,
                    title_boost: kb_config.title_boost,
                    content_boost: kb_config.content_boost,
                    chunking: kb_config.chunking,
                },
            );
        }

        // 6. Pre-warm chunker cache for ingestion nodes
        self.warm_chunker_cache();

        // 7. Initialize checkpoint store for crash-recovery (unless already set by tests)
        if self.checkpoint_store.is_none() {
            let cp_store: Arc<dyn CheckpointStore> =
                Arc::new(CypherCheckpointStore::new(self.conn.clone()));
            cp_store
                .initialize()
                .map_err(|e| CatalogError::DbError(e))?;
            self.checkpoint_store = Some(cp_store);
        }

        // 8. Initialize blob store for lucivy/sparse index persistence
        //    Must be before ensure_sparse_handle() which needs blob_store.
        if let Some(ref sync_conn) = self.sync_conn {
            let blob_ddl = self.dialect.create_blob_table();
            self.conn.execute(&blob_ddl).map_err(|e| CatalogError::DbError(e.to_string()))?;
            self.blob_store = Some(Arc::new(CypherBlobStore::from_sync_connection(sync_conn.clone())));
        } else if self.blob_store.is_none() {
            // Fallback: in-memory blob store for tests / in-memory DBs (no persistence needed)
            self.blob_store = Some(Arc::new(sparse_vector::blob_store::MemBlobStore::new()));
        }

        // 9. Create sparse vector handles for KBs that have sparse=true.
        if self.sparse_embedder.is_some() || self.dual_embedder.is_some() {
            let kb_sparse_tables: Vec<String> = self.config.knowledge_bases.iter()
                .filter(|(_, kbc)| kbc.signals.sparse())
                .map(|(kb_name, _)| format!("{kb_name}_Index_Chunk"))
                .collect();
            for table in kb_sparse_tables {
                self.ensure_sparse_handle(&table);
            }
        }

        // 10. Load persisted entity configs, relations, and KB configs from _catalog_meta
        self.load_entity_configs()?;
        self.load_relations()?;
        self.load_kb_configs()?;

        // 11. Initialize search backend (default: Rag3dbSearchBackend)
        if self.search_backend.is_none() {
            self.search_backend = Some(Arc::new(
                crate::rag3db_search_backend::Rag3dbSearchBackend::new(self.conn.clone()),
            ));
        }

        self.initialized = true;
        Ok(())
    }

    /// Get the search backend.
    pub fn search_backend(&self) -> Option<Arc<dyn crate::search_backend::SearchBackend>> {
        self.search_backend.clone()
    }

    // ── Entity Registration ──────────────────────────────────────────────

    /// Register an entity. Supports simple pipeline, KB participation, or both.
    ///
    /// For entities with simple pipeline fields (`is_content`/`is_title`):
    /// creates entity table, chunk table, FTS/vector/sparse indexes.
    ///
    /// For KB-only entities (`content_for`/`title_for`): creates only the
    /// entity table. Indexes are created by `register_kb()`.
    ///
    /// Order-independent: if a KB mentioned by this entity is already registered,
    /// it will be re-triggered to pick up the new fields.
    pub fn register_entity(
        &mut self,
        entity_name: &str,
        config: crate::config::EntityConfig,
    ) -> Result<(), CatalogError> {
        self.check_initialized()?;

        // Validate field definitions
        config.validate().map_err(|e| CatalogError::SchemaError(e))?;

        if self.kb_metadata.contains_key(entity_name) {
            return Err(CatalogError::SchemaError(
                format!("Name '{}' conflicts with an existing knowledge base", entity_name),
            ));
        }

        if !config.has_simple_pipeline() && !config.has_kb_participation() {
            return Err(CatalogError::SchemaError(
                format!("Entity '{}' has no content fields — need at least is_content=true (simple pipeline) or content_for/title_for (KB participation)", entity_name),
            ));
        }

        let entity_def = Self::entity_config_to_entity_def(&config);

        if let Some(old_config) = self.entity_configs.get(entity_name) {
            // ── Idempotent path: entity already registered ──
            self.migrate_entity(entity_name, old_config.clone(), &config)?;
        } else {
            // ── Fresh registration: create tables + indexes ──
            self.create_entity_tables(entity_name, &config, &entity_def)?;
        }

        // Create sparse handle if needed (simple pipeline with sparse signal)
        if config.has_simple_pipeline() && config.signals.sparse() {
            let chunk_table = format!("{entity_name}_Chunk");
            self.ensure_sparse_handle(&chunk_table);
        }

        // Persist + update in-memory
        self.persist_entity_config(entity_name, &config)?;
        self.config.entities.insert(entity_name.to_string(), entity_def);
        self.entity_configs.insert(entity_name.to_string(), config.clone());

        // Re-trigger KBs that this entity mentions (existing in kb_metadata OR
        // pre-registered in knowledge_bases but not yet materialized)
        let mut kb_names_to_retrigger = HashSet::new();
        for f in config.fields.values() {
            if let Some(ref kb) = f.title_for {
                if kb != "self" && (self.kb_metadata.contains_key(kb) || self.config.knowledge_bases.contains_key(kb)) {
                    kb_names_to_retrigger.insert(kb.clone());
                }
            }
            if let Some(ref kbs) = f.content_for {
                for kb in kbs {
                    if kb != "self" && (self.kb_metadata.contains_key(kb) || self.config.knowledge_bases.contains_key(kb)) {
                        kb_names_to_retrigger.insert(kb.clone());
                    }
                }
            }
        }
        for kb_name in kb_names_to_retrigger {
            let kb_config = self.config.knowledge_bases.get(&kb_name).cloned().unwrap_or_default();
            self.register_kb(&kb_name, kb_config)?;
        }

        Ok(())
    }

    /// Convert an EntityConfig (simple fields) to an EntityDef (catalog-level definition).
    fn entity_config_to_entity_def(config: &crate::config::EntityConfig) -> crate::config::EntityDef {
        let mut entity_fields = HashMap::new();
        for (name, sfd) in &config.fields {
            entity_fields.insert(name.clone(), crate::config::FieldDef {
                field_type: sfd.field_type.clone(),
                title_for: sfd.title_for.clone(),
                content_for: sfd.content_for.clone(),
                boost: None,
                default_value: None,
            });
        }
        crate::config::EntityDef {
            fields: entity_fields,
            hashsafe: None,
        }
    }

    /// Create all tables and indexes for a new entity.
    ///
    /// Always creates the entity node table. Only creates chunk table, FTS,
    /// vector and sparse indexes if the entity has simple pipeline content
    /// fields (is_content=true). KB-only entities get their indexes through
    /// `register_kb()` instead.
    fn create_entity_tables(
        &self,
        entity_name: &str,
        config: &crate::config::EntityConfig,
        entity_def: &crate::config::EntityDef,
    ) -> Result<(), CatalogError> {
        // 1. Entity node table (always)
        let entity_ddl = crate::schema::generate_node_table_ddl(entity_name, entity_def)
            .map_err(|e| CatalogError::SchemaError(e.to_string()))?;
        self.conn.execute(&entity_ddl)
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        // Skip chunk/FTS/vector/sparse for KB-only entities (no simple pipeline)
        if !config.has_simple_pipeline() {
            return Ok(());
        }

        // 2. Chunk table
        let chunk_ddl = crate::schema::generate_simple_chunk_table_ddl(
            entity_name, config, self.config.embedding_dim,
        ).map_err(|e| CatalogError::SchemaError(e.to_string()))?;
        self.conn.execute(&chunk_ddl)
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        // 3. CHUNKED_FROM relation
        let rel_ddl = crate::schema::generate_simple_chunk_rel_ddl(entity_name)
            .map_err(|e| CatalogError::SchemaError(e.to_string()))?;
        self.conn.execute(&rel_ddl)
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        // 4. FTS index on entity content fields
        let fts_fields: Vec<&str> = config.content_fields();
        let fts_ddl = crate::schema::generate_fts_index_ddl(entity_name, &fts_fields, &[]);
        let _ = self.conn.execute(&fts_ddl); // ignore if already exists

        // 5. Vector index on chunk table
        if config.signals.vector() {
            let chunk_table = format!("{entity_name}_Chunk");
            let idx_name = format!("{entity_name}_Chunk_vec");
            let vec_ddl = crate::schema::generate_vector_index_ddl(
                &chunk_table, "embedding", &idx_name,
            );
            let _ = self.conn.execute(&vec_ddl);
        }

        // 6. Sparse vector index — handled by ensure_sparse_handle() in register_entity()

        Ok(())
    }

    /// Migrate an existing entity: add new fields, detect removed/changed fields.
    fn migrate_entity(
        &self,
        entity_name: &str,
        old_config: crate::config::EntityConfig,
        new_config: &crate::config::EntityConfig,
    ) -> Result<(), CatalogError> {
        let old_fields = &old_config.fields;
        let new_fields = &new_config.fields;

        // Detect removed fields → error
        for name in old_fields.keys() {
            if !new_fields.contains_key(name) {
                return Err(CatalogError::SchemaError(
                    format!("Entity '{entity_name}': cannot remove field '{name}' (destructive migration not supported)")
                ));
            }
        }

        // Detect type changes → error
        for (name, old_f) in old_fields {
            if let Some(new_f) = new_fields.get(name) {
                if old_f.field_type != new_f.field_type {
                    return Err(CatalogError::SchemaError(
                        format!("Entity '{entity_name}': cannot change type of field '{name}' from {:?} to {:?}", old_f.field_type, new_f.field_type)
                    ));
                }
            }
        }

        // Add new fields via ALTER TABLE
        let mut content_changed = false;
        for (name, new_f) in new_fields {
            if !old_fields.contains_key(name) {
                use crate::dialect::{ColumnDef, ColumnType};
                let col = ColumnDef {
                    name: name.to_string(),
                    col_type: ColumnType::from_field_type(&new_f.field_type),
                };
                let alter_ddl = self.dialect.alter_add_column(entity_name, &col);
                self.conn.execute(&alter_ddl)
                    .map_err(|e| CatalogError::DbError(e.to_string()))?;

                if new_f.is_content || new_f.is_title || new_f.content_for.is_some() || new_f.title_for.is_some() {
                    content_changed = true;
                }
            }
        }

        // Check if content/title annotations changed on existing fields
        if !content_changed {
            for (name, new_f) in new_fields {
                if let Some(old_f) = old_fields.get(name) {
                    if old_f.is_content != new_f.is_content
                        || old_f.is_title != new_f.is_title
                        || old_f.content_for != new_f.content_for
                        || old_f.title_for != new_f.title_for
                    {
                        content_changed = true;
                        break;
                    }
                }
            }
        }

        // Rebuild FTS if content fields changed (only for simple pipeline entities)
        if content_changed {
            if new_config.has_simple_pipeline() {
                // Drop + recreate FTS index on entity table
                // TODO: migrate to Rust LucivyHandle when FTS migration is done (doc 02).
                // For now, FTS rebuild is rag3db-only (lucivy extension C++).
                // On PostgreSQL, FTS will be managed by lucivy handles directly.
                if self.dialect.name() == "rag3db" {
                    let drop_fts = format!("CALL DROP_LUCIVY_INDEX('{entity_name}')");
                    let _ = self.conn.execute(&drop_fts);

                    let fts_fields: Vec<&str> = new_config.content_fields();
                    let fts_ddl = crate::schema::generate_fts_index_ddl(entity_name, &fts_fields, &[]);
                    let _ = self.conn.execute(&fts_ddl);
                }
            }

            // Flag needs_reindex (for both simple and KB pipelines)
            self.persist_meta_key(
                &format!("needs_reindex:{entity_name}"),
                "true",
            )?;
            eprintln!("[rag3weaver] warning: Entity '{entity_name}' needs reindex after schema change — run catalog.reindex('{entity_name}')");
        }

        // Create missing indexes (new signals) — only if simple pipeline
        if new_config.has_simple_pipeline() && new_config.signals.vector() && !old_config.signals.vector() {
            let chunk_table = format!("{entity_name}_Chunk");
            let idx_name = format!("{entity_name}_Chunk_vec");
            let vec_ddl = self.dialect.create_vector_index(&chunk_table, "embedding", &idx_name);
            let _ = self.conn.execute(&vec_ddl);
        }
        // Sparse handle creation is handled by register_entity() after migrate_entity().

        Ok(())
    }

    /// Check if a name is a registered entity (simple or KB-only).
    pub fn is_registered_entity(&self, name: &str) -> bool {
        self.entity_configs.contains_key(name)
    }

    /// Check if a name is a registered simple entity (has simple pipeline with chunk table).
    pub fn is_simple_entity(&self, name: &str) -> bool {
        self.entity_configs.get(name).map_or(false, |ec| ec.has_simple_pipeline())
    }

    /// Get a simple entity config, if registered.
    pub fn entity_config(&self, name: &str) -> Option<&crate::config::EntityConfig> {
        self.entity_configs.get(name)
    }

    // ── Relation Registration ───────────────────────────────────────────

    /// Register a relation between two entities. Idempotent (IF NOT EXISTS).
    ///
    /// Both `from` and `to` must be known entities (registered via `register_entity()`
    /// or declared in `CatalogConfig`).
    pub fn register_relation(
        &mut self,
        rel_name: &str,
        from: &str,
        to: &str,
    ) -> Result<(), CatalogError> {
        self.check_initialized()?;

        // Validate identifiers
        crate::schema::validate_identifier(rel_name, "relation")
            .map_err(|e| CatalogError::SchemaError(e.to_string()))?;

        // Check that both endpoints exist
        if !self.config.entities.contains_key(from) {
            return Err(CatalogError::UnknownEntity(from.to_string()));
        }
        if !self.config.entities.contains_key(to) {
            return Err(CatalogError::UnknownEntity(to.to_string()));
        }

        // If already registered, check consistency
        if let Some(existing) = self.config.relations.get(rel_name) {
            if existing.from != from || existing.to != to {
                return Err(CatalogError::SchemaError(format!(
                    "Relation '{rel_name}' already registered as ({} → {}), cannot re-register as ({from} → {to})",
                    existing.from, existing.to,
                )));
            }
            // Same definition → no-op, but persist anyway for idempotence
        } else {
            // Create the rel table
            let ddl = self.dialect.create_rel_table(&rel_name, from, to, &[]);
            self.conn.execute(&ddl)
                .map_err(|e| CatalogError::DbError(e.to_string()))?;
        }

        // Persist + update in-memory
        let rel_def = RelationDef {
            from: from.to_string(),
            to: to.to_string(),
            properties: None,
        };
        self.persist_relation(rel_name, &rel_def)?;
        self.config.relations.insert(rel_name.to_string(), rel_def);

        Ok(())
    }

    // ── KB Registration ─────────────────────────────────────────────────

    /// Register a Knowledge Base. Idempotent with additive migration.
    ///
    /// Scans registered entities for fields with `title_for`/`content_for` pointing
    /// to this KB name. Creates `{KB}_Index`, `{KB}_Index_Chunk`, relation tables,
    /// FTS index, and vector/sparse indexes.
    ///
    /// Order-independent with `register_entity()`: if entities are registered
    /// after the KB, `register_entity()` will re-trigger this method. If
    /// re-called with new content refs, rebuilds the FTS index on `{KB}_Index`.
    pub fn register_kb(
        &mut self,
        kb_name: &str,
        kb_config: crate::config::KBConfig,
    ) -> Result<(), CatalogError> {
        self.check_initialized()?;

        crate::schema::validate_identifier(kb_name, "knowledge_base")
            .map_err(|e| CatalogError::SchemaError(e.to_string()))?;

        // Find the title entity (entity with a field that has title_for = kb_name)
        let kb_title_entities = crate::schema::resolve_kb_title_entities(&self.config);
        let kb_info = kb_title_entities.get(kb_name);

        // Collect all entities contributing to this KB
        let mut kb_entities = HashSet::new();
        let mut content_refs = Vec::new();
        for (entity_name, entity_def) in &self.config.entities {
            let entity_kbs = crate::schema::resolve_entity_kbs(entity_def);
            if let Some(mapping) = entity_kbs.get(kb_name) {
                kb_entities.insert(entity_name.clone());
                for field in &mapping.content_fields {
                    content_refs.push(KBFieldRef {
                        entity: entity_name.clone(),
                        field: field.clone(),
                    });
                }
            }
        }

        if let Some(info) = kb_info {
            // Title entity exists — can create/update tables
            if let Some(old_meta) = self.kb_metadata.get(kb_name) {
                // ── Idempotent: KB already exists — check if content refs changed ──
                let old_content: HashSet<_> = old_meta.content.iter()
                    .map(|r| (r.entity.as_str(), r.field.as_str()))
                    .collect();
                let new_content: HashSet<_> = content_refs.iter()
                    .map(|r| (r.entity.as_str(), r.field.as_str()))
                    .collect();
                if old_content != new_content {
                    // Rebuild FTS on {KB}_Index to include new content fields
                    let index_table = format!("{kb_name}_Index");
                    let drop_fts = format!("CALL DROP_LUCIVY_INDEX('{index_table}')");
                    let _ = self.conn.execute(&drop_fts);
                    let fts_ddl = crate::schema::generate_fts_index_ddl(
                        &index_table, &["_title", "_content"], &["_source_entity"],
                    );
                    let _ = self.conn.execute(&fts_ddl);
                }
            } else {
                // ── Fresh KB: create all tables + indexes ──
                self.create_kb_tables(kb_name, &kb_config, info, &kb_entities)?;
            }

            // Create sparse handle if needed
            if kb_config.signals.sparse() {
                let chunk_table = format!("{kb_name}_Index_Chunk");
                self.ensure_sparse_handle(&chunk_table);
            }

            // Build + store KBMetadata
            let title_ref = KBFieldRef {
                entity: info.title_entity.clone(),
                field: info.title_field.clone(),
            };
            let kb_meta = KBMetadata {
                name: kb_name.to_string(),
                title: title_ref,
                content: content_refs,
                entities: kb_entities,
                signals: kb_config.signals,
                keyword_weight: kb_config.keyword_weight,
                title_boost: kb_config.title_boost,
                content_boost: kb_config.content_boost,
                chunking: kb_config.chunking.clone(),
            };
            self.kb_metadata.insert(kb_name.to_string(), kb_meta);
        }
        // else: no entities yet — just persist config. When register_entity()
        // is called later with title_for/content_for pointing to this KB,
        // it will re-trigger register_kb() and create the tables then.

        // Persist + update config
        self.persist_kb_config(kb_name, &kb_config)?;
        self.config.knowledge_bases.insert(kb_name.to_string(), kb_config);

        // Warm chunker cache for the new KB
        self.warm_chunker_cache();

        Ok(())
    }

    /// Create all tables and indexes for a new Knowledge Base.
    fn create_kb_tables(
        &self,
        kb_name: &str,
        kb_config: &crate::config::KBConfig,
        kb_info: &crate::schema::KBSchemaInfo,
        kb_entities: &HashSet<String>,
    ) -> Result<(), CatalogError> {
        let embedding_dim = self.config.embedding_dim;

        // 1. {KB}_Index table
        let idx_ddl = crate::schema::generate_index_table_ddl(kb_name, kb_config, embedding_dim)
            .map_err(|e| CatalogError::SchemaError(e.to_string()))?;
        self.conn.execute(&idx_ddl)
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        // 2. {KB}_Index_Chunk table
        let chunk_ddl = crate::schema::generate_index_chunk_table_ddl(kb_name, kb_config, embedding_dim)
            .map_err(|e| CatalogError::SchemaError(e.to_string()))?;
        self.conn.execute(&chunk_ddl)
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        // 3. {KB}_Index_HAS_CHUNK rel
        let has_chunk_ddl = crate::schema::generate_index_chunk_rel_ddl(kb_name)
            .map_err(|e| CatalogError::SchemaError(e.to_string()))?;
        self.conn.execute(&has_chunk_ddl)
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        // 4. {TitleEntity}_IN_{KB} rel
        let in_ddl = crate::schema::generate_index_rel_ddl(&kb_info.title_entity, kb_name)
            .map_err(|e| CatalogError::SchemaError(e.to_string()))?;
        self.conn.execute(&in_ddl)
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        // 5. {Entity}_SOURCED_{KB} rels (one per contributing entity)
        for entity_name in kb_entities {
            let source_ddl = crate::schema::generate_source_rel_ddl(entity_name, kb_name)
                .map_err(|e| CatalogError::SchemaError(e.to_string()))?;
            self.conn.execute(&source_ddl)
                .map_err(|e| CatalogError::DbError(e.to_string()))?;
        }

        // 6. FTS index on {KB}_Index
        let index_table = format!("{kb_name}_Index");
        let fts_ddl = crate::schema::generate_fts_index_ddl(
            &index_table, &["_title", "_content"], &["_source_entity"],
        );
        let _ = self.conn.execute(&fts_ddl);

        // 7. Vector index on {KB}_Index_Chunk
        if kb_config.signals.vector() {
            let chunk_table = format!("{kb_name}_Index_Chunk");
            let emb_col = format!("{kb_name}_embedding");
            let idx_name = format!("{kb_name}_Index_Chunk_vec");
            let vec_ddl = crate::schema::generate_vector_index_ddl(&chunk_table, &emb_col, &idx_name);
            let _ = self.conn.execute(&vec_ddl);
        }

        // 8. Sparse handle — created by register_kb() after create_kb_tables().

        Ok(())
    }

    // ── Reindex ─────────────────────────────────────────────────────────

    /// Re-process all records of an entity after a schema change.
    ///
    /// Queries all existing records, enqueues them as updates, and drains.
    /// UpdateRecordNode handles the rest: rechunk for simple entities,
    /// re-aggregate for KB entities.
    ///
    /// Clears the `needs_reindex:{entity}` flag on success.
    pub fn reindex(&mut self, entity_name: &str) -> Result<ReindexStats, CatalogError> {
        self.check_initialized()?;
        let entity_def = self.check_entity(entity_name)?.clone();

        // Build field list for the query
        let mut field_names: Vec<&String> = entity_def.fields.keys().collect();
        field_names.sort();

        let mut all_fields = vec!["_uuid"];
        all_fields.extend(field_names.iter().map(|s| s.as_str()));
        let cypher = self.dialect.select_all(entity_name, &all_fields, None);

        let result = self.conn.execute(&cypher)
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        let mut records_enqueued = 0usize;

        for row in &result.rows {
            // First column is _uuid
            let uuid = match row.get(0) {
                Some(CypherValue::String(s)) => s.clone(),
                _ => continue,
            };

            // Build data map from remaining columns
            let mut data = BTreeMap::new();
            for (i, field_name) in field_names.iter().enumerate() {
                if let Some(val) = row.get(i + 1) {
                    data.insert((*field_name).clone(), val.clone());
                }
            }

            // Enqueue as update (same as catalog.update())
            let new_content = self.build_content_text(entity_name, &data);
            let new_content_hash = content_hash(&new_content);
            self.pending.updates.push(crate::records::UpdateRecord {
                entity_name: entity_name.to_string(),
                uuid,
                data,
                new_content_hash,
            });
            records_enqueued += 1;
        }

        // Drain if there's work to do
        if records_enqueued > 0 {
            self.drain();
        }

        // Clear the needs_reindex flag
        self.persist_meta_key(
            &format!("needs_reindex:{entity_name}"),
            "false",
        )?;

        Ok(ReindexStats {
            entity: entity_name.to_string(),
            records_processed: records_enqueued,
        })
    }

    // ── Persistence (_catalog_meta) ─────────────────────────────────────

    /// Persist a key-value pair to `_catalog_meta`.
    fn persist_meta_key(&self, key: &str, value: &str) -> Result<(), CatalogError> {
        let stmt = self.dialect.upsert_meta("key", "value");
        self.conn.execute_with_params(
            &stmt,
            &[
                QueryParam::new("key", CypherValue::String(key.to_string())),
                QueryParam::new("value", CypherValue::String(value.to_string())),
            ],
        ).map_err(|e| CatalogError::DbError(e.to_string()))?;
        Ok(())
    }

    /// Persist an entity config to `_catalog_meta`.
    fn persist_entity_config(
        &self,
        entity_name: &str,
        config: &crate::config::EntityConfig,
    ) -> Result<(), CatalogError> {
        let json = serde_json::to_string(config)
            .map_err(|e| CatalogError::SchemaError(format!("serialize entity config: {e}")))?;
        self.persist_meta_key(&format!("entity_config:{entity_name}"), &json)
    }

    /// Load all persisted entity configs from `_catalog_meta`.
    /// Called at the end of `initialize()` to restore simple entities.
    fn load_entity_configs(&mut self) -> Result<(), CatalogError> {
        let stmt = self.dialect.load_meta_by_prefix("prefix");
        let result = self.conn.execute_with_params(
            &stmt,
            &[QueryParam::new("prefix", CypherValue::String("entity_config:".into()))],
        ).map_err(|e| CatalogError::DbError(e.to_string()))?;

        for row in &result.rows {
            let key = match row.get(0) {
                Some(CypherValue::String(s)) => s.clone(),
                _ => continue,
            };
            let value = match row.get(1) {
                Some(CypherValue::String(s)) => s.clone(),
                _ => continue,
            };
            let entity_name = key.strip_prefix("entity_config:").unwrap_or(&key);
            let config: crate::config::EntityConfig = serde_json::from_str(&value)
                .map_err(|e| CatalogError::SchemaError(
                    format!("deserialize entity config for '{entity_name}': {e}")
                ))?;

            // Restore EntityDef in config.entities
            let entity_def = Self::entity_config_to_entity_def(&config);
            self.config.entities.insert(entity_name.to_string(), entity_def);
            self.entity_configs.insert(entity_name.to_string(), config);
        }

        Ok(())
    }

    /// Persist a relation definition to `_catalog_meta`.
    fn persist_relation(
        &self,
        rel_name: &str,
        rel_def: &RelationDef,
    ) -> Result<(), CatalogError> {
        let json = serde_json::to_string(rel_def)
            .map_err(|e| CatalogError::SchemaError(format!("serialize relation: {e}")))?;
        self.persist_meta_key(&format!("relation:{rel_name}"), &json)
    }

    /// Persist a KB config to `_catalog_meta`.
    fn persist_kb_config(
        &self,
        kb_name: &str,
        kb_config: &crate::config::KBConfig,
    ) -> Result<(), CatalogError> {
        let json = serde_json::to_string(kb_config)
            .map_err(|e| CatalogError::SchemaError(format!("serialize kb config: {e}")))?;
        self.persist_meta_key(&format!("kb_config:{kb_name}"), &json)
    }

    /// Load all persisted KB configs from `_catalog_meta` and rebuild KBMetadata.
    /// Called at the end of `initialize()` to restore dynamically registered KBs.
    fn load_kb_configs(&mut self) -> Result<(), CatalogError> {
        let stmt = self.dialect.load_meta_by_prefix("prefix");
        let result = self.conn.execute_with_params(
            &stmt,
            &[QueryParam::new("prefix", CypherValue::String("kb_config:".into()))],
        ).map_err(|e| CatalogError::DbError(e.to_string()))?;

        for row in &result.rows {
            let key = match row.get(0) {
                Some(CypherValue::String(s)) => s.clone(),
                _ => continue,
            };
            let value = match row.get(1) {
                Some(CypherValue::String(s)) => s.clone(),
                _ => continue,
            };
            let kb_name = key.strip_prefix("kb_config:").unwrap_or(&key);

            // Skip if already loaded by initialize() (config-driven KBs)
            if self.kb_metadata.contains_key(kb_name) {
                continue;
            }

            let kb_config: crate::config::KBConfig = serde_json::from_str(&value)
                .map_err(|e| CatalogError::SchemaError(
                    format!("deserialize kb config for '{kb_name}': {e}")
                ))?;

            // Rebuild KBMetadata from entity fields
            let kb_title_entities = crate::schema::resolve_kb_title_entities(&self.config);
            let kb_info = match kb_title_entities.get(kb_name) {
                Some(info) => info,
                None => continue, // No title entity found, skip
            };

            let mut kb_entities = HashSet::new();
            let mut content_refs = Vec::new();
            for (entity_name, entity_def) in &self.config.entities {
                let entity_kbs = crate::schema::resolve_entity_kbs(entity_def);
                if let Some(mapping) = entity_kbs.get(kb_name) {
                    kb_entities.insert(entity_name.clone());
                    for field in &mapping.content_fields {
                        content_refs.push(KBFieldRef {
                            entity: entity_name.clone(),
                            field: field.clone(),
                        });
                    }
                }
            }

            let title_ref = KBFieldRef {
                entity: kb_info.title_entity.clone(),
                field: kb_info.title_field.clone(),
            };
            self.kb_metadata.insert(kb_name.to_string(), KBMetadata {
                name: kb_name.to_string(),
                title: title_ref,
                content: content_refs,
                entities: kb_entities,
                signals: kb_config.signals,
                keyword_weight: kb_config.keyword_weight,
                title_boost: kb_config.title_boost,
                content_boost: kb_config.content_boost,
                chunking: kb_config.chunking.clone(),
            });
            self.config.knowledge_bases.insert(kb_name.to_string(), kb_config);
        }

        Ok(())
    }

    /// Load all persisted relations from `_catalog_meta`.
    /// Called at the end of `initialize()` to restore dynamically registered relations.
    fn load_relations(&mut self) -> Result<(), CatalogError> {
        let stmt = self.dialect.load_meta_by_prefix("prefix");
        let result = self.conn.execute_with_params(
            &stmt,
            &[QueryParam::new("prefix", CypherValue::String("relation:".into()))],
        ).map_err(|e| CatalogError::DbError(e.to_string()))?;

        for row in &result.rows {
            let key = match row.get(0) {
                Some(CypherValue::String(s)) => s.clone(),
                _ => continue,
            };
            let value = match row.get(1) {
                Some(CypherValue::String(s)) => s.clone(),
                _ => continue,
            };
            let rel_name = key.strip_prefix("relation:").unwrap_or(&key);
            let rel_def: RelationDef = serde_json::from_str(&value)
                .map_err(|e| CatalogError::SchemaError(
                    format!("deserialize relation '{rel_name}': {e}")
                ))?;
            self.config.relations.insert(rel_name.to_string(), rel_def);
        }

        Ok(())
    }

    // ── SearchTarget resolution ─────────────────────────────────────────

    /// Resolve a name (KB or simple entity) into a [`SearchTarget`](search::SearchTarget).
    ///
    /// Checks `kb_metadata` first (for KBs), then `entity_configs` (for simple entities).
    pub fn resolve_search_target(&self, name: &str) -> Result<search::SearchTarget, CatalogError> {
        // Try KB first
        if let Some(kb) = self.kb_metadata.get(name) {
            let kb_config = self
                .config
                .knowledge_bases
                .get(name)
                .cloned()
                .unwrap_or_default();
            let entity = format!("{name}_Index");
            let chunk_entity = format!("{name}_Index_Chunk");
            let title_entity = kb.title.entity.clone();
            let in_rel = format!("{title_entity}_IN_{name}");
            return Ok(search::SearchTarget {
                name: name.to_string(),
                parent_table: entity.clone(),
                chunk_table: chunk_entity,
                chunk_rel: format!("{entity}_HAS_CHUNK"),
                chunk_rel_fwd: true,
                bm25_fields: vec!["_title".to_string(), "_content".to_string()],
                enrich_fields: vec![
                    "_title".to_string(),
                    "_content".to_string(),
                    "_source_entity".to_string(),
                    "_source_uuid".to_string(),
                    "_content_hash".to_string(),
                ],
                default_signals: kb_config.signals,
                default_fusion: kb_config.fusion_config(),
                has_source_refs: true,
                filter_indirection: Some((title_entity, in_rel)),
            });
        }

        // Try simple entity (must have simple pipeline — KB-only entities are not searchable directly)
        if let Some(ec) = self.entity_configs.get(name) {
            if !ec.has_simple_pipeline() {
                // Find which KBs this entity participates in for a helpful error
                let kb_names: Vec<&String> = self.kb_metadata.iter()
                    .filter(|(_, meta)| meta.entities.contains(name))
                    .map(|(kb_name, _)| kb_name)
                    .collect();
                let suggestion = if kb_names.is_empty() {
                    String::new()
                } else {
                    format!(" — search on KB {} instead", kb_names.iter().map(|n| format!("'{n}'")).collect::<Vec<_>>().join(", "))
                };
                return Err(CatalogError::SchemaError(
                    format!("Entity '{name}' has no simple pipeline (KB-only){suggestion}")
                ));
            }
            let chunk_table = format!("{name}_Chunk");
            let mut enrich_fields: Vec<String> = ec.content_fields().into_iter().map(|s| s.to_string()).collect();
            if let Some(title) = ec.title_field() {
                let title_owned = title.to_string();
                if !enrich_fields.contains(&title_owned) {
                    enrich_fields.push(title_owned);
                }
            }
            enrich_fields.push("_content_hash".to_string());
            let bm25_fields: Vec<String> = ec.content_fields().into_iter().map(|s| s.to_string()).collect();
            return Ok(search::SearchTarget {
                name: name.to_string(),
                parent_table: name.to_string(),
                chunk_table,
                chunk_rel: format!("{name}_CHUNKED_FROM"),
                chunk_rel_fwd: false,
                bm25_fields,
                enrich_fields,
                default_signals: ec.signals,
                default_fusion: search::FusionConfig::default(),
                has_source_refs: false,
                filter_indirection: None,
            });
        }

        Err(CatalogError::UnknownKB(name.to_string()))
    }

    // ── Simple Entity Ingestion ────────────────────────────────────────

    /// Ingest records into a simple entity (registered via `register_entity`).
    ///
    /// Builds and executes a dataflow graph:
    /// ```text
    /// InsertRecordNode("insert")
    ///     →|inserted:entities| ChunkRecordNode("chunk")
    ///         →|chunks| InsertRecordNode("chunk_insert")
    ///             →|inserted:entities| EmbedNode("embed")
    ///         →|chunk_links| LinkRecordNode("chunk_link")
    ///             ←|trigger| chunk_insert.done
    ///     →|done:trigger| FlushNode("flush_fts", tables=["{Entity}"])
    /// ```
    pub fn ingest_entities(
        &mut self,
        entity_name: &str,
        records: Vec<BTreeMap<String, CypherValue>>,
    ) -> Result<FlushResult, CatalogError> {
        self.check_initialized()?;

        let entity_config = self.entity_configs.get(entity_name)
            .ok_or_else(|| CatalogError::UnknownEntity(entity_name.to_string()))?
            .clone();

        if records.is_empty() {
            return Ok(FlushResult::default());
        }

        // Ensure chunker is cached for this entity's config
        let chunker_key = ChunkerConfig {
            max_size: entity_config.chunking.max_size,
            overlap: entity_config.chunking.overlap,
            strategy: entity_config.chunking.strategy.clone(),
        };
        self.chunker_cache
            .entry(chunker_key)
            .or_insert_with(|| {
                let key = ChunkerConfig {
                    max_size: entity_config.chunking.max_size,
                    overlap: entity_config.chunking.overlap,
                    strategy: entity_config.chunking.strategy.clone(),
                };
                Chunker::new(key)
            });

        // Build entity records with UUIDs and content hashes
        let entity_def = self.config.entities.get(entity_name)
            .ok_or_else(|| CatalogError::UnknownEntity(entity_name.to_string()))?
            .clone();

        let mut entity_records: Vec<EntityRecord> = Vec::with_capacity(records.len());
        for mut data in records {
            // Generate deterministic UUID from hashsafe fields or all content fields
            let uuid = if let Some(ref hashsafe_fields) = entity_def.hashsafe {
                let field_values: Vec<&str> = hashsafe_fields
                    .iter()
                    .map(|f| data.get(f).and_then(|v| v.as_str()).unwrap_or(""))
                    .collect();
                hashsafe_uuid(entity_name, &field_values)
            } else {
                // Use all data fields as hashsafe input for deterministic UUIDs
                let mut field_values: Vec<String> = data.iter()
                    .map(|(k, v)| format!("{}={}", k, v.as_str().unwrap_or("")))
                    .collect();
                field_values.sort();
                let refs: Vec<&str> = field_values.iter().map(|s| s.as_str()).collect();
                hashsafe_uuid(entity_name, &refs)
            };
            data.insert("_uuid".into(), CypherValue::String(uuid.clone()));

            // Content hash from content fields
            let content_fields = entity_config.content_fields();
            let content_text: String = content_fields.iter()
                .filter_map(|f| data.get(*f).and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join("\n\n");
            data.insert("_content_hash".into(), CypherValue::String(content_hash(&content_text)));

            let (entity_ref, resolver) = EntityRef::new(entity_name);
            entity_records.push(EntityRecord {
                entity_name: entity_name.to_string(),
                data,
                entity_ref,
                resolver: Some(resolver),
            });
        }

        let record_count = entity_records.len();

        // Keep data copies for KB pipeline triggering (if needed)
        let kb_data: Vec<BTreeMap<String, CypherValue>> = if entity_config.has_kb_participation() {
            entity_records.iter().map(|r| r.data.clone()).collect()
        } else {
            Vec::new()
        };

        // Build dataflow graph
        let mut graph = DataflowGraph::new();

        // 1. Insert entities
        graph.add_node(Box::new(InsertRecordNode::new("insert"))).unwrap();
        graph.set_initial_input("insert", "entities",
            PortValue::new(BatchPayload::new(PortType::Entities, entity_records)));

        // 2. Chunk entities (uses entity_configs service)
        graph.add_node(Box::new(ChunkRecordNode::new("chunk"))).unwrap();
        graph.connect("insert", "inserted", "chunk", "entities").unwrap();

        // 3. Insert chunks
        graph.add_node(Box::new(InsertRecordNode::new("chunk_insert"))).unwrap();
        graph.connect("chunk", "chunks", "chunk_insert", "entities").unwrap();

        // 4. Link chunks → parent (CHUNKED_FROM)
        graph.add_node(Box::new(LinkRecordNode::new("chunk_link"))).unwrap();
        graph.connect("chunk", "chunk_links", "chunk_link", "relations").unwrap();
        graph.connect("chunk_insert", "done", "chunk_link", "trigger").unwrap();

        // 5. Embed chunks
        let signals = entity_config.signals;
        graph.add_node(Box::new(EmbedNode::new("embed", signals, 32))).unwrap();
        graph.connect("chunk_insert", "inserted", "embed", "entities").unwrap();
        graph.connect("chunk_link", "done", "embed", "trigger").unwrap();

        // 6. Flush FTS on entity table
        graph.add_node(Box::new(FlushNode::new("flush_fts", vec![entity_name.to_string()]))).unwrap();
        graph.connect("insert", "done", "flush_fts", "trigger").unwrap();

        // Build services
        let mut services = ServiceRegistry::new();
        services.register("conn", self.conn.clone());
        services.register("dialect", self.dialect.clone());
        services.register("node_id_cache", self.node_id_cache.clone());
        services.register("embedder", self.embedder.clone());
        services.register("embedding_dim", self.config.embedding_dim);
        services.register("config", self.config.clone());
        services.register("entity_configs", self.entity_configs.clone());
        services.register("chunker_cache", Arc::new(std::mem::take(&mut self.chunker_cache)));
        services.register("has_sparse",
            self.sparse_embedder.is_some() || self.dual_embedder.is_some());
        services.register("has_dual", self.dual_embedder.is_some());
        services.register("sparse_handles", self.sparse_handles.clone());
        services.register("fts_handles", self.fts_handles.clone());

        if let Some(ref sparse_emb) = self.sparse_embedder {
            services.register("sparse_embedder", sparse_emb.clone());
        }
        if let Some(ref dual_emb) = self.dual_embedder {
            services.register("dual_embedder", dual_emb.clone());
        }

        // Execute
        let node_count = graph.nodes.len();
        let runtime = DataflowRuntime::with_services(node_count + 20, services);

        let graph_def = graph.to_definition();
        let execution_id = format!(
            "ingest-{}-{}",
            &graph_def.hash()[..12],
            crate::dataflow::checkpoint::timestamp_ms(),
        );

        let result = if let Some(ref store) = self.checkpoint_store {
            runtime
                .execute_with_checkpoint(&mut graph, store.as_ref(), &execution_id)
        } else {
            runtime.execute(&mut graph)
        };

        match result {
            Ok(_output) => {
                // If this entity participates in KBs, trigger the KB pipeline.
                // The simple pipeline only inserts entity records + handles
                // chunking/embedding for the simple pipeline. We need drain()
                // (via UpdateRecordNode) to detect KB participation and route
                // records through the KB aggregate pipeline.
                if entity_config.has_kb_participation() {
                    for data in &kb_data {
                        let uuid = data.get("_uuid")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        // Strip internal fields from data — UpdateRecordNode
                        // does SET n.field = item.field and _uuid is the PK
                        let clean_data: BTreeMap<String, CypherValue> = data.iter()
                            .filter(|(k, _)| !k.starts_with('_'))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        // Use empty sentinel hash to force UpdateRecordNode to
                        // detect a change and enqueue AggregateRecords. For
                        // composite entities (has both is_content AND content_for),
                        // build_content_text would return the same hash as the
                        // simple pipeline, causing no-op detection.
                        self.pending.updates.push(crate::records::UpdateRecord {
                            entity_name: entity_name.to_string(),
                            uuid,
                            data: clean_data,
                            new_content_hash: String::new(),
                        });
                    }
                    self.drain();
                }

                Ok(FlushResult {
                    processed: record_count,
                    failed: 0,
                    ..Default::default()
                })
            }
            Err(e) => Err(CatalogError::DbError(format!("ingest_entities failed: {e}"))),
        }
    }

    // ── CRUD (synchronous, enqueue operations) ─────────────────────────

    pub fn create(
        &mut self,
        entity_name: &str,
        data: BTreeMap<String, CypherValue>,
    ) -> Result<EntityRef, CatalogError> {
        self.check_initialized()?;
        let entity_def = self.check_entity(entity_name)?.clone();

        // Generate UUID (hashsafe if configured, otherwise random)
        let uuid = if let Some(ref hashsafe_fields) = entity_def.hashsafe {
            let field_values: Vec<&str> = hashsafe_fields
                .iter()
                .map(|f| data.get(f).and_then(|v| v.as_str()).unwrap_or(""))
                .collect();
            hashsafe_uuid(entity_name, &field_values)
        } else {
            crate::refs::generate_temp_uuid()
        };

        // Compute content hash
        let content_text = self.build_content_text(entity_name, &data);
        let hash = content_hash(&content_text);

        // Build full data with system columns
        let mut full_data = data.clone();
        full_data.insert("_uuid".to_string(), CypherValue::String(uuid.clone()));
        full_data.insert(
            "_content_hash".to_string(),
            CypherValue::String(hash),
        );

        // Create entity ref pair
        let (entity_ref, resolver) = EntityRef::new(entity_name);

        // Push entity record with resolver into PendingWork
        self.pending.entities.push(EntityRecord::new(
            entity_name.to_string(),
            full_data,
            resolver,
            entity_ref.clone(),
        ));
        self.drain_counters.total_queued += 1;

        // For each KB where this entity has titleFor, create Index entry + Link + Aggregate.
        let entity_kbs = resolve_entity_kbs(&entity_def);
        for (kb_name, mapping) in &entity_kbs {
            if mapping.title_field.is_none() {
                continue; // This entity only has contentFor for this KB, not titleFor
            }
            let title_field = mapping.title_field.as_ref().unwrap();

            // Build {KB}_Index entry data
            let index_uuid = hashsafe_uuid(
                &format!("{kb_name}_Index"),
                &[entity_name, &uuid],
            );
            let title_max_chars = self.kb_metadata.get(kb_name.as_str())
                .map(|m| m.chunking.title_max_chars)
                .unwrap_or(256);
            let raw_title = data
                .get(title_field)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let title_text: String = if title_max_chars > 0 && raw_title.len() > title_max_chars {
                raw_title.chars().take(title_max_chars).collect()
            } else {
                raw_title.to_string()
            };

            // Collect content from this entity's own contentFor fields
            let mut content_parts: Vec<String> = Vec::new();
            for field_name in &mapping.content_fields {
                if let Some(text) = data.get(field_name).and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        content_parts.push(text.to_string());
                    }
                }
            }
            let content_text = content_parts.join("\n");

            let index_table = format!("{kb_name}_Index");
            let mut index_data = BTreeMap::new();
            index_data.insert("_uuid".to_string(), CypherValue::String(index_uuid.clone()));
            index_data.insert("_source_entity".to_string(), CypherValue::String(entity_name.to_string()));
            index_data.insert("_source_uuid".to_string(), CypherValue::String(uuid.clone()));
            // Sentinel hash: empty string forces KBGatherNode to always run on first drain.
            index_data.insert("_content_hash".to_string(), CypherValue::String(String::new()));
            index_data.insert("_title".to_string(), CypherValue::String(title_text));
            index_data.insert("_content".to_string(), CypherValue::String(content_text));

            // Index entity with resolver
            let (index_ref, index_resolver) = EntityRef::new(&index_table);
            self.pending.entities.push(EntityRecord::new(
                index_table,
                index_data,
                index_resolver,
                index_ref.clone(),
            ));

            // Link: {Entity}_IN_{KB}
            let in_rel_name = format!("{entity_name}_IN_{kb_name}");
            let (in_rel_ref, in_rel_resolver) = RelationRef::new(&in_rel_name);
            self.pending.relations.push(RelationRecord::new(
                in_rel_name,
                RefOrUuid::Ref(entity_ref.clone()),
                RefOrUuid::Ref(index_ref),
                BTreeMap::new(),
                in_rel_resolver,
                in_rel_ref,
            ));

            // Aggregate (deferred: will rebuild _content + chunks at drain time)
            self.pending.aggregates.push(AggregateRecord {
                index_entry_uuid: index_uuid,
                kb_name: kb_name.clone(),
                title_entity: entity_name.to_string(),
                source_uuid: uuid.clone(),
            });

            self.drain_counters.total_queued += 3; // index entity + link + aggregate
        }

        Ok(entity_ref)
    }

    pub fn link(
        &mut self,
        rel_name: &str,
        from: impl Into<RefOrUuid>,
        to: impl Into<RefOrUuid>,
        properties: BTreeMap<String, CypherValue>,
    ) -> Result<RelationRef, CatalogError> {
        self.check_initialized()?;

        let rel_def = self.config.relations.get(rel_name)
            .ok_or_else(|| CatalogError::UnknownRelation(rel_name.to_string()))?;
        let from_entity = rel_def.from.clone();
        let to_entity = rel_def.to.clone();

        let from_ref: RefOrUuid = from.into();
        let to_ref: RefOrUuid = to.into();

        let (relation_ref, resolver) = RelationRef::new(rel_name);

        // Push relation record with resolver into PendingWork
        self.pending.relations.push(RelationRecord::new(
            rel_name.to_string(),
            from_ref.clone(),
            to_ref.clone(),
            properties,
            resolver,
            relation_ref.clone(),
        ));
        self.drain_counters.total_queued += 1;

        // Incremental: if this relation connects a content entity to a title entity
        // for a KB, enqueue an AggregateRecord so the title entity's index is rebuilt.
        // Only when UUIDs are already resolved (incremental case). In batch mode,
        // UUIDs are pending EntityRefs and create() already enqueued AggregateRecords.
        for (kb_name, kb_meta) in &self.kb_metadata {
            let title_entity = &kb_meta.title.entity;
            let title_uuid = if from_entity == *title_entity && kb_meta.entities.contains(&to_entity) {
                from_ref.try_resolve().ok()
            } else if to_entity == *title_entity && kb_meta.entities.contains(&from_entity) {
                to_ref.try_resolve().ok()
            } else {
                None
            };
            if let Some(t_uuid) = title_uuid {
                let index_uuid = hashsafe_uuid(
                    &format!("{kb_name}_Index"),
                    &[title_entity, &t_uuid],
                );
                self.pending.aggregates.push(AggregateRecord {
                    index_entry_uuid: index_uuid,
                    kb_name: kb_name.clone(),
                    title_entity: title_entity.clone(),
                    source_uuid: t_uuid,
                });
                self.drain_counters.total_queued += 1;
            }
        }

        Ok(relation_ref)
    }

    // ── Direct DB reads ────────────────────────────────────────────────

    pub fn get(
        &self,
        entity_name: &str,
        uuid: &str,
    ) -> Result<Option<BTreeMap<String, CypherValue>>, CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;

        let cypher = self.dialect.select_entity_all_by_uuids(entity_name);
        let result = self
            .conn
            .execute_with_params(&cypher, &[QueryParam::new("uuids", CypherValue::List(vec![CypherValue::String(uuid.to_string())]))])
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        if result.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.row_to_map(&result.columns, &result.rows[0])))
    }

    pub fn get_many(
        &self,
        entity_name: &str,
        uuids: &[String],
    ) -> Result<Vec<BTreeMap<String, CypherValue>>, CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;

        if uuids.is_empty() {
            return Ok(vec![]);
        }

        let uuid_list = CypherValue::List(
            uuids
                .iter()
                .map(|u| CypherValue::String(u.clone()))
                .collect(),
        );
        let cypher = self.dialect.select_entity_all_by_uuids(entity_name);
        let result = self
            .conn
            .execute_with_params(
                &cypher,
                &[QueryParam {
                    name: "uuids".to_string(),
                    value: uuid_list,
                }],
            )
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        Ok(result
            .rows
            .iter()
            .map(|row| self.row_to_map(&result.columns, row))
            .collect())
    }

    pub fn exists(
        &self,
        entity_name: &str,
        uuid: &str,
    ) -> Result<bool, CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;

        let cypher = self.dialect.exists_by_uuid(entity_name);
        let result = self
            .conn
            .execute_with_params(&cypher, &[QueryParam::new("uuid", uuid)])
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        let count = result
            .rows
            .get(0)
            .and_then(|r| r.get(0))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        Ok(count > 0)
    }

    pub fn count(&self, entity_name: &str) -> Result<usize, CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;

        let cypher = self.dialect.count_rows(entity_name);
        let result = self
            .conn
            .execute(&cypher)
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        let count = result
            .rows
            .get(0)
            .and_then(|r| r.get(0))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        Ok(count as usize)
    }

    // ── Update / Delete (sync enqueue, executed at drain) ─────────────

    /// Enqueue a field update. Executed at the next `drain()` call.
    ///
    /// Content hash is pre-computed; at drain time, `UpdateRecordNode` reads the
    /// old hash from DB, detects changes, batch-SETs fields, and emits rechunk
    /// requests for changed simple entities.
    pub fn update(
        &mut self,
        entity_name: &str,
        uuid: &str,
        data: BTreeMap<String, CypherValue>,
    ) -> Result<(), CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;
        let new_content = self.build_content_text(entity_name, &data);
        let new_content_hash = content_hash(&new_content);
        self.pending.updates.push(crate::records::UpdateRecord {
            entity_name: entity_name.to_string(),
            uuid: uuid.to_string(),
            data,
            new_content_hash,
        });
        Ok(())
    }

    /// Enqueue an entity deletion. Executed at the next `drain()` call.
    ///
    /// At drain time, `DeleteRecordNode` cascade-deletes chunks, index entries,
    /// and the entity itself, then emits aggregate requests for affected KBs.
    pub fn delete(
        &mut self,
        entity_name: &str,
        uuid: &str,
    ) -> Result<(), CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;
        self.pending.deletes.push(crate::records::DeleteRecord {
            entity_name: entity_name.to_string(),
            uuid: uuid.to_string(),
        });
        Ok(())
    }

    // ── Queue control ──────────────────────────────────────────────────

    /// Build a dataflow graph from all pending records.
    ///
    /// Consumes `self.pending` (PendingWork) and builds a record-based graph:
    ///
    /// ```text
    /// entities → InsertRecordNode("inserts")
    ///                 └── done → LinkRecordNode("links") ← relations
    ///                               └── done → KBGatherNode("gather_kb") ← aggregates
    ///                                             └── kb_content → KBUpdateNode("update_kb")
    ///                                                                  └── kb_content → KBChunkNode("chunk_kb")
    ///                                                                                      ├── entities → InsertRecordNode("agg_inserts")
    ///                                                                                      ├── relations → LinkRecordNode("agg_links")
    ///                                                                                      └── agg_inserts ── done → KBEmbedNode("agg_embeds")
    /// ```
    ///
    /// No KBChunkRecordNode (entity-level chunks unused by search — future Mermaid template).
    /// No KBEmbedNode on raw entities (only KB_Index_Chunk are searched).
    fn build_ingestion_graph(&mut self) -> (
        DataflowGraph, ServiceRegistry, usize,
        Arc<Mutex<Vec<UpdateResult>>>, Arc<Mutex<Vec<DeleteResult>>>,
    ) {
        let mut pending = std::mem::take(&mut self.pending);
        let empty_results = || (
            DataflowGraph::new(), ServiceRegistry::new(), 0,
            Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(Vec::new())),
        );
        if pending.is_empty() {
            return empty_results();
        }

        // ─── Conflict resolution: delete wins over update for same UUID ───
        if !pending.deletes.is_empty() && !pending.updates.is_empty() {
            let delete_set: std::collections::HashSet<(&str, &str)> = pending.deletes.iter()
                .map(|d| (d.entity_name.as_str(), d.uuid.as_str()))
                .collect();
            let before = pending.updates.len();
            pending.updates.retain(|u| !delete_set.contains(&(u.entity_name.as_str(), u.uuid.as_str())));
            let dropped = before - pending.updates.len();
            if dropped > 0 {
                eprintln!("[conflict-resolution] dropped {dropped} update(s) superseded by delete");
            }
        }

        let op_count = pending.total_count();
        let has_entities = !pending.entities.is_empty();
        let has_relations = !pending.relations.is_empty();
        let has_aggregates = !pending.aggregates.is_empty();
        let has_deletes = !pending.deletes.is_empty();
        let has_updates = !pending.updates.is_empty();

        // Shared result containers — cloned Arcs returned to drain() for extraction
        let update_results: Arc<Mutex<Vec<UpdateResult>>> = Arc::new(Mutex::new(Vec::new()));
        let delete_results: Arc<Mutex<Vec<DeleteResult>>> = Arc::new(Mutex::new(Vec::new()));

        // Seed pending_aggregates with initial aggregates; DeleteRecordNode and
        // UpdateRecordNode will push additional ones during execution.
        let pending_aggregates: Arc<Mutex<Vec<AggregateRecord>>> =
            Arc::new(Mutex::new(pending.aggregates));

        // KB pipeline needed if initial aggregates or delete/update might produce more
        let needs_kb = has_aggregates
            || (!self.kb_metadata.is_empty() && (has_deletes || has_updates));

        // Capture KB index table names for FTS flush
        let flush_tables: Vec<String> = if needs_kb {
            self.kb_metadata.keys().map(|k| format!("{k}_Index")).collect()
        } else {
            vec![]
        };

        // Collect updated entity names before pending.updates is moved
        let update_entity_tables: Vec<String> = if has_updates {
            pending.updates.iter()
                .map(|u| u.entity_name.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect()
        } else {
            vec![]
        };

        // Warm chunker cache if needed by KB pipeline or rechunk pipeline
        if needs_kb || has_updates {
            self.warm_chunker_cache();
        }

        let mut graph = DataflowGraph::new();

        // ─── 0. DeleteRecordNode ────────────────────────────────────
        if has_deletes {
            graph.add_node(Box::new(DeleteRecordNode::new("deletes"))).unwrap();
            graph.set_initial_input("deletes", "deletes",
                PortValue::new(BatchPayload::new(PortType::Deletes, pending.deletes)));
        }

        // ─── 1. UpdateRecordNode ────────────────────────────────────
        if has_updates {
            graph.add_node(Box::new(UpdateRecordNode::new("updates"))).unwrap();
            graph.set_initial_input("updates", "updates",
                PortValue::new(BatchPayload::new(PortType::Updates, pending.updates)));
            if has_deletes {
                graph.connect("deletes", "done", "updates", "trigger").unwrap();
            }
        }

        // ─── 2. InsertRecordNode("inserts") — raw entities ─────────
        if has_entities {
            graph.add_node(Box::new(InsertRecordNode::new("inserts"))).unwrap();
            graph.set_initial_input("inserts", "entities",
                PortValue::new(BatchPayload::new(PortType::Entities, pending.entities)));
            // Ordering: deletes → updates → inserts
            if has_updates {
                graph.connect("updates", "done", "inserts", "trigger").unwrap();
            } else if has_deletes {
                graph.connect("deletes", "done", "inserts", "trigger").unwrap();
            }
        }

        // ─── 3. LinkRecordNode("links") — raw relations ────────────
        if has_relations {
            graph.add_node(Box::new(LinkRecordNode::new("links"))).unwrap();
            graph.set_initial_input("links", "relations",
                PortValue::new(BatchPayload::new(PortType::Relations, pending.relations)));
            if has_entities {
                graph.connect("inserts", "done", "links", "trigger").unwrap();
            }
        }

        // ─── 4. Rechunk pipeline (updated simple entities) ─────────
        if has_updates {
            graph.add_node(Box::new(RechunkDeleteNode::new("rechunk_delete"))).unwrap();
            graph.connect("updates", "rechunk_entities", "rechunk_delete", "entities").unwrap();

            graph.add_node(Box::new(ChunkRecordNode::new("rechunk_chunk"))).unwrap();
            graph.connect("rechunk_delete", "entities", "rechunk_chunk", "entities").unwrap();

            graph.add_node(Box::new(InsertRecordNode::new("rechunk_insert"))).unwrap();
            graph.connect("rechunk_chunk", "chunks", "rechunk_insert", "entities").unwrap();

            graph.add_node(Box::new(LinkRecordNode::new("rechunk_link"))).unwrap();
            graph.connect("rechunk_chunk", "chunk_links", "rechunk_link", "relations").unwrap();
            graph.connect("rechunk_insert", "done", "rechunk_link", "trigger").unwrap();

            // Signals resolved per-entity inside EmbedNode via entity_configs service.
            // The fallback signal here is unused when entity_configs is registered.
            graph.add_node(Box::new(EmbedNode::new("rechunk_embed", search::SearchSignals::BM25, 32))).unwrap();
            graph.connect("rechunk_insert", "inserted", "rechunk_embed", "entities").unwrap();
            graph.connect("rechunk_link", "done", "rechunk_embed", "trigger").unwrap();

            // Flush FTS for updated entity tables
            graph.add_node(Box::new(FlushNode::new("rechunk_flush", update_entity_tables))).unwrap();
            graph.connect("rechunk_embed", "done", "rechunk_flush", "trigger").unwrap();
        }

        // ─── 5. KB pipeline: gather → update → chunk ───────────────
        if needs_kb {
            // KBGatherNode reads from pending_aggregates service (not port input).
            // It must wait until all aggregate producers (delete, update) are done.
            graph.add_node(Box::new(KBGatherNode::new("gather_kb"))).unwrap();
            if has_relations {
                graph.connect("links", "done", "gather_kb", "trigger").unwrap();
            } else if has_entities {
                graph.connect("inserts", "done", "gather_kb", "trigger").unwrap();
            } else if has_updates {
                graph.connect("updates", "done", "gather_kb", "trigger").unwrap();
            } else if has_deletes {
                graph.connect("deletes", "done", "gather_kb", "trigger").unwrap();
            }

            graph.add_node(Box::new(KBUpdateNode::new("update_kb"))).unwrap();
            graph.connect("gather_kb", "kb_content", "update_kb", "kb_content").unwrap();

            graph.add_node(Box::new(KBChunkNode::new("chunk_kb"))).unwrap();
            graph.connect("update_kb", "kb_content", "chunk_kb", "kb_content").unwrap();

            graph.add_node(Box::new(InsertRecordNode::new("agg_inserts"))).unwrap();
            graph.connect("chunk_kb", "entities", "agg_inserts", "entities").unwrap();

            graph.add_node(Box::new(LinkRecordNode::new("agg_links"))).unwrap();
            graph.connect("chunk_kb", "relations", "agg_links", "relations").unwrap();
            graph.connect("agg_inserts", "done", "agg_links", "trigger").unwrap();

            graph.add_node(Box::new(KBEmbedNode::new("agg_embeds", 32))).unwrap();
            graph.connect("agg_inserts", "inserted", "agg_embeds", "entities").unwrap();
            graph.connect("agg_links", "done", "agg_embeds", "trigger").unwrap();

            graph.add_node(Box::new(FlushNode::new("flush_fts", flush_tables.clone()))).unwrap();
            graph.connect("update_kb", "done", "flush_fts", "trigger").unwrap();
        }

        // ─── Services ──────────────────────────────────────────────
        let mut services = ServiceRegistry::new();
        services.register("conn", self.conn.clone());
        services.register("dialect", self.dialect.clone());
        services.register("node_id_cache", self.node_id_cache.clone());
        services.register("embedder", self.embedder.clone());
        services.register("embedding_dim", self.config.embedding_dim);
        services.register("config", self.config.clone());
        services.register("kb_metadata", self.kb_metadata.clone());
        services.register("has_sparse",
            self.sparse_embedder.is_some() || self.dual_embedder.is_some());
        services.register("has_dual", self.dual_embedder.is_some());
        services.register("sparse_handles", self.sparse_handles.clone());
        services.register("fts_handles", self.fts_handles.clone());

        // Shared services for delete/update nodes
        services.register("pending_aggregates", pending_aggregates);
        services.register("update_results", update_results.clone());
        services.register("delete_results", delete_results.clone());

        // Event bus for node-emitted lifecycle events + warnings
        services.register("event_bus", Arc::new(self.event_bus.shared()));

        // entity_configs needed by DeleteRecordNode, UpdateRecordNode, ChunkRecordNode
        if has_deletes || has_updates || needs_kb {
            services.register("entity_configs", self.entity_configs.clone());
        }

        // chunker_cache needed by KBChunkNode and ChunkRecordNode (rechunk)
        if needs_kb || has_updates {
            services.register("chunker_cache", Arc::new(std::mem::take(&mut self.chunker_cache)));
        }
        if let Some(ref sparse_emb) = self.sparse_embedder {
            services.register("sparse_embedder", sparse_emb.clone());
        }
        if let Some(ref dual_emb) = self.dual_embedder {
            services.register("dual_embedder", dual_emb.clone());
        }
        if let Some(ref fail_node) = self.fail_node {
            services.register("fail_node", fail_node.clone());
        }

        (graph, services, op_count, update_results, delete_results)
    }

    /// Drain all pending operations via the dataflow runtime with checkpoint persistence.
    pub fn drain(&mut self) -> FlushResult {
        let (mut graph, services, op_count, update_results, delete_results) =
            self.build_ingestion_graph();
        if graph.nodes.is_empty() {
            return FlushResult::default();
        }

        let node_count = graph.nodes.len();
        let runtime = DataflowRuntime::with_services(node_count + 20, services);

        // Generate deterministic execution_id from graph hash + timestamp
        let graph_def = graph.to_definition();
        let execution_id = format!(
            "drain-{}-{}",
            &graph_def.hash()[..12],
            crate::dataflow::checkpoint::timestamp_ms(),
        );

        let result = if let Some(ref store) = self.checkpoint_store {
            runtime
                .execute_with_checkpoint(&mut graph, store.as_ref(), &execution_id)
        } else {
            runtime.execute(&mut graph)
        };

        match result {
            Ok(_output) => {
                self.drain_counters.total_processed += op_count;
                self.drain_counters.flush_count += 1;
                // Extract results from shared services
                let updates = std::mem::take(
                    &mut *update_results.lock().unwrap_or_else(|e| e.into_inner()),
                );
                let deletes = std::mem::take(
                    &mut *delete_results.lock().unwrap_or_else(|e| e.into_inner()),
                );
                FlushResult {
                    processed: op_count,
                    failed: 0,
                    update_results: updates,
                    delete_results: deletes,
                }
            }
            Err(e) => {
                eprintln!("[rag3weaver] drain FAILED: {e}");
                self.event_bus.emit(CatalogEvent::Error {
                    context: "drain".to_string(),
                    message: format!("ingestion dataflow failed: {e}"),
                });
                self.drain_counters.total_failed += op_count;
                self.drain_counters.flush_count += 1;
                FlushResult { processed: 0, failed: op_count, ..Default::default() }
            }
        }
    }

    /// Drain (WASM-only alias kept for the FFI surface).
    /// `drain()` is synchronous since the async→sync migration; the rayon pool
    /// argument is retained for call-site compatibility.
    #[cfg(feature = "wasm-emscripten")]
    pub fn drain_parallel(&mut self, _pool: &rayon::ThreadPool) -> FlushResult {
        self.drain()
    }

    /// Flush only entity inserts via a minimal dataflow graph.
    /// Leaves relations and aggregates in `pending` for a later `drain()`.
    pub fn flush_insertions(&mut self) -> FlushResult {
        let entities = std::mem::take(&mut self.pending.entities);
        if entities.is_empty() {
            return FlushResult::default();
        }

        let op_count = entities.len();
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(InsertRecordNode::new("inserts"))).unwrap();
        graph.set_initial_input("inserts", "entities",
            PortValue::new(BatchPayload::new(PortType::Entities, entities)));

        let mut services = ServiceRegistry::new();
        services.register("conn", self.conn.clone());
        services.register("dialect", self.dialect.clone());
        services.register("node_id_cache", self.node_id_cache.clone());

        let runtime = DataflowRuntime::with_services(5, services);
        match runtime.execute(&mut graph) {
            Ok(_) => {
                self.drain_counters.total_processed += op_count;
                self.drain_counters.flush_count += 1;
                FlushResult { processed: op_count, failed: 0, ..Default::default() }
            }
            Err(e) => {
                self.event_bus.emit(CatalogEvent::Error {
                    context: "flush_insertions".to_string(),
                    message: format!("insert-only dataflow failed: {e}"),
                });
                self.drain_counters.total_failed += op_count;
                self.drain_counters.flush_count += 1;
                FlushResult { processed: 0, failed: op_count, ..Default::default() }
            }
        }
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Access the PendingWork queue.
    pub fn pending_work(&self) -> &PendingWork {
        &self.pending
    }

    pub fn drain_stats(&self) -> DrainStats {
        DrainStats {
            pending: self.pending.total_count(),
            failed: self.drain_counters.total_failed,
            total_queued: self.drain_counters.total_queued,
            total_processed: self.drain_counters.total_processed,
            total_failed: self.drain_counters.total_failed,
            flush_count: self.drain_counters.flush_count,
        }
    }

    /// Resume a previously failed drain execution from its checkpoint.
    ///
    /// Reconstructs the graph from the checkpointed `GraphDefinition`
    /// (nodes + edges), then calls `execute_with_checkpoint()` which skips
    /// already-completed nodes and resumes from the failure point.
    pub fn drain_resume(&mut self, execution_id: &str) -> Result<FlushResult, CatalogError> {
        let store = self
            .checkpoint_store
            .clone()
            .ok_or(CatalogError::NotInitialized)?;

        // Load the checkpoint to get the graph definition
        let checkpoint = store
            .load_execution(execution_id)
            .map_err(|e| CatalogError::DbError(e))?
            .ok_or_else(|| {
                CatalogError::DbError(format!("checkpoint not found: {execution_id}"))
            })?;

        // Reconstruct the graph from the checkpointed definition
        let mut registry = NodeRegistry::new();
        register_builtins(&mut registry);

        let mut graph = DataflowGraph::new();
        for node_def in &checkpoint.graph_def.nodes {
            let node = registry
                .create(&node_def.node_type, &node_def.name, &node_def.config)
                .map_err(|e| CatalogError::DbError(e))?;
            graph.add_node(node).map_err(|e| CatalogError::DbError(e))?;
        }
        for edge_def in &checkpoint.graph_def.edges {
            graph
                .connect(
                    &edge_def.from_node,
                    &edge_def.from_port,
                    &edge_def.to_node,
                    &edge_def.to_port,
                )
                .map_err(|e| CatalogError::DbError(e))?;
        }

        // Rebuild the ServiceRegistry (same as build_ingestion_graph)
        let mut services = ServiceRegistry::new();
        services.register("conn", self.conn.clone());
        services.register("dialect", self.dialect.clone());
        services.register("node_id_cache", self.node_id_cache.clone());
        services.register("embedder", self.embedder.clone());
        services.register("embedding_dim", self.config.embedding_dim);
        services.register("config", self.config.clone());
        services.register("kb_metadata", self.kb_metadata.clone());
        services.register("has_sparse",
            self.sparse_embedder.is_some() || self.dual_embedder.is_some());
        services.register("has_dual", self.dual_embedder.is_some());
        services.register("sparse_handles", self.sparse_handles.clone());
        services.register("fts_handles", self.fts_handles.clone());

        // Chunker cache: rebuild for KB nodes
        self.warm_chunker_cache();
        services.register("chunker_cache", Arc::new(std::mem::take(&mut self.chunker_cache)));

        if let Some(ref sparse_emb) = self.sparse_embedder {
            services.register("sparse_embedder", sparse_emb.clone());
        }
        if let Some(ref dual_emb) = self.dual_embedder {
            services.register("dual_embedder", dual_emb.clone());
        }
        if let Some(ref fail_node) = self.fail_node {
            services.register("fail_node", fail_node.clone());
        }

        let node_count = graph.nodes.len();
        let runtime = DataflowRuntime::with_services(node_count + 20, services);

        match runtime
            .execute_with_checkpoint(&mut graph, store.as_ref(), execution_id)
        {
            Ok(_) => {
                self.drain_counters.flush_count += 1;
                Ok(FlushResult {
                    processed: node_count,
                    failed: 0,
                    ..Default::default()
                })
            }
            Err(e) => {
                self.event_bus.emit(CatalogEvent::Error {
                    context: "drain_resume".to_string(),
                    message: format!("resume failed: {e}"),
                });
                self.drain_counters.flush_count += 1;
                Ok(FlushResult {
                    processed: 0,
                    failed: node_count,
                    ..Default::default()
                })
            }
        }
    }

    /// Check for incomplete checkpoint executions (status=Running).
    ///
    /// Returns execution IDs that can be passed to `drain_resume()`.
    pub fn check_pending_checkpoints(&self) -> Result<Vec<String>, CatalogError> {
        let store = self
            .checkpoint_store
            .as_ref()
            .ok_or(CatalogError::NotInitialized)?;
        store
            .find_incomplete()
            .map_err(|e| CatalogError::DbError(e))
    }

    /// Direct access to the underlying connection (useful for debugging/tests).
    pub fn conn(&self) -> &dyn DbConnection {
        self.conn.as_ref()
    }

    /// Get a cloned Arc to the connection (for recording, observability).
    pub fn conn_arc(&self) -> Arc<dyn DbConnection> {
        self.conn.clone()
    }

    /// Execute raw Cypher (useful for debugging/tests).
    pub fn execute_raw(&self, cypher: &str) -> Result<crate::connection::QueryResult, CatalogError> {
        self.conn.execute(cypher).map_err(|e| CatalogError::DbError(e.to_string()))
    }

    // ── Event bus ──────────────────────────────────────────────────────

    pub fn subscribe(&self) -> async_broadcast::Receiver<CatalogEvent> {
        self.event_bus.subscribe()
    }

    // ── Node ID cache ─────────────────────────────────────────────────

    /// Access the shared node ID cache (uuid → internal rag3db node ID).
    /// Populated automatically by InsertRecordNode on each INSERT.
    pub fn node_id_cache(&self) -> &Arc<RwLock<NodeIdCache>> {
        &self.node_id_cache
    }

    // ── Schema queries ─────────────────────────────────────────────────

    pub fn get_kb_metadata(&self, kb_name: &str) -> Option<&KBMetadata> {
        self.kb_metadata.get(kb_name)
    }

    pub fn get_entity_def(&self, name: &str) -> Option<&EntityDef> {
        self.config.entities.get(name)
    }

    pub fn get_relation_def(&self, name: &str) -> Option<&RelationDef> {
        self.config.relations.get(name)
    }

    pub fn get_kbs_for_entity(&self, entity_name: &str) -> Vec<&str> {
        self.kb_metadata
            .iter()
            .filter(|(_, kb)| kb.entities.contains(entity_name))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    // ── Search ─────────────────────────────────────────────────────────

    pub fn search(
        &mut self,
        name: &str,
        query: &str,
        options: search::SearchOptions,
    ) -> Result<search::SearchResponse, CatalogError> {
        self.check_initialized()?;

        let target = self.resolve_search_target(name)?;

        let pending_count = self.pending.total_count();

        // Consistency
        match options.consistency {
            search::Consistency::Strict => {
                self.drain();
            }
            search::Consistency::Eventual => {
                if self.has_pending() {
                    self.flush_insertions();
                }
            }
            search::Consistency::Immediate => {}
        }

        // Resolve signals: per-query override > target default
        let signals = options.signals.unwrap_or(target.default_signals);

        let search_limit = (options.limit + options.offset) * 2;
        let entity = &target.parent_table;
        let vector_entity = &target.chunk_table;
        let bm25_fields = &target.bm25_fields;
        let enrich_fields = &target.enrich_fields;

        // Parse filters: filter_condition takes priority over legacy filters HashMap
        let condition: Option<FilterCondition> = if options.filter_condition.is_some() {
            options.filter_condition.clone()
        } else if !options.filters.is_empty() {
            Some(options.filters.clone().into())
        } else {
            None
        };

        // For vector search: compile ALL filters to Cypher WHERE (already pre-filter)
        let (filter_where, filter_params, filter_match) = if let Some(ref cond) = condition {
            let mut parser = FilterParser::new(&self.config.relations, self.dialect.as_ref());
            let parsed = parser
                .parse_condition(cond, &entity, "n")
                .map_err(|e| CatalogError::FilterError(e.to_string()))?;
            let where_str = if parsed.where_clauses.is_empty() {
                None
            } else {
                Some(parsed.combine_where())
            };
            let match_str = if parsed.match_clauses.is_empty() {
                None
            } else {
                Some(parsed.match_clauses.join(" "))
            };
            (where_str, parsed.params, match_str)
        } else {
            (None, vec![], None)
        };

        // For BM25 search: resolve ALL filters to allowed_ids.
        // KB: filters resolved via title entity (e.g. Directory) JOINed to {KB}_Index.
        // Simple: filters resolved directly on the entity table.
        let allowed_ids = if let Some(ref cond) = condition {
            let (filter_entity, filter_alias, join_from): (&str, &str, Option<(&str, &str, &str)>) =
                if let Some((ref title_entity, ref in_rel)) = target.filter_indirection {
                    (title_entity.as_str(), "t", Some(("t", title_entity.as_str(), in_rel.as_str())))
                } else {
                    (entity, "n", None)
                };

            let mut parser = FilterParser::new(&self.config.relations, self.dialect.as_ref());
            let parsed = parser
                .parse_condition(cond, filter_entity, filter_alias)
                .map_err(|e| CatalogError::FilterError(e.to_string()))?;

            if !parsed.where_clauses.is_empty() {
                let resolve_table = if join_from.is_some() { entity } else { entity };
                let resolve_alias = if join_from.is_some() { "idx" } else { "n" };
                let query = self.dialect.filter_resolve_offsets(
                    resolve_table,
                    resolve_alias,
                    &parsed.match_clauses,
                    &parsed.combine_where(),
                    join_from,
                );
                let result = if parsed.params.is_empty() {
                    self.conn.execute(&query)
                        .map_err(|e| CatalogError::DbError(e.to_string()))?
                } else {
                    self.conn.execute_with_params(&query, &parsed.params)
                        .map_err(|e| CatalogError::DbError(e.to_string()))?
                };
                let ids: Vec<u64> = result
                    .rows
                    .iter()
                    .filter_map(|r| r.first().and_then(|v| v.as_i64()).map(|i| i as u64))
                    .collect();
                Some(ids)
            } else {
                None
            }
        } else {
            None
        };

        // Both KB and simple entities always have chunks
        let is_chunked = true;

        // ── Timing + diagnostics ───────────────────────────────────────
        let search_start = Instant::now();
        let mut diag = if options.diagnostics {
            Some(search::SearchDiagnostics::default())
        } else {
            None
        };

        // ── Embed query: use dual embedder when both dense+sparse are needed ──
        let need_dense = signals.vector();
        let need_sparse = signals.sparse();

        let t_embed = Instant::now();
        let (embedding, query_sparse) = if need_dense && need_sparse {
            if let Some(ref dual_emb) = self.dual_embedder {
                // Single forward pass → dense + sparse
                let (dense_vecs, sparse_vecs) = dual_emb
                    .embed_dual(&[query.to_string()])
                    .map_err(|e| CatalogError::EmbedError(e.to_string()))?;
                (
                    dense_vecs.into_iter().next().unwrap_or_default(),
                    sparse_vecs.into_iter().next(),
                )
            } else {
                // Fallback: separate embedders
                let dense = search::embed_query(self.embedder.as_ref(), query, &mut self.embedding_cache)?;
                let sparse = if let Some(ref sparse_emb) = self.sparse_embedder {
                    sparse_emb.embed_sparse(&[query.to_string()])
                        .map_err(|e| CatalogError::EmbedError(e.to_string()))?
                        .into_iter().next()
                } else { None };
                (dense, sparse)
            }
        } else if need_dense {
            let dense = if let Some(ref dual_emb) = self.dual_embedder {
                let (dense_vecs, _) = dual_emb.embed_dual(&[query.to_string()])
                    .map_err(|e| CatalogError::EmbedError(e.to_string()))?;
                dense_vecs.into_iter().next().unwrap_or_default()
            } else {
                search::embed_query(self.embedder.as_ref(), query, &mut self.embedding_cache)?
            };
            (dense, None)
        } else if need_sparse {
            let sparse = if let Some(ref dual_emb) = self.dual_embedder {
                let (_, sparse_vecs) = dual_emb.embed_dual(&[query.to_string()])
                    .map_err(|e| CatalogError::EmbedError(e.to_string()))?;
                sparse_vecs.into_iter().next()
            } else if let Some(ref sparse_emb) = self.sparse_embedder {
                sparse_emb.embed_sparse(&[query.to_string()])
                    .map_err(|e| CatalogError::EmbedError(e.to_string()))?
                    .into_iter().next()
            } else { None };
            (vec![], sparse)
        } else {
            (vec![], None)
        };

        if let Some(ref mut d) = diag { d.embed_ms = t_embed.elapsed().as_millis() as u64; }

        // ── Run searches based on signals ─────────────────────────────────
        let t_vector = Instant::now();
        let vector_results = if need_dense {
            search::search_vector_via_backend(
                self.search_backend.as_ref().unwrap().as_ref(),
                vector_entity,
                &embedding,
                search_limit,
                filter_where.as_deref(),
                &filter_params,
                filter_match.as_deref(),
            )?
        } else {
            vec![]
        };

        if let Some(ref mut d) = diag { d.vector_ms = t_vector.elapsed().as_millis() as u64; }

        let t_bm25 = Instant::now();
        let bm25_results = if signals.bm25() {
            if is_chunked {
                search::search_bm25_chunked(
                    self.conn.as_ref(), &target, query, bm25_fields,
                    options.bm25_mode, options.fuzzy_distance, search_limit,
                    allowed_ids.as_deref(), enrich_fields, options.result_mode,
                    diag.as_mut(),
                )?
            } else {
                search::search_bm25(
                    self.conn.as_ref(), entity, query, bm25_fields,
                    options.bm25_mode, options.fuzzy_distance, search_limit,
                    allowed_ids.as_deref(), enrich_fields,
                )?
            }
        } else {
            vec![]
        };

        if let Some(ref mut d) = diag { d.bm25_ms = t_bm25.elapsed().as_millis() as u64; }

        let vector_count = vector_results.len();
        let bm25_count = bm25_results.len();

        let t_sparse = Instant::now();
        let sparse_results = if let Some(qv) = query_sparse {
            if let Some(handle) = self.sparse_handle(vector_entity) {
                let sparse_fields = if is_chunked { &[][..] } else { enrich_fields.as_slice() };
                search::search_sparse_via_backend(
                    &handle,
                    self.search_backend.as_ref().unwrap().as_ref(),
                    vector_entity,
                    &qv,
                    search_limit,
                    sparse_fields,
                )?
            } else {
                vec![]
            }
        } else {
            vec![]
        };
        if let Some(ref mut d) = diag { d.sparse_ms = t_sparse.elapsed().as_millis() as u64; }
        let sparse_count = sparse_results.len();

        // Resolve chunk-level results to parent-level with ChunkInfo + enrichment
        let t_resolve = Instant::now();
        let vector_results = if is_chunked && !vector_results.is_empty() {
            search::resolve_vector_chunks_with_dialect(
                self.conn.as_ref(), &target, vector_results, enrich_fields,
                options.result_mode, self.dialect.as_ref(),
            )?
        } else { vector_results };
        let sparse_results = if is_chunked && !sparse_results.is_empty() {
            search::resolve_vector_chunks_with_dialect(
                self.conn.as_ref(), &target, sparse_results, enrich_fields,
                options.result_mode, self.dialect.as_ref(),
            )?
        } else { sparse_results };

        if let Some(ref mut d) = diag { d.resolve_ms = t_resolve.elapsed().as_millis() as u64; }

        let t_fuse = Instant::now();
        let fusion_config = options.fusion.as_ref()
            .cloned()
            .unwrap_or(target.default_fusion.clone());
        let mut fused = search::fuse_results(
            &vector_results,
            &bm25_results,
            &sparse_results,
            &fusion_config,
        );
        let fused_count = fused.len();

        // Pagination
        if options.offset > 0 {
            if options.offset >= fused.len() {
                fused.clear();
            } else {
                fused = fused.split_off(options.offset);
            }
        }
        fused.truncate(options.limit);

        if let Some(ref mut d) = diag { d.fuse_ms = t_fuse.elapsed().as_millis() as u64; }

        // Enrich results that don't already have data (e.g. vector non-chunked)
        let t_enrich = Instant::now();
        let needs_enrich: bool = fused.iter().any(|r| r.data.is_none());
        if needs_enrich && !enrich_fields.is_empty() {
            search::enrich_results_with_data_via_backend(
                self.search_backend.as_ref().unwrap().as_ref(), entity, enrich_fields, &mut fused,
            )?;
        }

        // SourceResolved: resolve index entries → source entities (KB only)
        if target.has_source_refs && options.result_mode == search::ResultMode::SourceResolved {
            self.resolve_to_source_entities(&mut fused)?;
        }

        if let Some(ref mut d) = diag { d.enrich_ms = t_enrich.elapsed().as_millis() as u64; }

        let total_ms = search_start.elapsed().as_millis() as u64;
        if let Some(ref mut d) = diag { d.total_ms = total_ms; }

        self.event_bus.emit(CatalogEvent::SearchCompleted {
            kb: name.to_string(),
            results: fused.len(),
            duration_ms: total_ms,
        });

        Ok(search::SearchResponse {
            results: fused,
            meta: search::SearchMeta {
                query: query.to_string(),
                target: name.to_string(),
                signals,
                consistency: options.consistency,
                partial: pending_count > 0
                    && options.consistency == search::Consistency::Immediate,
                pending_count,
                vector_count,
                bm25_count,
                sparse_count,
                fused_count,
                search_time_ms: total_ms,
                diagnostics: diag,
            },
        })
    }

    /// Resolve index entry results to their source entities.
    ///
    /// Reads `_source_entity` and `_source_uuid` from each result's data,
    /// batch-fetches the source entities, and replaces uuid/entity/data.
    /// Deduplicates by source UUID, keeping the highest score.
    fn resolve_to_source_entities(
        &self,
        results: &mut Vec<search::SearchResult>,
    ) -> Result<(), CatalogError> {
        use crate::connection::CypherValue;

        // 1. Group by entity type → [source_uuid]
        let mut by_entity: HashMap<String, Vec<String>> = HashMap::new();
        for r in results.iter() {
            if let Some(ref data) = r.data {
                let entity = data.get("_source_entity").and_then(|v| v.as_str());
                let uuid = data.get("_source_uuid").and_then(|v| v.as_str());
                if let (Some(e), Some(u)) = (entity, uuid) {
                    by_entity.entry(e.to_string()).or_default().push(u.to_string());
                }
            }
        }

        // 2. Batch fetch source entity data
        let mut source_data: HashMap<String, (String, BTreeMap<String, CypherValue>)> = HashMap::new();
        for (entity_name, uuids) in &by_entity {
            let deduped: HashSet<&str> = uuids.iter().map(|s| s.as_str()).collect();
            let uuid_param = CypherValue::List(
                deduped.iter().map(|u| CypherValue::String(u.to_string())).collect(),
            );
            let cypher = self.dialect.select_entity_all_by_uuids(entity_name);
            let result = self.conn
                .execute_with_params(
                    &cypher,
                    &[QueryParam { name: "uuids".into(), value: uuid_param }],
                )
                .map_err(|e| CatalogError::DbError(e.to_string()))?;

            for row in &result.rows {
                if let Some(CypherValue::Map(map)) = row.first() {
                    if let Some(uuid) = map.get("_uuid").and_then(|v| v.as_str()) {
                        source_data.insert(
                            uuid.to_string(),
                            (entity_name.clone(), map.clone()),
                        );
                    }
                }
            }
        }

        // 3. Replace uuid/entity/data for each result
        for r in results.iter_mut() {
            let source_uuid = r.data.as_ref()
                .and_then(|d| d.get("_source_uuid"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            if let Some(su) = source_uuid {
                if let Some((entity_name, data)) = source_data.get(&su) {
                    r.uuid = su;
                    r.entity = Some(entity_name.clone());
                    r.data = Some(data.clone());
                }
            }
        }

        // 4. Deduplicate by UUID (same source entity), keep highest score
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut to_remove: Vec<usize> = Vec::new();
        for (i, r) in results.iter().enumerate() {
            if let Some(&prev_idx) = seen.get(&r.uuid) {
                if r.score > results[prev_idx].score {
                    to_remove.push(prev_idx);
                    seen.insert(r.uuid.clone(), i);
                } else {
                    to_remove.push(i);
                }
            } else {
                seen.insert(r.uuid.clone(), i);
            }
        }
        to_remove.sort_unstable();
        to_remove.dedup();
        for idx in to_remove.into_iter().rev() {
            results.remove(idx);
        }

        Ok(())
    }

    pub fn search_with_explore(
        &mut self,
        kb_name: &str,
        query: &str,
        options: search::ExploreOptions,
    ) -> Result<search::ExploreResult, CatalogError> {
        let response = self.search(kb_name, query, options.search)?;

        let seed_nodes: Vec<search::GraphNode> = response
            .results
            .iter()
            .map(|r| search::GraphNode {
                uuid: r.uuid.clone(),
                entity: r.entity.clone().unwrap_or_default(),
                label: r.uuid.clone(),
                depth: 0,
                is_search_result: true,
                data: BTreeMap::new(),
            })
            .collect();

        let graph = search::explore_bfs(
            self.conn.as_ref(),
            seed_nodes,
            &options.outgoing_relations,
            &options.incoming_relations,
            options.depth,
            options.top_k,
        )?;

        Ok(search::ExploreResult {
            results: response.results,
            graph,
            meta: response.meta,
        })
    }

    // ── Private helpers ────────────────────────────────────────────────

    fn check_initialized(&self) -> Result<(), CatalogError> {
        if !self.initialized {
            Err(CatalogError::NotInitialized)
        } else {
            Ok(())
        }
    }

    fn check_entity(&self, name: &str) -> Result<&EntityDef, CatalogError> {
        self.config
            .entities
            .get(name)
            .ok_or_else(|| CatalogError::UnknownEntity(name.to_string()))
    }

    fn build_content_text(
        &self,
        entity_name: &str,
        data: &BTreeMap<String, CypherValue>,
    ) -> String {
        if let Some(config) = self.entity_configs.get(entity_name) {
            let simple_fields = config.content_fields();
            if !simple_fields.is_empty() {
                // Simple pipeline: use is_content fields
                return simple_fields
                    .iter()
                    .filter_map(|f| data.get(*f).and_then(|v| v.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n\n");
            }
            // KB-only entity registered via register_entity:
            // use all text/string fields with content_for or title_for
            let mut parts = Vec::new();
            let mut field_names: Vec<&String> = config.fields.keys().collect();
            field_names.sort();
            for fname in field_names {
                let f = &config.fields[fname];
                if f.title_for.is_some() || f.content_for.is_some() {
                    if let Some(val) = data.get(fname.as_str()) {
                        if let Some(s) = val.as_str() {
                            parts.push(s.to_string());
                        }
                    }
                }
            }
            return parts.join("\n\n");
        }
        // KB entity path (CatalogConfig): all Text/String fields, "|" separator
        let entity_def = match self.config.entities.get(entity_name) {
            Some(def) => def,
            None => return String::new(),
        };
        let mut parts = Vec::new();
        let mut sorted_fields: Vec<&String> = entity_def.fields.keys().collect();
        sorted_fields.sort();
        for field_name in sorted_fields {
            let field_def = &entity_def.fields[field_name];
            if matches!(field_def.field_type, FieldType::Text | FieldType::String) {
                if let Some(val) = data.get(field_name) {
                    if let Some(s) = val.as_str() {
                        parts.push(s.to_string());
                    }
                }
            }
        }
        parts.join("|")
    }

    /// Pre-warm the chunker cache for all KB and simple entity chunking configs.
    fn warm_chunker_cache(&mut self) {
        for kb in self.kb_metadata.values() {
            let key = ChunkerConfig {
                max_size: kb.chunking.max_size,
                overlap: kb.chunking.overlap,
                strategy: kb.chunking.strategy.clone(),
            };
            self.chunker_cache
                .entry(key.clone())
                .or_insert_with(|| Chunker::new(key));
        }
        for ec in self.entity_configs.values() {
            let key = ChunkerConfig {
                max_size: ec.chunking.max_size,
                overlap: ec.chunking.overlap,
                strategy: ec.chunking.strategy.clone(),
            };
            self.chunker_cache
                .entry(key.clone())
                .or_insert_with(|| Chunker::new(key));
        }
    }

    fn row_to_map(
        &self,
        columns: &[String],
        row: &[CypherValue],
    ) -> BTreeMap<String, CypherValue> {
        let mut data = BTreeMap::new();
        for (i, col) in columns.iter().enumerate() {
            if i < row.len() {
                data.insert(col.clone(), row[i].clone());
            }
        }
        data
    }

    // ── Strategy Search ──────────────────────────────────────────────

    /// Build a configured [`DataflowGraph`] + [`ServiceRegistry`] for search with strategy.
    ///
    /// Use with [`DataflowRuntime`] for event observation:
    /// ```ignore
    /// let (mut graph, services) = Catalog::build_dataflow_graph(catalog, kb, q, strategy);
    /// let runtime = DataflowRuntime::with_services(10, services);
    /// let mut rx = runtime.subscribe();
    /// let output = runtime.execute(&mut graph)?;
    /// ```
    pub fn build_dataflow_graph(
        catalog: Arc<Mutex<Catalog>>,
        kb_name: &str,
        query: &str,
        strategy: crate::search_strategy::SearchStrategy,
    ) -> (crate::dataflow::DataflowGraph, crate::dataflow::ServiceRegistry) {
        use crate::dataflow::*;
        use crate::dataflow::services::ConnService;

        let mut graph = DataflowGraph::new();

        // Services
        let mut services = ServiceRegistry::new();
        let conn = catalog.lock().unwrap().conn_arc();
        services.register("catalog", catalog.clone());
        services.register("conn", ConnService(conn));

        // Source node
        graph
            .add_node(Box::new(KBQuerySourceNode::new(
                kb_name,
                query,
                &strategy.search,
            )))
            .unwrap();

        // Primary search (catalog resolved via service)
        graph
            .add_node(Box::new(KBSearchNode::new("primary_search")))
            .unwrap();
        graph
            .connect("query_source", "query", "primary_search", "query")
            .unwrap();

        // Expansion: one FetchRelatedNode per rule + ComposeNode
        if !strategy.expansions.is_empty() {
            for (i, rule) in strategy.expansions.iter().enumerate() {
                let fetch_name = format!("fetch_related_{i}");
                graph
                    .add_node(Box::new(FetchRelatedNode::new(
                        &fetch_name,
                        rule.relation.clone(),
                        rule.direction.clone(),
                        rule.limit,
                        rule.source_entity.clone(),
                    )))
                    .unwrap();
                graph
                    .connect("primary_search", "results", &fetch_name, "results")
                    .unwrap();
            }

            graph.add_node(Box::new(ComposeNode::new("compose"))).unwrap();
            graph
                .connect("primary_search", "results", "compose", "results")
                .unwrap();
            for i in 0..strategy.expansions.len() {
                graph
                    .connect(&format!("fetch_related_{i}"), "children", "compose", "children")
                    .unwrap();
            }
        }

        (graph, services)
    }

    /// Run a search with reactive expansion (graph traversal after search).
    ///
    /// This is an associated function taking `Arc<Mutex<Catalog>>` so that
    /// nodes can call `catalog.search()`.
    ///
    /// For event observation, use [`Self::build_dataflow_graph()`] +
    /// [`DataflowRuntime::subscribe()`] + [`DataflowRuntime::execute()`].
    pub fn search_with_strategy(
        catalog: Arc<Mutex<Catalog>>,
        kb_name: &str,
        query: &str,
        strategy: crate::search_strategy::SearchStrategy,
    ) -> Result<crate::search_strategy::SearchStrategyResponse, CatalogError> {
        let has_expansions = !strategy.expansions.is_empty();
        let (mut graph, services) =
            Self::build_dataflow_graph(catalog, kb_name, query, strategy);

        let output = crate::dataflow::execute_via_luciole(
            &mut graph,
            std::sync::Arc::new(services),
        )
        .map_err(|e| CatalogError::DbError(e))?;

        // Results from terminal node
        let results_node = if has_expansions {
            "compose"
        } else {
            "primary_search"
        };
        let results = output
            .get(results_node, "results")
            .and_then(|v| v.downcast::<Vec<crate::search_strategy::UnifiedResult>>())
            .cloned()
            .unwrap_or_default();

        let meta = output
            .get("primary_search", "meta")
            .and_then(|v| v.downcast::<crate::search::SearchMeta>())
            .cloned()
            .ok_or_else(|| {
                CatalogError::DbError(
                    "search_with_strategy: no meta after processing".into(),
                )
            })?;

        Ok(crate::search_strategy::SearchStrategyResponse { results, meta })
    }
}

// ─── Migration support ──────────────────────────────────────────────────────
//
// Internal methods used by MigrationRunner. All DB logic for migrations lives
// here so the runner remains a pure orchestrator (filesystem + ordering).

use crate::dataflow::migrations::{AppliedMigration, MigrationError, MigrationFile};
use crate::dataflow::checkpoint::{ExecutionCheckpoint, timestamp_ms};

impl Catalog {
    /// Ensure migration schema tables exist.
    pub(crate) fn migration_initialize(&self) -> Result<(), MigrationError> {
        use crate::dialect::{ColumnDef, ColumnType};

        let migration_cols = vec![
            ColumnDef { name: "version".into(), col_type: ColumnType::Int64 },
            ColumnDef { name: "name".into(), col_type: ColumnType::Text },
            ColumnDef { name: "status".into(), col_type: ColumnType::Text },
            ColumnDef { name: "direction".into(), col_type: ColumnType::Text },
            ColumnDef { name: "checksum".into(), col_type: ColumnType::Text },
            ColumnDef { name: "execution_id".into(), col_type: ColumnType::Text },
            ColumnDef { name: "applied_at".into(), col_type: ColumnType::Int64 },
            ColumnDef { name: "duration_ms".into(), col_type: ColumnType::Int64 },
            ColumnDef { name: "error".into(), col_type: ColumnType::Text },
        ];
        let ddl = self.dialect.create_table("_DataflowMigration", &migration_cols);
        self.conn.execute(&ddl)
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        let lock_cols = vec![
            ColumnDef { name: "locked_by".into(), col_type: ColumnType::Text },
            ColumnDef { name: "locked_at".into(), col_type: ColumnType::Int64 },
            ColumnDef { name: "expires_at".into(), col_type: ColumnType::Int64 },
        ];
        let ddl = self.dialect.create_table("_DataflowMigrationLock", &lock_cols);
        self.conn.execute(&ddl)
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        Ok(())
    }

    /// Load applied migrations from the database.
    pub(crate) fn migration_load_applied(
        &self,
    ) -> Result<HashMap<u64, AppliedMigration>, MigrationError> {
        let query = self.dialect.select_all(
            "_DataflowMigration",
            &["version", "name", "status", "checksum", "execution_id", "applied_at", "duration_ms", "error"],
            Some("version"),
        );
        let result = self.conn.execute(&query)
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        let mut applied = HashMap::new();
        for row in &result.rows {
            let version = row[0].as_i64().unwrap_or(0) as u64;
            let name = row[1].as_str().unwrap_or("").to_string();
            let status = row[2].as_str().unwrap_or("applied").to_string();
            let checksum = row[3].as_str().unwrap_or("").to_string();
            let execution_id = row[4].as_str().unwrap_or("").to_string();
            let applied_at = row[5].as_i64().unwrap_or(0) as u64;
            let duration_ms = row[6].as_i64().unwrap_or(0) as u64;
            let error = row[7].as_str().unwrap_or("").to_string();

            applied.insert(
                version,
                AppliedMigration {
                    name,
                    status,
                    checksum,
                    execution_id,
                    applied_at,
                    duration_ms,
                    error,
                },
            );
        }

        Ok(applied)
    }

    /// Acquire migration lock (TTL-based).
    pub(crate) fn migration_acquire_lock(
        &self,
        lock_id: &str,
    ) -> Result<(), MigrationError> {
        const LOCK_TTL_MS: u64 = 10 * 60 * 1000;
        const LOCK_UUID: &str = "_migration_lock";
        let now = timestamp_ms();

        // Check existing lock
        let query = self.dialect.select_by_uuids(
            "_DataflowMigrationLock",
            &["locked_by", "locked_at", "expires_at"],
        );
        let result = self.conn
            .execute_with_params(
                &query,
                &[QueryParam::new(
                    "uuids",
                    CypherValue::List(vec![CypherValue::String(LOCK_UUID.to_string())]),
                )],
            )
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        if let Some(row) = result.rows.first() {
            let locked_by = row[0].as_str().unwrap_or("unknown").to_string();
            let locked_at = row[1].as_i64().unwrap_or(0) as u64;
            let expires_at = row[2].as_i64().unwrap_or(0) as u64;

            if now < expires_at {
                return Err(MigrationError::Locked {
                    by: locked_by,
                    since: locked_at,
                });
            }
            // Delete expired lock
            let del_query = self.dialect.batch_delete("_DataflowMigrationLock");
            self.conn
                .execute_with_params(
                    &del_query,
                    &[QueryParam::new(
                        "uuids",
                        CypherValue::List(vec![CypherValue::String(LOCK_UUID.to_string())]),
                    )],
                )
                .map_err(|e| MigrationError::DbError(e.to_string()))?;
        }

        // Create new lock
        let insert_query = self.dialect.batch_upsert(
            "_DataflowMigrationLock",
            &["_uuid", "locked_by", "locked_at", "expires_at"],
        );
        let mut item = std::collections::BTreeMap::new();
        item.insert("_uuid".to_string(), CypherValue::String(LOCK_UUID.to_string()));
        item.insert("locked_by".to_string(), CypherValue::String(lock_id.to_string()));
        item.insert("locked_at".to_string(), CypherValue::Int(now as i64));
        item.insert("expires_at".to_string(), CypherValue::Int((now + LOCK_TTL_MS) as i64));
        self.conn
            .execute_with_params(
                &insert_query,
                &[QueryParam::new(
                    "items",
                    CypherValue::List(vec![CypherValue::Map(item)]),
                )],
            )
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        Ok(())
    }

    /// Release migration lock.
    pub(crate) fn migration_release_lock(&self) -> Result<(), MigrationError> {
        const LOCK_UUID: &str = "_migration_lock";
        let query = self.dialect.batch_delete("_DataflowMigrationLock");
        self.conn
            .execute_with_params(
                &query,
                &[QueryParam::new(
                    "uuids",
                    CypherValue::List(vec![CypherValue::String(LOCK_UUID.to_string())]),
                )],
            )
            .map_err(|e| MigrationError::DbError(e.to_string()))?;
        Ok(())
    }

    /// Record a migration apply/rollback result.
    pub(crate) fn migration_record(
        &self,
        file: &MigrationFile,
        status: &str,
        direction: &str,
        execution_id: &str,
        duration_ms: u64,
        error: &str,
    ) -> Result<(), MigrationError> {
        let uuid = format!("migration-{:03}", file.version);
        let now = timestamp_ms();

        let query = self.dialect.batch_upsert(
            "_DataflowMigration",
            &["_uuid", "version", "name", "status", "direction", "checksum",
              "execution_id", "applied_at", "duration_ms", "error"],
        );
        let mut item = std::collections::BTreeMap::new();
        item.insert("_uuid".to_string(), CypherValue::String(uuid));
        item.insert("version".to_string(), CypherValue::Int(file.version as i64));
        item.insert("name".to_string(), CypherValue::String(file.name.clone()));
        item.insert("status".to_string(), CypherValue::String(status.to_string()));
        item.insert("direction".to_string(), CypherValue::String(direction.to_string()));
        item.insert("checksum".to_string(), CypherValue::String(file.checksum.clone()));
        item.insert("execution_id".to_string(), CypherValue::String(execution_id.to_string()));
        item.insert("applied_at".to_string(), CypherValue::Int(now as i64));
        item.insert("duration_ms".to_string(), CypherValue::Int(duration_ms as i64));
        item.insert("error".to_string(), CypherValue::String(error.to_string()));

        self.conn
            .execute_with_params(
                &query,
                &[QueryParam::new(
                    "items",
                    CypherValue::List(vec![CypherValue::Map(item)]),
                )],
            )
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        Ok(())
    }

    /// Update migration status only (used when file is missing on rollback).
    pub(crate) fn migration_update_status(
        &self,
        version: u64,
        status: &str,
    ) -> Result<(), MigrationError> {
        let uuid = format!("migration-{:03}", version);
        let query = self.dialect.batch_update_fields("_DataflowMigration", &["status"]);
        let mut item = std::collections::BTreeMap::new();
        item.insert("_uuid".to_string(), CypherValue::String(uuid));
        item.insert("status".to_string(), CypherValue::String(status.to_string()));

        self.conn
            .execute_with_params(
                &query,
                &[QueryParam::new(
                    "items",
                    CypherValue::List(vec![CypherValue::Map(item)]),
                )],
            )
            .map_err(|e| MigrationError::DbError(e.to_string()))?;
        Ok(())
    }

    /// Execute a migration graph with checkpoint support.
    pub(crate) fn migration_execute_graph(
        &self,
        graph: &mut DataflowGraph,
        execution_id: &str,
    ) -> Result<(), MigrationError> {
        let mut services = ServiceRegistry::new();
        services.register("conn", self.conn.clone());

        let checkpoint_store = CypherCheckpointStore::new(self.conn.clone());
        checkpoint_store
            .initialize()
            .map_err(|e| MigrationError::DbError(e))?;

        let runtime = DataflowRuntime::with_services(100, services);
        runtime
            .execute_with_checkpoint(graph, &checkpoint_store, execution_id)
            .map_err(|e| MigrationError::ExecutionError {
                version: 0,
                name: String::new(),
                detail: e,
            })?;
        Ok(())
    }

    /// Rollback a migration: undo nodes in reverse topological order,
    /// then re-enqueue restored entities and auto-drain.
    pub(crate) fn migration_rollback_graph(
        &mut self,
        graph: &mut DataflowGraph,
        checkpoint: &ExecutionCheckpoint,
    ) -> Result<(), MigrationError> {
        let order = graph
            .topological_sort()
            .map_err(|e| MigrationError::GraphError {
                version: 0,
                name: String::new(),
                detail: e,
            })?;
        let reversed: Vec<String> = order.into_iter().rev().collect();

        let mut services = ServiceRegistry::new();
        services.register("conn", self.conn.clone());
        let _services = Arc::new(services);

        for node_name in &reversed {
            let node_idx = graph
                .nodes
                .iter()
                .position(|n| n.name() == node_name)
                .ok_or_else(|| MigrationError::GraphError {
                    version: 0,
                    name: String::new(),
                    detail: format!("node '{}' not found in graph", node_name),
                })?;
            let node = &mut graph.nodes[node_idx];

            let undo_ctx = checkpoint
                .nodes
                .get(node_name.as_str())
                .and_then(|nc| nc.undo_context.clone());

            if let Some(ref ctx_val) = undo_ctx {
                let boxed_ctx: Box<dyn std::any::Any + Send> = Box::new(ctx_val.clone());
                node.undo(boxed_ctx)
                    .map_err(|e| MigrationError::ExecutionError {
                        version: 0,
                        name: String::new(),
                        detail: format!("undo of node '{}' failed: {e}", node_name),
                    })?;

                // After DeleteRecordNode undo, re-enqueue restored entities for re-ingestion
                if node.node_type() == "DeleteRecordNode" {
                    self.enqueue_restored_entities(ctx_val);
                }
            }
        }

        // Auto-drain: rebuild chunks, embeddings, FTS for restored entities
        if self.has_pending() {
            let _ = self.drain();
        }

        Ok(())
    }

    /// Extract restored entities from DeleteRecordNode undo context and enqueue
    /// them as creates so drain() will rebuild chunks/embeddings/FTS.
    ///
    /// The undo context is `{ "EntityName": [{ _uuid, field1, ... }, ...] }`.
    /// Entities are already restored in DB by undo() — we just need to re-run
    /// the ingestion pipeline (chunk, embed, FTS index).
    fn enqueue_restored_entities(&mut self, undo_ctx: &serde_json::Value) {
        let groups = match undo_ctx.as_object() {
            Some(g) => g,
            None => return,
        };
        for (entity_name, items) in groups {
            let arr = match items.as_array() {
                Some(a) => a,
                None => continue,
            };
            for item in arr {
                let props = match item.as_object() {
                    Some(p) => p,
                    None => continue,
                };
                let mut data = BTreeMap::new();
                for (k, v) in props {
                    data.insert(k.clone(), json_to_cypher_value(v));
                }
                // Build EntityRecord with the existing _uuid (entity already in DB).
                // Resolve the ref immediately — no downstream node needs to wait.
                let (entity_ref, resolver) = crate::refs::EntityRef::new(entity_name);
                if let Some(uuid) = data.get("_uuid").and_then(|v| v.as_str()) {
                    resolver.resolve(uuid.to_string());
                }
                self.pending.entities.push(EntityRecord {
                    entity_name: entity_name.clone(),
                    data,
                    entity_ref,
                    resolver: None, // already resolved above
                });
            }
        }
    }
}

/// Convert serde_json::Value to CypherValue (for re-enqueuing restored entities).
fn json_to_cypher_value(v: &serde_json::Value) -> CypherValue {
    match v {
        serde_json::Value::String(s) => CypherValue::String(s.clone()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                CypherValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                CypherValue::Float(f)
            } else {
                CypherValue::Null
            }
        }
        serde_json::Value::Bool(b) => CypherValue::Bool(*b),
        serde_json::Value::Null => CypherValue::Null,
        serde_json::Value::Array(arr) => {
            CypherValue::List(arr.iter().map(json_to_cypher_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let map = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_cypher_value(v)))
                .collect();
            CypherValue::Map(map)
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use crate::connection::MockConnection;
    use crate::embedder::MockEmbedder;

    // ── test config ────────────────────────────────────────────────────

    fn make_test_config() -> CatalogConfig {
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            FieldDef {
                field_type: FieldType::Text,
                title_for: Some("main".to_string()),
                content_for: None,

                boost: Some(2.0),
                default_value: None,
            },
        );
        fields.insert(
            "body".to_string(),
            FieldDef {
                field_type: FieldType::Text,
                title_for: None,
                content_for: Some(vec!["main".to_string()]),

                boost: None,
                default_value: None,
            },
        );
        fields.insert(
            "page_count".to_string(),
            FieldDef {
                field_type: FieldType::Int64,
                title_for: None,
                content_for: None,

                boost: None,
                default_value: None,
            },
        );

        let mut entities = HashMap::new();
        entities.insert(
            "Document".to_string(),
            EntityDef {
                fields,
                hashsafe: Some(vec!["title".to_string()]),
            },
        );

        let mut relations = HashMap::new();
        relations.insert(
            "REFERENCES".to_string(),
            RelationDef {
                from: "Document".to_string(),
                to: "Document".to_string(),
                properties: None,
            },
        );

        let mut knowledge_bases = HashMap::new();
        knowledge_bases.insert("main".to_string(), KBConfig::default());

        CatalogConfig {
            name: Some("test-catalog".to_string()),
            entities,
            relations,
            knowledge_bases,
            embedding_dim: 384,
            ..Default::default()
        }
    }

    fn make_catalog() -> Catalog {
        Catalog::new(
            Box::new(MockConnection::new()),
            Box::new(MockEmbedder::new(384)),
            make_test_config(),
        )
    }

    fn make_doc_data(title: &str, body: &str) -> BTreeMap<String, CypherValue> {
        let mut data = BTreeMap::new();
        data.insert("title".to_string(), CypherValue::String(title.to_string()));
        data.insert("body".to_string(), CypherValue::String(body.to_string()));
        data.insert("page_count".to_string(), CypherValue::Int(42));
        data
    }

    /// Number of chunks produced by the Chunker for a given body text.
    #[allow(dead_code)] // conservé : utilitaire de test/diagnostic
    fn count_chunks(body: &str) -> usize {
        let chunker = Chunker::new(ChunkerConfig::default());
        chunker.chunk(body).len()
    }

    /// Records enqueued at create() time:
    /// 1 EntityRecord(entity) + 1 EntityRecord({KB}_Index) + 1 RelationRecord(_IN_) + 1 AggregateRecord.
    fn ops_enqueued_per_create(_body: &str) -> usize {
        4
    }

    /// Total records processed after drain():
    /// 2 inserts (entity + index) + 1 link (_IN_) + 1 aggregate.
    fn ops_per_create(_body: &str) -> usize {
        4
    }

    // ── lifecycle ──────────────────────────────────────────────────────

    #[test]
    fn new_catalog() {
        let catalog = make_catalog();
        assert!(!catalog.initialized);
        assert!(catalog.kb_metadata.is_empty());
    }

    #[test]
    fn initialize_success() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();
        assert!(catalog.initialized);
        assert_eq!(catalog.kb_metadata.len(), 1);
        assert!(catalog.kb_metadata.contains_key("main"));
    }

    #[test]
    fn initialize_validates_schema() {
        // Config with contentFor but no titleFor → invalid
        let mut fields = HashMap::new();
        fields.insert(
            "body".to_string(),
            FieldDef {
                field_type: FieldType::Text,
                title_for: None,
                content_for: Some(vec!["orphan_kb".to_string()]),

                boost: None,
                default_value: None,
            },
        );
        let config = CatalogConfig {
            entities: [(
                "Doc".to_string(),
                EntityDef {
                    fields,
                    hashsafe: None,
                },
            )]
            .into(),
            ..Default::default()
        };

        let mut catalog = Catalog::new(
            Box::new(MockConnection::new()),
            Box::new(MockEmbedder::new(384)),
            config,
        );
        let err = catalog.initialize().unwrap_err();
        assert!(
            matches!(err, CatalogError::ValidationFailed(_)),
            "expected ValidationFailed, got {err:?}"
        );
    }

    // ── not initialized ────────────────────────────────────────────────

    #[test]
    fn create_before_init_errors() {
        let mut catalog = make_catalog();
        let err = catalog.create("Document", BTreeMap::new()).unwrap_err();
        assert!(matches!(err, CatalogError::NotInitialized));
    }

    #[test]
    fn link_before_init_errors() {
        let mut catalog = make_catalog();
        let err = catalog
            .link("REFERENCES", "a", "b", BTreeMap::new())
            .unwrap_err();
        assert!(matches!(err, CatalogError::NotInitialized));
    }

    #[test]
    fn get_before_init_errors() {
        let catalog = make_catalog();
        let err = catalog.get("Document", "uuid").unwrap_err();
        assert!(matches!(err, CatalogError::NotInitialized));
    }

    // ── create ─────────────────────────────────────────────────────────

    #[test]
    fn create_returns_pending_ref() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let data = make_doc_data("Hello", "World");
        let entity_ref = catalog.create("Document", data).unwrap();

        assert_eq!(entity_ref.entity(), "Document");
        assert!(!entity_ref.is_ready()); // pending until drain
    }

    #[test]
    fn create_unknown_entity_errors() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let err = catalog.create("Ghost", BTreeMap::new()).unwrap_err();
        assert!(matches!(err, CatalogError::UnknownEntity(ref s) if s == "Ghost"));
    }

    #[test]
    fn create_enqueues_insert_and_embed() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let body = "Body text";
        let data = make_doc_data("Title", body);
        catalog.create("Document", data).unwrap();

        let stats = catalog.drain_stats();
        let expected = ops_enqueued_per_create(body);
        assert_eq!(stats.total_queued, expected);
        assert_eq!(stats.pending, expected);
    }

    #[test]
    fn create_hashsafe_deterministic() {
        let mut c1 = make_catalog();
        let mut c2 = make_catalog();
        c1.initialize().unwrap();
        c2.initialize().unwrap();

        let data1 = make_doc_data("Same Title", "Different body 1");
        let data2 = make_doc_data("Same Title", "Different body 2");

        let ref1 = c1.create("Document", data1).unwrap();
        let ref2 = c2.create("Document", data2).unwrap();

        // Drain both to resolve refs
        c1.drain();
        c2.drain();

        // Same hashsafe field (title) → same UUID
        assert_eq!(ref1.uuid().unwrap(), ref2.uuid().unwrap());
    }

    // ── link ───────────────────────────────────────────────────────────

    #[test]
    fn link_returns_pending_ref() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let rel_ref = catalog
            .link("REFERENCES", "uuid-a", "uuid-b", BTreeMap::new())
            .unwrap();

        assert_eq!(rel_ref.relation(), "REFERENCES");
        assert!(!rel_ref.is_ready());
    }

    #[test]
    fn link_unknown_relation_errors() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let err = catalog
            .link("GHOST_REL", "a", "b", BTreeMap::new())
            .unwrap_err();
        assert!(matches!(err, CatalogError::UnknownRelation(ref s) if s == "GHOST_REL"));
    }

    // ── drain ──────────────────────────────────────────────────────────

    #[test]
    fn drain_resolves_inserts() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let body = "Content here";
        let data = make_doc_data("Test Doc", body);
        let entity_ref = catalog.create("Document", data).unwrap();

        assert!(!entity_ref.is_ready());

        let result = catalog.drain();
        assert_eq!(result.processed, ops_per_create(body));
        assert_eq!(result.failed, 0);

        assert!(entity_ref.is_ready());
        // UUID should be a hashsafe UUID (deterministic from title)
        let uuid = entity_ref.uuid().unwrap();
        assert_eq!(uuid.len(), 36); // UUID format
    }

    #[test]
    fn drain_resolves_links() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let body_a = "Body A";
        let body_b = "Body B";
        let data1 = make_doc_data("Doc A", body_a);
        let data2 = make_doc_data("Doc B", body_b);
        let ref1 = catalog.create("Document", data1).unwrap();
        let ref2 = catalog.create("Document", data2).unwrap();

        let rel_ref = catalog
            .link(
                "REFERENCES",
                ref1.clone(),
                ref2.clone(),
                BTreeMap::new(),
            )
            .unwrap();

        let result = catalog.drain();
        let expected = ops_per_create(body_a) + ops_per_create(body_b) + 1; // +1 user link
        assert_eq!(result.processed, expected);
        assert_eq!(result.failed, 0);

        assert!(ref1.is_ready());
        assert!(ref2.is_ready());
        assert!(rel_ref.is_ready());

        let resolved = rel_ref.resolved().unwrap();
        assert_eq!(resolved.from_uuid, ref1.uuid().unwrap());
        assert_eq!(resolved.to_uuid, ref2.uuid().unwrap());
    }

    #[test]
    fn drain_empty_queue() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let result = catalog.drain();
        assert_eq!(result.processed, 0);
        assert_eq!(result.failed, 0);
    }

    // ── read operations (with mock) ────────────────────────────────────

    #[test]
    fn get_returns_none_empty_mock() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let result = catalog.get("Document", "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn exists_false_empty_mock() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let result = catalog.exists("Document", "nonexistent").unwrap();
        assert!(!result);
    }

    #[test]
    fn count_zero_empty_mock() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let result = catalog.count("Document").unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn get_many_empty_uuids() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let result = catalog.get_many("Document", &[]).unwrap();
        assert!(result.is_empty());
    }

    // ── update / delete (with mock) ────────────────────────────────────

    #[test]
    fn update_enqueues_sync() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        // update() is sync — just enqueues, no error for nonexistent uuid
        let data = make_doc_data("New Title", "New Body");
        catalog.update("Document", "nonexistent", data).unwrap();
        // Verify it was enqueued
        assert!(!catalog.pending.updates.is_empty());
    }

    #[test]
    fn delete_enqueues_sync() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        // delete() is sync — just enqueues
        catalog.delete("Document", "some-uuid").unwrap();
        assert_eq!(catalog.pending.deletes.len(), 1);
        assert_eq!(catalog.pending.deletes[0].uuid, "some-uuid");
        assert_eq!(catalog.pending.deletes[0].entity_name, "Document");
    }

    // ── schema queries ─────────────────────────────────────────────────

    #[test]
    fn get_kb_metadata_after_init() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let kb = catalog.get_kb_metadata("main").unwrap();
        assert_eq!(kb.name, "main");
        assert_eq!(kb.title.entity, "Document");
        assert_eq!(kb.title.field, "title");
        assert_eq!(kb.content.len(), 1);
        assert_eq!(kb.content[0].field, "body");
        assert_eq!(kb.signals, search::SearchSignals::HYBRID);
        assert_eq!(kb.keyword_weight, 0.3);

        assert!(catalog.get_kb_metadata("nonexistent").is_none());
    }

    #[test]
    fn get_kbs_for_entity_after_init() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let kbs = catalog.get_kbs_for_entity("Document");
        assert_eq!(kbs, vec!["main"]);

        let kbs = catalog.get_kbs_for_entity("Ghost");
        assert!(kbs.is_empty());
    }

    #[test]
    fn get_entity_def_and_relation_def() {
        let catalog = make_catalog();

        assert!(catalog.get_entity_def("Document").is_some());
        assert!(catalog.get_entity_def("Ghost").is_none());
        assert!(catalog.get_relation_def("REFERENCES").is_some());
        assert!(catalog.get_relation_def("GHOST").is_none());
    }

    // ── drain stats ────────────────────────────────────────────────────

    #[test]
    fn has_pending_and_stats() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        assert!(!catalog.has_pending());
        assert_eq!(catalog.drain_stats().total_queued, 0);

        let body = "B";
        catalog
            .create("Document", make_doc_data("A", body))
            .unwrap();

        let enqueued = ops_enqueued_per_create(body);
        assert!(catalog.has_pending());
        let stats = catalog.drain_stats();
        assert_eq!(stats.total_queued, enqueued);
        assert_eq!(stats.pending, enqueued);

        catalog.drain();

        assert!(!catalog.has_pending());
        let stats = catalog.drain_stats();
        assert_eq!(stats.total_processed, ops_per_create(body));
    }

    // ── flush_insertions ───────────────────────────────────────────────

    #[test]
    fn flush_insertions_only() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let body = "Flush test";
        let data = make_doc_data("Partial", body);
        let entity_ref = catalog.create("Document", data).unwrap();

        // Flush prio <= 1.0: 2 InsertOps (entity + {KB}_Index)
        let result = catalog.flush_insertions();
        assert_eq!(result.processed, 2);
        assert!(entity_ref.is_ready());

        // LinkOp (prio 2.0) + AggregateOp (prio 2.5) still pending
        assert!(catalog.has_pending());

        // Drain the rest: 1 link + 1 aggregate
        let result = catalog.drain();
        assert_eq!(result.processed, 2);
        assert!(!catalog.has_pending());
    }

    // ── filter_condition priority ─────────────────────────────────────

    #[test]
    fn search_filter_condition_takes_priority() {
        use crate::filter::{FilterCondition, FilterValue};
        use crate::search::SearchOptions;

        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        // Both filters and filter_condition set — filter_condition should win
        let mut opts = SearchOptions::default();
        opts.filters.insert(
            "page_count".to_string(),
            FilterValue::Direct(CypherValue::Int(10)),
        );
        opts.filter_condition = Some(FilterCondition::Field {
            key: "page_count".to_string(),
            value: FilterValue::Direct(CypherValue::Int(99)),
        });

        // With MockConnection, search returns empty but should not error
        let response = catalog.search("main", "test", opts).unwrap();
        assert!(response.results.is_empty());
    }

    // ── Phase A: Shadow records tests ─────────────────────────────────

    #[test]
    fn create_populates_pending_work() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let _ref = catalog.create("Document", make_doc_data("Hello", "World")).unwrap();

        let pw = catalog.pending_work();
        // 1 Document entity + 1 main_Index entity
        assert_eq!(pw.entities.len(), 2, "should have 2 entity records (Document + main_Index)");
        assert_eq!(pw.entities[0].entity_name, "Document");
        assert_eq!(pw.entities[1].entity_name, "main_Index");

        // 1 Document_IN_main relation
        assert_eq!(pw.relations.len(), 1, "should have 1 relation record (Document_IN_main)");
        assert_eq!(pw.relations[0].rel_name, "Document_IN_main");

        // 1 AggregateRecord
        assert_eq!(pw.aggregates.len(), 1, "should have 1 aggregate record");
        assert_eq!(pw.aggregates[0].kb_name, "main");
        assert_eq!(pw.aggregates[0].title_entity, "Document");
    }

    #[test]
    fn link_populates_pending_work() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let from_ref = catalog.create("Document", make_doc_data("A", "aaa")).unwrap();
        let to_ref = catalog.create("Document", make_doc_data("B", "bbb")).unwrap();

        let _rel = catalog.link("REFERENCES", from_ref, to_ref, BTreeMap::new()).unwrap();

        let pw = catalog.pending_work();
        // 2 creates × (1 Document + 1 main_Index) = 4 entities
        assert_eq!(pw.entities.len(), 4);
        // 2 creates × 1 Document_IN_main + 1 REFERENCES = 3 relations
        assert_eq!(pw.relations.len(), 3);
        assert_eq!(pw.relations[2].rel_name, "REFERENCES");
    }

    #[test]
    fn drain_clears_pending_work() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        catalog.create("Document", make_doc_data("Test", "body")).unwrap();
        assert!(!catalog.pending_work().is_empty());

        // drain() clears pending work
        let _ = catalog.drain();
        assert!(catalog.pending_work().is_empty(), "pending should be cleared after drain");
    }

    // ── checkpoint E2E ────────────────────────────────────────────────

    use crate::dataflow::checkpoint_store::MockCheckpointStore;
    use crate::dataflow::checkpoint::CheckpointExecutionStatus;

    fn make_catalog_with_mock_checkpoint() -> (Catalog, Arc<MockCheckpointStore>) {
        let mock_store = Arc::new(MockCheckpointStore::new());
        let mut catalog = make_catalog();
        catalog.set_checkpoint_store(mock_store.clone());
        (catalog, mock_store)
    }

    #[test]
    fn checkpoint_drain_marks_completed() {
        let (mut catalog, _store) = make_catalog_with_mock_checkpoint();
        catalog.initialize().unwrap();

        catalog.create("Document", make_doc_data("Test", "Body")).unwrap();
        let result = catalog.drain();
        assert_eq!(result.failed, 0);
        assert!(result.processed > 0);

        // Checkpoint should be marked completed (no pending checkpoints)
        let pending = catalog.check_pending_checkpoints().unwrap();
        assert!(pending.is_empty(), "checkpoint should be cleaned up after successful drain");
    }

    #[test]
    fn checkpoint_resume_nonexistent_returns_error() {
        let (mut catalog, _store) = make_catalog_with_mock_checkpoint();
        catalog.initialize().unwrap();

        let err = catalog.drain_resume("nonexistent-exec-id");
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(msg.contains("not found"), "expected 'not found' error, got: {msg}");
    }

    #[test]
    fn checkpoint_resume_already_completed_is_noop() {
        let (mut catalog, store) = make_catalog_with_mock_checkpoint();
        catalog.initialize().unwrap();

        // Do a normal drain to create a completed checkpoint
        catalog.create("Document", make_doc_data("Test", "Body")).unwrap();
        let result = catalog.drain();
        assert_eq!(result.failed, 0);

        // Find the completed execution_id
        // MockCheckpointStore keeps all executions; find the one with status Completed
        let exec_id = {
            let mut found = None;
            store.mutate_all(|execs| {
                for (id, cp) in execs.iter() {
                    if cp.status == CheckpointExecutionStatus::Completed {
                        found = Some(id.clone());
                    }
                }
            });
            found.expect("should have a completed execution")
        };

        // Resume on completed execution → should succeed as no-op
        let resume_result = catalog.drain_resume(&exec_id).unwrap();
        // execute_with_checkpoint returns Ok(DataflowOutput::empty()) for completed,
        // so drain_resume sees Ok → reports processed
        assert_eq!(resume_result.failed, 0);
    }

    #[test]
    fn checkpoint_check_pending_empty_initially() {
        let (mut catalog, _store) = make_catalog_with_mock_checkpoint();
        catalog.initialize().unwrap();

        let pending = catalog.check_pending_checkpoints().unwrap();
        assert!(pending.is_empty());
    }

    // ── register_entity ─────────────────────────────────────────────

    fn make_product_entity_config() -> crate::config::EntityConfig {
        let mut fields = HashMap::new();
        fields.insert("name".into(), crate::config::SimpleFieldDef {
            field_type: FieldType::String,
            is_title: true,
            is_content: false,
            ..Default::default()
        });
        fields.insert("description".into(), crate::config::SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        });
        fields.insert("details".into(), crate::config::SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        });
        fields.insert("price".into(), crate::config::SimpleFieldDef {
            field_type: FieldType::Double,
            is_title: false,
            is_content: false,
            ..Default::default()
        });
        crate::config::EntityConfig {
            fields,
            ..Default::default()
        }
    }

    #[test]
    fn register_entity_stores_config() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let config = make_product_entity_config();
        catalog.register_entity("Product", config).unwrap();

        assert!(catalog.is_simple_entity("Product"));
        assert!(!catalog.is_simple_entity("Unknown"));
    }

    #[test]
    fn register_entity_adds_to_catalog_entities() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let config = make_product_entity_config();
        catalog.register_entity("Product", config).unwrap();

        // Should be in config.entities too (for ChunkRecordNode compatibility)
        assert!(catalog.config.entities.contains_key("Product"));
        let entity_def = &catalog.config.entities["Product"];
        assert!(entity_def.fields.contains_key("name"));
        assert!(entity_def.fields.contains_key("description"));
        assert!(entity_def.fields.contains_key("price"));
    }

    #[test]
    fn register_entity_content_fields() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let config = make_product_entity_config();
        catalog.register_entity("Product", config).unwrap();

        let ec = catalog.entity_config("Product").unwrap();
        let content = ec.content_fields();
        assert_eq!(content, vec!["description", "details"]);
        assert_eq!(ec.title_field(), Some("name"));
    }

    #[test]
    fn register_entity_before_init_fails() {
        let mut catalog = make_catalog();
        let config = make_product_entity_config();
        let err = catalog.register_entity("Product", config).unwrap_err();
        assert!(matches!(err, CatalogError::NotInitialized));
    }

    #[test]
    fn ingest_entities_before_init_fails() {
        let mut catalog = make_catalog();
        let err = catalog.ingest_entities("Product", vec![]).unwrap_err();
        assert!(matches!(err, CatalogError::NotInitialized));
    }

    #[test]
    fn ingest_entities_unknown_entity_fails() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();
        let err = catalog.ingest_entities("Unknown", vec![BTreeMap::new()]).unwrap_err();
        assert!(matches!(err, CatalogError::UnknownEntity(_)));
    }

    #[test]
    fn ingest_entities_empty_records_ok() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let config = make_product_entity_config();
        catalog.register_entity("Product", config).unwrap();

        let result = catalog.ingest_entities("Product", vec![]).unwrap();
        assert_eq!(result.processed, 0);
    }

    #[test]
    fn ingest_entities_returns_processed_count() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let config = make_product_entity_config();
        catalog.register_entity("Product", config).unwrap();

        let mut data = BTreeMap::new();
        data.insert("name".into(), CypherValue::String("Red Shoes".into()));
        data.insert("description".into(), CypherValue::String("A nice pair of shoes.".into()));
        data.insert("details".into(), CypherValue::String("Made in Italy.".into()));
        data.insert("price".into(), CypherValue::Float(59.99));

        let result = catalog.ingest_entities("Product", vec![data]).unwrap();
        assert_eq!(result.processed, 1);
        assert_eq!(result.failed, 0);
    }

    // ── resolve_search_target ─────────────────────────────────────────

    #[test]
    fn resolve_search_target_kb() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let t = catalog.resolve_search_target("main").unwrap();
        assert_eq!(t.name, "main");
        assert_eq!(t.parent_table, "main_Index");
        assert_eq!(t.chunk_table, "main_Index_Chunk");
        assert_eq!(t.chunk_rel, "main_Index_HAS_CHUNK");
        assert!(t.chunk_rel_fwd);
        assert_eq!(t.bm25_fields, vec!["_title", "_content"]);
        assert!(t.has_source_refs);
        assert!(t.filter_indirection.is_some());
        let (title_ent, in_rel) = t.filter_indirection.unwrap();
        assert_eq!(title_ent, "Document");
        assert_eq!(in_rel, "Document_IN_main");
    }

    #[test]
    fn resolve_search_target_simple_entity() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let config = make_product_entity_config();
        catalog.register_entity("Product", config).unwrap();

        let t = catalog.resolve_search_target("Product").unwrap();
        assert_eq!(t.name, "Product");
        assert_eq!(t.parent_table, "Product");
        assert_eq!(t.chunk_table, "Product_Chunk");
        assert_eq!(t.chunk_rel, "Product_CHUNKED_FROM");
        assert!(!t.chunk_rel_fwd);
        // BM25 fields = content fields sorted
        assert_eq!(t.bm25_fields, vec!["description", "details"]);
        assert!(!t.has_source_refs);
        assert!(t.filter_indirection.is_none());
        // Enrich fields contain content + title + _content_hash
        assert!(t.enrich_fields.contains(&"description".to_string()));
        assert!(t.enrich_fields.contains(&"details".to_string()));
        assert!(t.enrich_fields.contains(&"name".to_string()));
        assert!(t.enrich_fields.contains(&"_content_hash".to_string()));
    }

    #[test]
    fn resolve_search_target_unknown_fails() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let err = catalog.resolve_search_target("Unknown").unwrap_err();
        assert!(matches!(err, CatalogError::UnknownKB(_)));
    }

    #[test]
    fn search_target_parent_to_chunk_match_kb() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let t = catalog.resolve_search_target("main").unwrap();
        let pattern = t.parent_to_chunk_match("n", "c");
        assert_eq!(
            pattern,
            "MATCH (n:main_Index)-[:main_Index_HAS_CHUNK]->(c:main_Index_Chunk)"
        );
    }

    #[test]
    fn search_target_parent_to_chunk_match_simple() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let config = make_product_entity_config();
        catalog.register_entity("Product", config).unwrap();

        let t = catalog.resolve_search_target("Product").unwrap();
        let pattern = t.parent_to_chunk_match("n", "c");
        // Simple: reversed direction
        assert_eq!(
            pattern,
            "MATCH (n:Product)<-[:Product_CHUNKED_FROM]-(c:Product_Chunk)"
        );
    }

    #[test]
    fn search_target_chunk_to_parent_match_kb() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let t = catalog.resolve_search_target("main").unwrap();
        let pattern = t.chunk_to_parent_match("p", "c");
        assert_eq!(
            pattern,
            "MATCH (p:main_Index)-[:main_Index_HAS_CHUNK]->(c)"
        );
    }

    #[test]
    fn search_target_chunk_to_parent_match_simple() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let config = make_product_entity_config();
        catalog.register_entity("Product", config).unwrap();

        let t = catalog.resolve_search_target("Product").unwrap();
        let pattern = t.chunk_to_parent_match("p", "c");
        // Simple: chunk→parent direction
        assert_eq!(
            pattern,
            "MATCH (c)-[:Product_CHUNKED_FROM]->(p:Product)"
        );
    }

    #[test]
    fn search_target_signals_default() {
        let mut catalog = make_catalog();
        catalog.initialize().unwrap();

        let config = make_product_entity_config();
        catalog.register_entity("Product", config).unwrap();

        let t = catalog.resolve_search_target("Product").unwrap();
        // Default = HYBRID (BM25 + Vector)
        assert!(t.default_signals.bm25());
        assert!(t.default_signals.vector());
    }
}

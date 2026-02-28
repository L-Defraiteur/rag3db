//! Catalog: CRUD facade assembling all rag3weaver pipeline components.
//!
//! The `Catalog` struct is the main entry point. It owns the database connection,
//! embedder, operation queue, and event bus. After `initialize()`, it provides
//! synchronous `create()`/`link()` methods that enqueue operations, and async
//! `drain()` to process them.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::config::{CatalogConfig, ChunkingConfig, EntityDef, FieldType, RelationDef, SearchMode};
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::embedder::{Embedder, SparseEmbedder};
use crate::events::{CatalogEvent, EventBus};
use crate::filter::{FilterCompiler, FilterCondition, FilterParser};
use crate::search;
use crate::hash::content_hash;
use crate::node_id_cache::{InternalNodeId, NodeIdCache};
use crate::ops::{CatalogOp, ChunkOp, EmbedOp, InsertOp, LinkOp, SparseEmbedOp, RefOrUuid};
use crate::queue::{FlushResult, OperationItem, OperationQueue, Processor, QueueSender, QueueStats};
use crate::refs::{EntityRef, RelationRef};
use crate::schema::{entity_has_chunks, generate_full_schema, generate_insert_cypher};
use crate::sparse_index::{SparseIndex, SparseVector};
use crate::chunker::{Chunker, ChunkerConfig};
use crate::uuid::{chunk_uuid, hashsafe_uuid};
use crate::validator::{validate_schema, KBFieldRef};

// ─── KBMetadata ────────────────────────────────────────────────────────────

/// Resolved metadata for a Knowledge Base, built at `Catalog::initialize()`.
#[derive(Debug, Clone)]
pub struct KBMetadata {
    pub name: String,
    pub title: KBFieldRef,
    pub content: Vec<KBFieldRef>,
    pub entities: HashSet<String>,
    pub search: SearchMode,
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

// ─── UpdateResult / DeleteResult ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Updated,
    Unchanged,
}

#[derive(Debug)]
pub struct UpdateResult {
    pub uuid: String,
    pub entity: String,
    pub status: UpdateStatus,
    pub reembedded: bool,
    pub chunks_created: usize,
    pub chunks_deleted: usize,
}

#[derive(Debug)]
pub struct DeleteResult {
    pub uuid: String,
    pub entity: String,
    pub chunks_deleted: usize,
    pub relations_deleted: usize,
}

// ─── Catalog ───────────────────────────────────────────────────────────────

pub struct Catalog {
    conn: Arc<dyn DbConnection>,
    embedder: Arc<dyn Embedder>,
    sparse_embedder: Option<Arc<dyn SparseEmbedder>>,
    config: CatalogConfig,
    queue: OperationQueue,
    event_bus: EventBus,
    kb_metadata: HashMap<String, KBMetadata>,
    initialized: bool,
    embedding_cache: HashMap<String, Vec<f32>>,
    /// Per-KB sparse inverted indexes (in-memory, rebuilt from DB at initialize).
    sparse_indexes: HashMap<String, SparseIndex>,
    /// Cache mapping entity UUIDs to rag3db internal node IDs.
    /// Shared with InsertProcessor (populated on INSERT via RETURN ID(n)).
    node_id_cache: Arc<RwLock<NodeIdCache>>,
    /// Cached chunkers keyed by config to avoid re-instantiation.
    chunker_cache: HashMap<ChunkerConfig, Chunker>,
}

impl Catalog {
    // ── Lifecycle ───────────────────────────────────────────────────────

    pub fn new(
        conn: Box<dyn DbConnection>,
        embedder: Box<dyn Embedder>,
        config: CatalogConfig,
    ) -> Self {
        let queue_config = crate::queue::FlushConfig {
            auto: config.flush.auto_flush,
            max_count: config.flush.max_count,
            completed_retention_ms: config.flush.completed_retention_ms,
        };
        Self {
            conn: Arc::from(conn),
            embedder: Arc::from(embedder),
            sparse_embedder: None,
            config,
            queue: OperationQueue::new(queue_config),
            event_bus: EventBus::new(64),
            kb_metadata: HashMap::new(),
            initialized: false,
            embedding_cache: HashMap::new(),
            sparse_indexes: HashMap::new(),
            node_id_cache: Arc::new(RwLock::new(NodeIdCache::new())),
            chunker_cache: HashMap::new(),
        }
    }

    /// Set the sparse embedder (optional). Must be called before `initialize()`.
    pub fn set_sparse_embedder(&mut self, embedder: Box<dyn SparseEmbedder>) {
        self.sparse_embedder = Some(Arc::from(embedder));
    }

    pub async fn initialize(&mut self) -> Result<(), CatalogError> {
        // 1. Validate schema
        let validation = validate_schema(&self.config);
        if !validation.valid {
            return Err(CatalogError::ValidationFailed(
                validation.errors.join("; "),
            ));
        }

        // 2. Generate DDL
        let schema = generate_full_schema(&self.config)
            .map_err(|e| CatalogError::SchemaError(e.to_string()))?;

        // 3. Execute DDL statements (tables first)
        for ddl in &schema.ddl {
            self.conn
                .execute(ddl)
                .await
                .map_err(|e| CatalogError::DbError(e.to_string()))?;
        }

        // 4. Execute index statements
        for idx in &schema.indexes {
            self.conn
                .execute(idx)
                .await
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
                    search: kb_config.search,
                    keyword_weight: kb_config.keyword_weight,
                    title_boost: kb_config.title_boost,
                    content_boost: kb_config.content_boost,
                    chunking: kb_config.chunking,
                },
            );
        }

        // 6. Register processors

        // Pre-warm chunker cache and register chunk processor
        self.warm_chunker_cache();
        self.queue.register_processor(
            "chunk",
            Box::new(ChunkProcessor {
                config: self.config.clone(),
                kb_metadata: self.kb_metadata.clone(),
                chunker_cache: std::mem::take(&mut self.chunker_cache),
                has_sparse: self.sparse_embedder.is_some(),
            }),
        );

        self.queue.register_processor(
            "insert",
            Box::new(InsertProcessor {
                conn: self.conn.clone(),
                node_id_cache: self.node_id_cache.clone(),
            }),
        );
        self.queue.register_processor(
            "link",
            Box::new(LinkProcessor {
                conn: self.conn.clone(),
            }),
        );
        self.queue.register_processor(
            "embed",
            Box::new(EmbedProcessor {
                conn: self.conn.clone(),
                embedder: self.embedder.clone(),
                embedding_dim: self.config.embedding_dim,
            }),
        );

        // Register sparse embed processor if sparse embedder is set
        if let Some(ref sparse_emb) = self.sparse_embedder {
            self.queue.register_processor(
                "sparse_embed",
                Box::new(SparseEmbedProcessor {
                    conn: self.conn.clone(),
                    sparse_embedder: sparse_emb.clone(),
                }),
            );
        }

        // Rebuild in-memory sparse indexes for KBs that have sparse=true
        for (kb_name, kb_config) in &self.config.knowledge_bases {
            if kb_config.sparse {
                let index = self.rebuild_sparse_index(kb_name).await?;
                self.sparse_indexes.insert(kb_name.clone(), index);
            }
        }

        self.initialized = true;
        Ok(())
    }

    // ── CRUD (synchronous, enqueue operations) ─────────────────────────

    pub fn create(
        &mut self,
        entity_name: &str,
        data: HashMap<String, CypherValue>,
    ) -> Result<EntityRef, CatalogError> {
        self.check_initialized()?;
        let entity_def = self.check_entity(entity_name)?;

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

        // Create InsertOp
        let insert_op = CatalogOp::Insert(InsertOp::new(
            entity_name.to_string(),
            full_data,
            resolver,
            entity_ref.clone(),
        ));

        // Enqueue InsertOp + ChunkOp (deferred chunking, processed at drain time).
        let mut ops: Vec<CatalogOp> = vec![insert_op];
        if let Some(chunk_op) = self.maybe_enqueue_chunk_op(entity_name, &uuid, &entity_ref, &data) {
            ops.push(chunk_op);
        }

        self.queue.enqueue_all(ops);
        Ok(entity_ref)
    }

    pub fn link(
        &mut self,
        rel_name: &str,
        from: impl Into<RefOrUuid>,
        to: impl Into<RefOrUuid>,
        properties: HashMap<String, CypherValue>,
    ) -> Result<RelationRef, CatalogError> {
        self.check_initialized()?;

        if !self.config.relations.contains_key(rel_name) {
            return Err(CatalogError::UnknownRelation(rel_name.to_string()));
        }

        let (relation_ref, resolver) = RelationRef::new(rel_name);

        let op = CatalogOp::Link(LinkOp::new(
            rel_name.to_string(),
            from.into(),
            to.into(),
            properties,
            resolver,
            relation_ref.clone(),
        ));

        self.queue.enqueue(op);
        Ok(relation_ref)
    }

    // ── Direct DB reads ────────────────────────────────────────────────

    pub async fn get(
        &self,
        entity_name: &str,
        uuid: &str,
    ) -> Result<Option<HashMap<String, CypherValue>>, CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;

        let cypher = format!(
            "MATCH (n:{entity_name} {{_uuid: $uuid}}) RETURN n"
        );
        let result = self
            .conn
            .execute_with_params(&cypher, &[QueryParam::new("uuid", uuid)])
            .await
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        if result.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.row_to_map(&result.columns, &result.rows[0])))
    }

    pub async fn get_many(
        &self,
        entity_name: &str,
        uuids: &[String],
    ) -> Result<Vec<HashMap<String, CypherValue>>, CatalogError> {
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
        let cypher = format!(
            "MATCH (n:{entity_name}) WHERE n._uuid IN $uuids RETURN n"
        );
        let result = self
            .conn
            .execute_with_params(
                &cypher,
                &[QueryParam {
                    name: "uuids".to_string(),
                    value: uuid_list,
                }],
            )
            .await
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        Ok(result
            .rows
            .iter()
            .map(|row| self.row_to_map(&result.columns, row))
            .collect())
    }

    pub async fn exists(
        &self,
        entity_name: &str,
        uuid: &str,
    ) -> Result<bool, CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;

        let cypher = format!(
            "MATCH (n:{entity_name} {{_uuid: $uuid}}) RETURN count(n) AS cnt"
        );
        let result = self
            .conn
            .execute_with_params(&cypher, &[QueryParam::new("uuid", uuid)])
            .await
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        let count = result
            .rows
            .get(0)
            .and_then(|r| r.get(0))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        Ok(count > 0)
    }

    pub async fn count(&self, entity_name: &str) -> Result<usize, CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;

        let cypher = format!("MATCH (n:{entity_name}) RETURN count(n) AS cnt");
        let result = self
            .conn
            .execute(&cypher)
            .await
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        let count = result
            .rows
            .get(0)
            .and_then(|r| r.get(0))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        Ok(count as usize)
    }

    // ── Update / Delete ────────────────────────────────────────────────

    pub async fn update(
        &mut self,
        entity_name: &str,
        uuid: &str,
        data: HashMap<String, CypherValue>,
    ) -> Result<UpdateResult, CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;

        // Compute new hash before the query
        let new_content = self.build_content_text(entity_name, &data);
        let new_hash = content_hash(&new_content);

        // Build SET clause — always include _content_hash
        let mut set_parts = Vec::new();
        let mut params: Vec<QueryParam> = vec![QueryParam::new("uuid", uuid)];

        let mut sorted_keys: Vec<&String> = data.keys().collect();
        sorted_keys.sort();
        for key in sorted_keys {
            set_parts.push(format!("n.{key} = ${key}"));
            params.push(QueryParam {
                name: key.clone(),
                value: data[key].clone(),
            });
        }
        set_parts.push("n._content_hash = $new_hash".to_string());
        params.push(QueryParam::new("new_hash", new_hash.clone()));

        // Single query: capture old hash via WITH, then SET + RETURN
        let cypher = format!(
            "MATCH (n:{entity_name} {{_uuid: $uuid}}) \
             WITH n, n._content_hash AS old_hash \
             SET {} \
             RETURN old_hash",
            set_parts.join(", ")
        );
        let result = self
            .conn
            .execute_with_params(&cypher, &params)
            .await
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        if result.is_empty() {
            return Err(CatalogError::NotFound {
                entity: entity_name.to_string(),
                uuid: uuid.to_string(),
            });
        }

        let old_hash = result.rows[0]
            .get(0)
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content_changed = old_hash != new_hash;

        // Re-chunk if content changed
        let mut reembedded = false;
        let mut chunks_deleted = 0usize;
        let mut chunks_created = 0usize;
        if content_changed {
            let entity_def = &self.config.entities[entity_name];

            // Delete old chunks
            if entity_has_chunks(entity_def) {
                let chunk_table = format!("{entity_name}_Chunk");
                let del_cypher = format!(
                    "MATCH (c:{chunk_table} {{_parent_uuid: $uuid}}) \
                     DETACH DELETE c RETURN count(c) AS cnt"
                );
                let del_result = self
                    .conn
                    .execute_with_params(&del_cypher, &[QueryParam::new("uuid", uuid)])
                    .await
                    .map_err(|e| CatalogError::DbError(e.to_string()))?;

                chunks_deleted = del_result
                    .rows
                    .get(0)
                    .and_then(|r| r.get(0))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as usize;
            }

            // Re-chunk and re-embed via ChunkOp (deferred, parallel at drain time)
            let (entity_ref, resolver) = EntityRef::new(entity_name);
            resolver.resolve(uuid.to_string());
            if let Some(chunk_op) = self.maybe_enqueue_chunk_op(entity_name, uuid, &entity_ref, &data) {
                reembedded = true;
                // chunks_created will be determined at drain time; estimate from chunker
                let chunker = Chunker::new(ChunkerConfig::default());
                let body = data.values()
                    .filter_map(|v| v.as_str())
                    .max_by_key(|s| s.len())
                    .unwrap_or("");
                chunks_created = chunker.chunk(body).len().max(1);
                self.queue.enqueue_all(vec![chunk_op]);
            }
        }

        self.event_bus.emit(CatalogEvent::EntityUpdated {
            entity: entity_name.to_string(),
            uuid: uuid.to_string(),
            reembedded,
            chunks_created,
            chunks_deleted,
        });

        Ok(UpdateResult {
            uuid: uuid.to_string(),
            entity: entity_name.to_string(),
            status: if content_changed {
                UpdateStatus::Updated
            } else {
                UpdateStatus::Unchanged
            },
            reembedded,
            chunks_created,
            chunks_deleted,
        })
    }

    pub async fn delete(
        &mut self,
        entity_name: &str,
        uuid: &str,
    ) -> Result<DeleteResult, CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;

        // Delete chunks then entity — kept as 2 queries because Kuzu does not
        // support DETACH DELETE inside a WITH chain (no FOREACH either).
        // Delete is a rare one-off operation so 2 round-trips is acceptable.
        let mut chunks_deleted = 0;
        if entity_has_chunks(&self.config.entities[entity_name]) {
            let chunk_table = format!("{entity_name}_Chunk");
            let cypher = format!(
                "MATCH (c:{chunk_table} {{_parent_uuid: $uuid}}) \
                 DETACH DELETE c RETURN count(c) AS cnt"
            );
            let result = self
                .conn
                .execute_with_params(&cypher, &[QueryParam::new("uuid", uuid)])
                .await
                .map_err(|e| CatalogError::DbError(e.to_string()))?;

            chunks_deleted = result
                .rows
                .get(0)
                .and_then(|r| r.get(0))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as usize;
        }

        // DETACH DELETE the entity
        let cypher = format!(
            "MATCH (n:{entity_name} {{_uuid: $uuid}}) DETACH DELETE n"
        );
        self.conn
            .execute_with_params(&cypher, &[QueryParam::new("uuid", uuid)])
            .await
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        // Remove from node ID cache
        if let Ok(mut cache) = self.node_id_cache.write() {
            cache.remove(uuid);
        }

        // Remove from in-memory sparse indexes
        for (kb_name, sparse_idx) in &mut self.sparse_indexes {
            let kb_sparse = self
                .config
                .knowledge_bases
                .get(kb_name)
                .map(|c| c.sparse)
                .unwrap_or(false);
            if kb_sparse {
                sparse_idx.remove(uuid);
            }
        }

        self.event_bus.emit(CatalogEvent::EntityDeleted {
            entity: entity_name.to_string(),
            uuid: uuid.to_string(),
            chunks_deleted,
        });

        Ok(DeleteResult {
            uuid: uuid.to_string(),
            entity: entity_name.to_string(),
            chunks_deleted,
            relations_deleted: 0,
        })
    }

    // ── Queue control ──────────────────────────────────────────────────

    pub async fn drain(&mut self) -> FlushResult {
        let result = self.queue.drain().await;

        // Refresh sparse indexes after drain (new sparse vectors may have been stored)
        for (kb_name, kb_config) in &self.config.knowledge_bases {
            if kb_config.sparse {
                if let Ok(index) = self.rebuild_sparse_index(kb_name).await {
                    self.sparse_indexes.insert(kb_name.clone(), index);
                }
            }
        }

        result
    }

    /// Parallel drain: inserts + embeds via rayon::join, then links sequentially.
    /// Uses block_on internally (no async runtime needed). WASM-only.
    #[cfg(feature = "wasm-emscripten")]
    pub fn drain_parallel(&mut self, pool: &rayon::ThreadPool) -> FlushResult {
        use crate::queue::{run_processor, queue_channel};

        let mut groups = self.queue.take_pending_grouped();
        if groups.is_empty() {
            return FlushResult::default();
        }

        let mut inserts = Vec::new();
        let mut embeds = Vec::new();
        let mut links = Vec::new();
        for (op_type, items) in groups.drain(..) {
            match op_type {
                "insert" => inserts = items,
                "embed" => embeds = items,
                "link" => links = items,
                _ => {}
            }
        }

        let insert_proc = self.queue.get_processor("insert");
        let embed_proc = self.queue.get_processor("embed");
        let link_proc = self.queue.get_processor("link");
        let (sender, _receiver) = queue_channel();

        // Phase 1: inserts + embeds in parallel
        let (r_insert, r_embed) = pool.install(|| {
            rayon::join(
                || run_processor(insert_proc.as_deref(), &mut inserts, &sender),
                || run_processor(embed_proc.as_deref(), &mut embeds, &sender),
            )
        });

        // Phase 2: links sequential (need resolved UUIDs from inserts)
        let r_link = run_processor(link_proc.as_deref(), &mut links, &sender);

        // Return non-completed items to the queue
        let mut all = inserts;
        all.extend(embeds);
        all.extend(links);
        self.queue.return_items(all);

        FlushResult {
            processed: r_insert.processed + r_embed.processed + r_link.processed,
            failed: r_insert.failed + r_embed.failed + r_link.failed,
            persisted: 0,
        }
    }

    pub async fn flush_insertions(&mut self) -> FlushResult {
        self.queue.flush_insertions().await
    }

    pub fn has_pending(&self) -> bool {
        self.queue.has_pending()
    }

    pub fn queue_stats(&self) -> QueueStats {
        self.queue.stats()
    }

    // ── Event bus ──────────────────────────────────────────────────────

    pub fn subscribe(&self) -> async_broadcast::Receiver<CatalogEvent> {
        self.event_bus.subscribe()
    }

    // ── Node ID cache ─────────────────────────────────────────────────

    /// Access the shared node ID cache (uuid → internal rag3db node ID).
    /// Populated automatically by InsertProcessor on each INSERT.
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

    pub async fn search(
        &mut self,
        kb_name: &str,
        query: &str,
        options: search::SearchOptions,
    ) -> Result<search::SearchResponse, CatalogError> {
        self.check_initialized()?;

        let kb = self
            .kb_metadata
            .get(kb_name)
            .ok_or_else(|| CatalogError::UnknownKB(kb_name.to_string()))?
            .clone();

        let pending_count = self.queue.stats().pending;

        // Consistency
        match options.consistency {
            search::Consistency::Strict => {
                self.queue.drain().await;
            }
            search::Consistency::Eventual => {
                if self.has_pending() {
                    self.flush_insertions().await;
                }
            }
            search::Consistency::Immediate => {}
        }

        let search_type = match kb.search {
            SearchMode::Semantic => search::SearchType::Semantic,
            SearchMode::Fulltext => search::SearchType::BM25Only,
            SearchMode::Hybrid => search::SearchType::Hybrid,
        };

        let search_limit = (options.limit + options.offset) * 2;
        let entity = kb.title.entity.clone();
        let keyword_weight = options.keyword_weight.unwrap_or(kb.keyword_weight);

        // Collect text fields for BM25 search (title + content fields for this entity)
        let bm25_fields: Vec<String> = {
            let mut fields = vec![];
            if kb.title.entity == entity {
                fields.push(kb.title.field.clone());
            }
            for c in &kb.content {
                if c.entity == entity {
                    fields.push(c.field.clone());
                }
            }
            fields
        };

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
            let mut parser = FilterParser::new(&self.config.relations);
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

        // For BM25 search: split filters into Tantivy-native and Kuzu-only
        let (tantivy_filters, allowed_ids) = if let Some(ref cond) = condition {
            let split = FilterCompiler::split(cond);

            // Tantivy-native part → FilterClause JSON
            let tf = split
                .tantivy
                .as_ref()
                .map(|t| FilterCompiler::to_tantivy_json(t));

            // Kuzu-only part → pre-resolve IDs via Cypher MATCH
            let aids = if let Some(ref kuzu_cond) = split.kuzu {
                let mut parser = FilterParser::new(&self.config.relations);
                let parsed = parser
                    .parse_condition(kuzu_cond, &entity, "n")
                    .map_err(|e| CatalogError::FilterError(e.to_string()))?;
                if !parsed.where_clauses.is_empty() {
                    let match_prefix = if parsed.match_clauses.is_empty() {
                        format!("MATCH (n:{entity})")
                    } else {
                        format!(
                            "MATCH (n:{entity}) {}",
                            parsed.match_clauses.join(" ")
                        )
                    };
                    let cypher = format!(
                        "{match_prefix} WHERE {} RETURN id(n)",
                        parsed.combine_where()
                    );
                    let result = if parsed.params.is_empty() {
                        self.conn
                            .execute(&cypher)
                            .await
                            .map_err(|e| CatalogError::DbError(e.to_string()))?
                    } else {
                        self.conn
                            .execute_with_params(&cypher, &parsed.params)
                            .await
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

            (tf, aids)
        } else {
            (None, None)
        };

        // Embed query
        let embedding =
            search::embed_query(self.embedder.as_ref(), query, &mut self.embedding_cache)
                .await?;

        // Run searches based on mode
        let (vector_results, bm25_results) = match search_type {
            search::SearchType::Hybrid => {
                let vr = search::search_vector(
                    self.conn.as_ref(),
                    &entity,
                    kb_name,
                    &embedding,
                    search_limit,
                    filter_where.as_deref(),
                    &filter_params,
                    filter_match.as_deref(),
                )
                .await?;
                let br = search::search_bm25(
                    self.conn.as_ref(),
                    &entity,
                    query,
                    &bm25_fields,
                    options.bm25_mode,
                    options.fuzzy_distance,
                    search_limit,
                    tantivy_filters.as_deref(),
                    allowed_ids.as_deref(),
                )
                .await?;
                (vr, br)
            }
            search::SearchType::Semantic => {
                let vr = search::search_vector(
                    self.conn.as_ref(),
                    &entity,
                    kb_name,
                    &embedding,
                    search_limit,
                    filter_where.as_deref(),
                    &filter_params,
                    filter_match.as_deref(),
                )
                .await?;
                (vr, vec![])
            }
            search::SearchType::BM25Only => {
                let br = search::search_bm25(
                    self.conn.as_ref(),
                    &entity,
                    query,
                    &bm25_fields,
                    options.bm25_mode,
                    options.fuzzy_distance,
                    search_limit,
                    tantivy_filters.as_deref(),
                    allowed_ids.as_deref(),
                )
                .await?;
                (vec![], br)
            }
        };

        let vector_count = vector_results.len();
        let bm25_count = bm25_results.len();

        // Sparse search (if KB has sparse enabled and we have a sparse embedder)
        let kb_config = self
            .config
            .knowledge_bases
            .get(kb_name)
            .cloned()
            .unwrap_or_default();
        let sparse_results = if kb_config.sparse {
            if let (Some(sparse_emb), Some(sparse_idx)) =
                (&self.sparse_embedder, self.sparse_indexes.get(kb_name))
            {
                let query_vecs = sparse_emb
                    .embed_sparse(&[query.to_string()])
                    .await
                    .map_err(|e| CatalogError::EmbedError(e.to_string()))?;
                if let Some(qv) = query_vecs.into_iter().next() {
                    search::search_sparse(sparse_idx, &qv, &entity, search_limit)
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else {
            vec![]
        };
        let sparse_count = sparse_results.len();
        let sparse_weight = options
            .sparse_weight
            .unwrap_or(kb_config.sparse_weight);

        let mut fused = search::fuse_results(
            &vector_results,
            &bm25_results,
            &sparse_results,
            options.hybrid_strategy,
            keyword_weight,
            sparse_weight,
            options.boost_factor,
            options.rrf_k,
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

        self.event_bus.emit(CatalogEvent::SearchCompleted {
            kb: kb_name.to_string(),
            results: fused.len(),
            duration_ms: 0,
        });

        Ok(search::SearchResponse {
            results: fused,
            meta: search::SearchMeta {
                query: query.to_string(),
                kb: kb_name.to_string(),
                search_type,
                consistency: options.consistency,
                partial: pending_count > 0
                    && options.consistency == search::Consistency::Immediate,
                pending_count,
                vector_count,
                bm25_count,
                sparse_count,
                fused_count,
                search_time_ms: 0,
            },
        })
    }

    pub async fn search_with_explore(
        &mut self,
        kb_name: &str,
        query: &str,
        options: search::ExploreOptions,
    ) -> Result<search::ExploreResult, CatalogError> {
        let response = self.search(kb_name, query, options.search).await?;

        let seed_nodes: Vec<search::GraphNode> = response
            .results
            .iter()
            .map(|r| search::GraphNode {
                uuid: r.uuid.clone(),
                entity: r.entity.clone().unwrap_or_default(),
                label: r.uuid.clone(),
                depth: 0,
                is_search_result: true,
                data: HashMap::new(),
            })
            .collect();

        let graph = search::explore_bfs(
            self.conn.as_ref(),
            seed_nodes,
            &options.outgoing_relations,
            &options.incoming_relations,
            options.depth,
            options.top_k,
        )
        .await?;

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

    /// Rebuild the in-memory sparse index for a KB by reading stored
    /// sparse vectors from the database.
    async fn rebuild_sparse_index(&self, kb_name: &str) -> Result<SparseIndex, CatalogError> {
        let mut index = SparseIndex::new();

        let kb_meta = match self.kb_metadata.get(kb_name) {
            Some(m) => m,
            None => return Ok(index), // no metadata yet → empty index
        };

        let indices_col = format!("{kb_name}_sparse_indices");
        let weights_col = format!("{kb_name}_sparse_weights");

        // Single UNION ALL query for all entity types
        let union_parts: Vec<String> = kb_meta
            .entities
            .iter()
            .map(|entity| {
                format!(
                    "MATCH (n:{entity}) \
                     WHERE n.{indices_col} IS NOT NULL \
                     RETURN n._uuid, n.{indices_col}, n.{weights_col}"
                )
            })
            .collect();

        if union_parts.is_empty() {
            return Ok(index);
        }

        let cypher = union_parts.join(" UNION ALL ");
        let result = self
            .conn
            .execute(&cypher)
            .await
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        {
            for row in &result.rows {
                let uuid = row.get(0).and_then(|v| v.as_str()).unwrap_or("");
                if uuid.is_empty() {
                    continue;
                }

                let indices: Vec<u32> = match row.get(1) {
                    Some(CypherValue::List(list)) => list
                        .iter()
                        .filter_map(|v| v.as_i64().map(|i| i as u32))
                        .collect(),
                    _ => continue,
                };

                let weights: Vec<f32> = match row.get(2) {
                    Some(CypherValue::List(list)) => list
                        .iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect(),
                    _ => continue,
                };

                if indices.len() == weights.len() && !indices.is_empty() {
                    let sv = SparseVector::new(indices, weights);
                    index.insert(uuid, &sv);
                }
            }
        }

        Ok(index)

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
        data: &HashMap<String, CypherValue>,
    ) -> String {
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

    /// Pre-warm the chunker cache for all KB chunking configs.
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
    }

    /// Check if entity has chunks and enqueue a ChunkOp if so.
    fn maybe_enqueue_chunk_op(
        &self,
        entity_name: &str,
        parent_uuid: &str,
        entity_ref: &EntityRef,
        data: &HashMap<String, CypherValue>,
    ) -> Option<CatalogOp> {
        let entity_def = self.config.entities.get(entity_name)?;
        if !entity_has_chunks(entity_def) {
            return None;
        }
        let kbs = self.get_kbs_for_entity(entity_name);
        if kbs.is_empty() {
            return None;
        }
        Some(CatalogOp::Chunk(ChunkOp {
            entity_name: entity_name.to_string(),
            parent_uuid: parent_uuid.to_string(),
            entity_ref: entity_ref.clone(),
            data: data.clone(),
        }))
    }

    fn build_embed_texts(
        &self,
        entity_name: &str,
        kb_name: &str,
        data: &HashMap<String, CypherValue>,
    ) -> Vec<String> {
        let kb = match self.kb_metadata.get(kb_name) {
            Some(kb) => kb,
            None => return vec![],
        };

        let mut texts = Vec::new();

        // Title field (if from this entity)
        if kb.title.entity == entity_name {
            if let Some(val) = data.get(&kb.title.field) {
                if let Some(s) = val.as_str() {
                    if !s.is_empty() {
                        texts.push(s.to_string());
                    }
                }
            }
        }

        // Content fields (if from this entity)
        for content_ref in &kb.content {
            if content_ref.entity == entity_name {
                if let Some(val) = data.get(&content_ref.field) {
                    if let Some(s) = val.as_str() {
                        if !s.is_empty() {
                            texts.push(s.to_string());
                        }
                    }
                }
            }
        }

        texts
    }

    fn row_to_map(
        &self,
        columns: &[String],
        row: &[CypherValue],
    ) -> HashMap<String, CypherValue> {
        let mut data = HashMap::new();
        for (i, col) in columns.iter().enumerate() {
            if i < row.len() {
                data.insert(col.clone(), row[i].clone());
            }
        }
        data
    }
}

// ─── compute_chunk_ops (standalone) ────────────────────────────────────────

/// Build InsertOps, LinkOps, EmbedOps, SparseEmbedOps for all chunks of an entity.
/// Standalone function (no &self) for use by ChunkProcessor in parallel via rayon.
fn compute_chunk_ops(
    entity_name: &str,
    parent_uuid: &str,
    entity_ref: &EntityRef,
    data: &HashMap<String, CypherValue>,
    config: &CatalogConfig,
    kb_metadata: &HashMap<String, KBMetadata>,
    chunker_cache: &HashMap<ChunkerConfig, Chunker>,
    has_sparse: bool,
) -> Vec<CatalogOp> {
    let entity_def = match config.entities.get(entity_name) {
        Some(def) => def,
        None => return vec![],
    };
    if !entity_has_chunks(entity_def) {
        return vec![];
    }

    let kb_names: Vec<&String> = kb_metadata
        .iter()
        .filter(|(_, kb)| kb.entities.contains(entity_name))
        .map(|(name, _)| name)
        .collect();
    if kb_names.is_empty() {
        return vec![];
    }

    let mut ops = Vec::new();

    for kb_name in &kb_names {
        let kb_meta = match kb_metadata.get(*kb_name) {
            Some(kb) => kb,
            None => continue,
        };
        let kb_config = config.knowledge_bases.get(*kb_name);

        let title_text: Option<String> = if kb_meta.title.entity == entity_name {
            data.get(&kb_meta.title.field)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        } else {
            None
        };

        let kb_sparse = kb_config.map(|c| c.sparse).unwrap_or(false) && has_sparse;
        let chunking = &kb_meta.chunking;

        let chunker_key = ChunkerConfig {
            max_size: chunking.max_size,
            overlap: chunking.overlap,
            strategy: chunking.strategy.clone(),
        };
        let chunker = chunker_cache
            .get(&chunker_key)
            .expect("chunker must be pre-warmed in cache");

        for content_ref in &kb_meta.content {
            if content_ref.entity != entity_name {
                continue;
            }
            let field_name = &content_ref.field;
            let field_def = match entity_def.fields.get(field_name) {
                Some(fd) => fd,
                None => continue,
            };
            if !field_def.chunked {
                continue;
            }

            let field_text = match data.get(field_name).and_then(|v| v.as_str()) {
                Some(s) if !s.is_empty() => s,
                _ => continue,
            };

            let chunks = chunker.chunk(field_text);
            if chunks.is_empty() {
                continue;
            }

            let chunk_table = format!("{entity_name}_Chunk");
            let rel_name = format!("{entity_name}_HAS_CHUNK");

            for chunk in &chunks {
                let c_uuid = chunk_uuid(parent_uuid, field_name, chunk.index);

                let embed_text = match &title_text {
                    Some(title) => format!("{title}\n---\n{}", chunk.text),
                    None => chunk.text.clone(),
                };

                let mut chunk_data = HashMap::new();
                chunk_data.insert("_uuid".to_string(), CypherValue::String(c_uuid.clone()));
                chunk_data.insert("_parent_uuid".to_string(), CypherValue::String(parent_uuid.to_string()));
                chunk_data.insert("_parent_field".to_string(), CypherValue::String(field_name.clone()));
                chunk_data.insert("_kb_name".to_string(), CypherValue::String(kb_name.to_string()));
                chunk_data.insert("_text".to_string(), CypherValue::String(chunk.text.clone()));
                chunk_data.insert("_text_hash".to_string(), CypherValue::String(content_hash(&chunk.text)));
                chunk_data.insert("_index".to_string(), CypherValue::Int(chunk.index as i64));
                chunk_data.insert("_start_char".to_string(), CypherValue::Int(chunk.start_byte as i64));
                chunk_data.insert("_end_char".to_string(), CypherValue::Int(chunk.end_byte as i64));
                chunk_data.insert("_start_line".to_string(), CypherValue::Int(chunk.start_line as i64));
                chunk_data.insert("_end_line".to_string(), CypherValue::Int(chunk.end_line as i64));
                chunk_data.insert("_core_start_char".to_string(), CypherValue::Int(chunk.core_start_byte as i64));
                chunk_data.insert("_core_end_char".to_string(), CypherValue::Int(chunk.core_end_byte as i64));
                chunk_data.insert("_core_start_line".to_string(), CypherValue::Int(chunk.core_start_line as i64));
                chunk_data.insert("_core_end_line".to_string(), CypherValue::Int(chunk.core_end_line as i64));

                let (chunk_ref, chunk_resolver) = EntityRef::new(&chunk_table);

                ops.push(CatalogOp::Insert(InsertOp::new(
                    chunk_table.clone(),
                    chunk_data,
                    chunk_resolver,
                    chunk_ref.clone(),
                )));

                let (link_ref, link_resolver) = RelationRef::new(&rel_name);
                ops.push(CatalogOp::Link(LinkOp::new(
                    rel_name.clone(),
                    RefOrUuid::Ref(entity_ref.clone()),
                    RefOrUuid::Uuid(c_uuid.clone()),
                    HashMap::new(),
                    link_resolver,
                    link_ref,
                )));

                ops.push(CatalogOp::Embed(EmbedOp {
                    entity_ref: chunk_ref.clone(),
                    kb_name: kb_name.to_string(),
                    texts: vec![embed_text.clone()],
                }));

                if kb_sparse {
                    ops.push(CatalogOp::SparseEmbed(SparseEmbedOp {
                        entity_ref: chunk_ref,
                        kb_name: kb_name.to_string(),
                        texts: vec![embed_text],
                    }));
                }
            }
        }
    }

    ops
}

// ─── ChunkProcessor ───────────────────────────────────────────────────────

/// Processes ChunkOps by running the chunker (in parallel via rayon) and
/// emitting downstream InsertOp/LinkOp/EmbedOp/SparseEmbedOp via the sender.
struct ChunkProcessor {
    config: CatalogConfig,
    kb_metadata: HashMap<String, KBMetadata>,
    chunker_cache: HashMap<ChunkerConfig, Chunker>,
    has_sparse: bool,
}

#[async_trait]
impl Processor for ChunkProcessor {
    async fn process(&self, items: &mut [OperationItem], sender: &QueueSender) -> Result<(), String> {
        use rayon::prelude::*;

        // Collect ChunkOps from items
        let chunk_ops: Vec<&ChunkOp> = items
            .iter()
            .filter_map(|item| match &item.op {
                CatalogOp::Chunk(c) => Some(c),
                _ => None,
            })
            .collect();

        // Parallel chunking via rayon
        let all_downstream: Vec<Vec<CatalogOp>> = chunk_ops
            .par_iter()
            .map(|chunk_op| {
                compute_chunk_ops(
                    &chunk_op.entity_name,
                    &chunk_op.parent_uuid,
                    &chunk_op.entity_ref,
                    &chunk_op.data,
                    &self.config,
                    &self.kb_metadata,
                    &self.chunker_cache,
                    self.has_sparse,
                )
            })
            .collect();

        // Emit all downstream ops
        for ops in all_downstream {
            sender.emit_all(ops);
        }

        Ok(())
    }
}

// ─── InsertProcessor ───────────────────────────────────────────────────────

struct InsertProcessor {
    conn: Arc<dyn DbConnection>,
    node_id_cache: Arc<RwLock<NodeIdCache>>,
}

#[async_trait]
impl Processor for InsertProcessor {
    async fn process(&self, items: &mut [OperationItem], _sender: &QueueSender) -> Result<(), String> {
        for item in items.iter_mut() {
            if let CatalogOp::Insert(ref mut insert) = item.op {
                let mut columns: Vec<&str> =
                    insert.data.keys().map(|k| k.as_str()).collect();
                columns.sort();

                // Append RETURN ID(n) to capture the internal node ID
                let base_cypher = generate_insert_cypher(&insert.entity_name, &columns);
                let cypher = base_cypher.replace(
                    &format!("(:{}", insert.entity_name),
                    &format!("(n:{}", insert.entity_name),
                ) + " RETURN ID(n)";

                let params: Vec<QueryParam> = columns
                    .iter()
                    .map(|&col| QueryParam {
                        name: col.to_string(),
                        value: insert
                            .data
                            .get(col)
                            .cloned()
                            .unwrap_or(CypherValue::Null),
                    })
                    .collect();

                let result = self
                    .conn
                    .execute_with_params(&cypher, &params)
                    .await
                    .map_err(|e| e.to_string())?;

                // Resolve the entity ref with the generated UUID
                let uuid = insert
                    .data
                    .get("_uuid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Cache the internal node ID if returned (format: "table_id:offset")
                if let Some(id_val) = result.rows.first().and_then(|row| row.first()) {
                    if let Some(id_str) = id_val.as_str() {
                        if let Some(node_id) = InternalNodeId::parse(id_str) {
                            if let Ok(mut cache) = self.node_id_cache.write() {
                                cache.insert(&uuid, node_id);
                            }
                        }
                    }
                }

                if let Some(resolver) = insert.take_resolver() {
                    resolver.resolve(uuid);
                }
            }
        }
        Ok(())
    }
}

// ─── LinkProcessor ─────────────────────────────────────────────────────────

struct LinkProcessor {
    conn: Arc<dyn DbConnection>,
}

#[async_trait]
impl Processor for LinkProcessor {
    async fn process(&self, items: &mut [OperationItem], _sender: &QueueSender) -> Result<(), String> {
        for item in items.iter_mut() {
            if let CatalogOp::Link(ref mut link) = item.op {
                let from_uuid = link
                    .from
                    .resolve()
                    .await
                    .map_err(|e| format!("link from resolution failed: {e}"))?;
                let to_uuid = link
                    .to
                    .resolve()
                    .await
                    .map_err(|e| format!("link to resolution failed: {e}"))?;

                // Build MATCH...CREATE cypher
                let mut cypher = format!(
                    "MATCH (a {{_uuid: $from_uuid}}), (b {{_uuid: $to_uuid}}) \
                     CREATE (a)-[:{}", link.rel_name
                );
                let mut params = vec![
                    QueryParam::new("from_uuid", from_uuid.clone()),
                    QueryParam::new("to_uuid", to_uuid.clone()),
                ];

                if !link.properties.is_empty() {
                    let mut prop_keys: Vec<&String> = link.properties.keys().collect();
                    prop_keys.sort();
                    let prop_strs: Vec<String> =
                        prop_keys.iter().map(|k| format!("{k}: ${k}")).collect();
                    cypher.push_str(&format!(" {{{}}}", prop_strs.join(", ")));
                    for key in prop_keys {
                        params.push(QueryParam {
                            name: key.clone(),
                            value: link.properties[key].clone(),
                        });
                    }
                }
                cypher.push_str("]->(b)");

                self.conn
                    .execute_with_params(&cypher, &params)
                    .await
                    .map_err(|e| e.to_string())?;

                if let Some(resolver) = link.take_resolver() {
                    resolver.resolve(from_uuid, to_uuid);
                }
            }
        }
        Ok(())
    }
}

// ─── EmbedProcessor ────────────────────────────────────────────────────────

struct EmbedProcessor {
    conn: Arc<dyn DbConnection>,
    embedder: Arc<dyn Embedder>,
    embedding_dim: usize,
}

#[async_trait]
impl Processor for EmbedProcessor {
    async fn process(&self, items: &mut [OperationItem], _sender: &QueueSender) -> Result<(), String> {
        // Phase 1: Wait for all entity refs to resolve, collect embed work
        struct EmbedWork {
            uuid: String,
            text: String,
            entity_name: String,
            embedding_col: String,
        }

        let mut works = Vec::new();

        for item in items.iter_mut() {
            if let CatalogOp::Embed(ref mut embed) = item.op {
                let uuid = embed
                    .entity_ref
                    .ready()
                    .await
                    .map_err(|e| format!("embed ref resolution failed: {e}"))?;

                if embed.texts.is_empty() {
                    continue;
                }

                works.push(EmbedWork {
                    uuid,
                    text: embed.texts.join("\n"),
                    entity_name: embed.entity_ref.entity().to_string(),
                    embedding_col: format!("{}_embedding", embed.kb_name),
                });
            }
        }

        if works.is_empty() {
            return Ok(());
        }

        // Phase 2: Batch embed all texts in a single call
        let texts: Vec<String> = works.iter().map(|w| w.text.clone()).collect();
        let vectors = self
            .embedder
            .embed(&texts)
            .await
            .map_err(|e| format!("embedding failed: {e}"))?;

        if vectors.len() != works.len() {
            return Err(format!(
                "embedder returned {} vectors for {} texts",
                vectors.len(),
                works.len()
            ));
        }

        // Phase 3: Batch store embeddings via UNWIND (one query per entity+col group)
        let mut groups: HashMap<(&str, &str), Vec<(&EmbedWork, &Vec<f32>)>> = HashMap::new();
        for (work, vector) in works.iter().zip(vectors.iter()) {
            if vector.len() != self.embedding_dim {
                return Err(format!(
                    "embedding dimension mismatch: expected {}, got {}",
                    self.embedding_dim,
                    vector.len()
                ));
            }
            groups
                .entry((&work.entity_name, &work.embedding_col))
                .or_default()
                .push((work, vector));
        }

        for ((entity_name, col), group) in &groups {
            let items_param = CypherValue::List(
                group
                    .iter()
                    .map(|(work, vec)| {
                        let mut map = HashMap::new();
                        map.insert(
                            "uuid".to_string(),
                            CypherValue::String(work.uuid.clone()),
                        );
                        map.insert(
                            "emb".to_string(),
                            CypherValue::List(
                                vec.iter().map(|&f| CypherValue::Float(f as f64)).collect(),
                            ),
                        );
                        CypherValue::Map(map)
                    })
                    .collect(),
            );

            let cypher = format!(
                "UNWIND $items AS item \
                 MATCH (n:{entity_name} {{_uuid: item.uuid}}) \
                 SET n.{col} = item.emb"
            );

            self.conn
                .execute_with_params(
                    &cypher,
                    &[QueryParam {
                        name: "items".to_string(),
                        value: items_param,
                    }],
                )
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

// ─── SparseEmbedProcessor ──────────────────────────────────────────────────

struct SparseEmbedProcessor {
    conn: Arc<dyn DbConnection>,
    sparse_embedder: Arc<dyn SparseEmbedder>,
}

#[async_trait]
impl Processor for SparseEmbedProcessor {
    async fn process(&self, items: &mut [OperationItem], _sender: &QueueSender) -> Result<(), String> {
        struct SparseWork {
            uuid: String,
            text: String,
            entity_name: String,
            kb_name: String,
        }

        let mut works = Vec::new();

        for item in items.iter_mut() {
            if let CatalogOp::SparseEmbed(ref mut op) = item.op {
                let uuid = op
                    .entity_ref
                    .ready()
                    .await
                    .map_err(|e| format!("sparse embed ref resolution failed: {e}"))?;

                if op.texts.is_empty() {
                    continue;
                }

                works.push(SparseWork {
                    uuid,
                    text: op.texts.join("\n"),
                    entity_name: op.entity_ref.entity().to_string(),
                    kb_name: op.kb_name.clone(),
                });
            }
        }

        if works.is_empty() {
            return Ok(());
        }

        // Batch embed
        let texts: Vec<String> = works.iter().map(|w| w.text.clone()).collect();
        let sparse_vecs = self
            .sparse_embedder
            .embed_sparse(&texts)
            .await
            .map_err(|e| format!("sparse embedding failed: {e}"))?;

        if sparse_vecs.len() != works.len() {
            return Err(format!(
                "sparse embedder returned {} vectors for {} texts",
                sparse_vecs.len(),
                works.len()
            ));
        }

        // Batch store sparse vectors via UNWIND (one query per entity+kb group)
        let mut groups: HashMap<(&str, &str), Vec<(&SparseWork, &SparseVector)>> = HashMap::new();
        for (work, sv) in works.iter().zip(sparse_vecs.iter()) {
            groups
                .entry((&work.entity_name, &work.kb_name))
                .or_default()
                .push((work, sv));
        }

        for ((entity_name, kb_name), group) in &groups {
            let indices_col = format!("{kb_name}_sparse_indices");
            let weights_col = format!("{kb_name}_sparse_weights");

            let items_param = CypherValue::List(
                group
                    .iter()
                    .map(|(work, sv)| {
                        let mut map = HashMap::new();
                        map.insert(
                            "uuid".to_string(),
                            CypherValue::String(work.uuid.clone()),
                        );
                        map.insert(
                            "indices".to_string(),
                            CypherValue::List(
                                sv.indices
                                    .iter()
                                    .map(|&i| CypherValue::Int(i as i64))
                                    .collect(),
                            ),
                        );
                        map.insert(
                            "weights".to_string(),
                            CypherValue::List(
                                sv.values
                                    .iter()
                                    .map(|&f| CypherValue::Float(f as f64))
                                    .collect(),
                            ),
                        );
                        CypherValue::Map(map)
                    })
                    .collect(),
            );

            let cypher = format!(
                "UNWIND $items AS item \
                 MATCH (n:{entity_name} {{_uuid: item.uuid}}) \
                 SET n.{indices_col} = item.indices, n.{weights_col} = item.weights"
            );

            self.conn
                .execute_with_params(
                    &cypher,
                    &[QueryParam {
                        name: "items".to_string(),
                        value: items_param,
                    }],
                )
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(())
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
                chunked: false,
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
                chunked: true,
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
                chunked: false,
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

    fn make_doc_data(title: &str, body: &str) -> HashMap<String, CypherValue> {
        let mut data = HashMap::new();
        data.insert("title".to_string(), CypherValue::String(title.to_string()));
        data.insert("body".to_string(), CypherValue::String(body.to_string()));
        data.insert("page_count".to_string(), CypherValue::Int(42));
        data
    }

    /// Number of chunks produced by the Chunker for a given body text.
    fn count_chunks(body: &str) -> usize {
        let chunker = Chunker::new(ChunkerConfig::default());
        chunker.chunk(body).len()
    }

    /// Ops enqueued at create() time: 1 entity insert + 1 ChunkOp.
    fn ops_enqueued_per_create(_body: &str) -> usize {
        2 // 1 InsertOp + 1 ChunkOp
    }

    /// Total ops processed after drain(): ChunkOp + entity insert + N*(chunk insert + link + embed).
    fn ops_per_create(body: &str) -> usize {
        let n = count_chunks(body);
        1 + 1 + n * 3 // 1 ChunkOp + 1 entity insert + N * (chunk insert + link + embed)
    }

    // ── lifecycle ──────────────────────────────────────────────────────

    #[test]
    fn new_catalog() {
        let catalog = make_catalog();
        assert!(!catalog.initialized);
        assert!(catalog.kb_metadata.is_empty());
    }

    #[tokio::test]
    async fn initialize_success() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();
        assert!(catalog.initialized);
        assert_eq!(catalog.kb_metadata.len(), 1);
        assert!(catalog.kb_metadata.contains_key("main"));
    }

    #[tokio::test]
    async fn initialize_validates_schema() {
        // Config with contentFor but no titleFor → invalid
        let mut fields = HashMap::new();
        fields.insert(
            "body".to_string(),
            FieldDef {
                field_type: FieldType::Text,
                title_for: None,
                content_for: Some(vec!["orphan_kb".to_string()]),
                chunked: false,
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
        let err = catalog.initialize().await.unwrap_err();
        assert!(
            matches!(err, CatalogError::ValidationFailed(_)),
            "expected ValidationFailed, got {err:?}"
        );
    }

    // ── not initialized ────────────────────────────────────────────────

    #[test]
    fn create_before_init_errors() {
        let mut catalog = make_catalog();
        let err = catalog.create("Document", HashMap::new()).unwrap_err();
        assert!(matches!(err, CatalogError::NotInitialized));
    }

    #[test]
    fn link_before_init_errors() {
        let mut catalog = make_catalog();
        let err = catalog
            .link("REFERENCES", "a", "b", HashMap::new())
            .unwrap_err();
        assert!(matches!(err, CatalogError::NotInitialized));
    }

    #[tokio::test]
    async fn get_before_init_errors() {
        let catalog = make_catalog();
        let err = catalog.get("Document", "uuid").await.unwrap_err();
        assert!(matches!(err, CatalogError::NotInitialized));
    }

    // ── create ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_returns_pending_ref() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let data = make_doc_data("Hello", "World");
        let entity_ref = catalog.create("Document", data).unwrap();

        assert_eq!(entity_ref.entity(), "Document");
        assert!(!entity_ref.is_ready()); // pending until drain
    }

    #[tokio::test]
    async fn create_unknown_entity_errors() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let err = catalog.create("Ghost", HashMap::new()).unwrap_err();
        assert!(matches!(err, CatalogError::UnknownEntity(ref s) if s == "Ghost"));
    }

    #[tokio::test]
    async fn create_enqueues_insert_and_embed() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let body = "Body text";
        let data = make_doc_data("Title", body);
        catalog.create("Document", data).unwrap();

        let stats = catalog.queue_stats();
        let expected = ops_enqueued_per_create(body);
        assert_eq!(stats.total_queued, expected);
        assert_eq!(stats.pending, expected);
    }

    #[tokio::test]
    async fn create_hashsafe_deterministic() {
        let mut c1 = make_catalog();
        let mut c2 = make_catalog();
        c1.initialize().await.unwrap();
        c2.initialize().await.unwrap();

        let data1 = make_doc_data("Same Title", "Different body 1");
        let data2 = make_doc_data("Same Title", "Different body 2");

        let ref1 = c1.create("Document", data1).unwrap();
        let ref2 = c2.create("Document", data2).unwrap();

        // Drain both to resolve refs
        c1.drain().await;
        c2.drain().await;

        // Same hashsafe field (title) → same UUID
        assert_eq!(ref1.uuid().unwrap(), ref2.uuid().unwrap());
    }

    // ── link ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn link_returns_pending_ref() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let rel_ref = catalog
            .link("REFERENCES", "uuid-a", "uuid-b", HashMap::new())
            .unwrap();

        assert_eq!(rel_ref.relation(), "REFERENCES");
        assert!(!rel_ref.is_ready());
    }

    #[tokio::test]
    async fn link_unknown_relation_errors() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let err = catalog
            .link("GHOST_REL", "a", "b", HashMap::new())
            .unwrap_err();
        assert!(matches!(err, CatalogError::UnknownRelation(ref s) if s == "GHOST_REL"));
    }

    // ── drain ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn drain_resolves_inserts() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let body = "Content here";
        let data = make_doc_data("Test Doc", body);
        let entity_ref = catalog.create("Document", data).unwrap();

        assert!(!entity_ref.is_ready());

        let result = catalog.drain().await;
        assert_eq!(result.processed, ops_per_create(body));
        assert_eq!(result.failed, 0);

        assert!(entity_ref.is_ready());
        // UUID should be a hashsafe UUID (deterministic from title)
        let uuid = entity_ref.uuid().unwrap();
        assert_eq!(uuid.len(), 36); // UUID format
    }

    #[tokio::test]
    async fn drain_resolves_links() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

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
                HashMap::new(),
            )
            .unwrap();

        let result = catalog.drain().await;
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

    #[tokio::test]
    async fn drain_empty_queue() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let result = catalog.drain().await;
        assert_eq!(result.processed, 0);
        assert_eq!(result.failed, 0);
    }

    // ── read operations (with mock) ────────────────────────────────────

    #[tokio::test]
    async fn get_returns_none_empty_mock() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let result = catalog.get("Document", "nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn exists_false_empty_mock() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let result = catalog.exists("Document", "nonexistent").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn count_zero_empty_mock() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let result = catalog.count("Document").await.unwrap();
        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn get_many_empty_uuids() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let result = catalog.get_many("Document", &[]).await.unwrap();
        assert!(result.is_empty());
    }

    // ── update / delete (with mock) ────────────────────────────────────

    #[tokio::test]
    async fn update_not_found() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let data = make_doc_data("New Title", "New Body");
        let err = catalog
            .update("Document", "nonexistent", data)
            .await
            .unwrap_err();
        assert!(matches!(err, CatalogError::NotFound { .. }));
    }

    #[tokio::test]
    async fn delete_succeeds_with_mock() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        // MockConnection returns Ok(empty) for everything, including DELETE
        let result = catalog.delete("Document", "some-uuid").await.unwrap();
        assert_eq!(result.entity, "Document");
        assert_eq!(result.uuid, "some-uuid");
    }

    // ── schema queries ─────────────────────────────────────────────────

    #[tokio::test]
    async fn get_kb_metadata_after_init() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let kb = catalog.get_kb_metadata("main").unwrap();
        assert_eq!(kb.name, "main");
        assert_eq!(kb.title.entity, "Document");
        assert_eq!(kb.title.field, "title");
        assert_eq!(kb.content.len(), 1);
        assert_eq!(kb.content[0].field, "body");
        assert_eq!(kb.search, SearchMode::Hybrid);
        assert_eq!(kb.keyword_weight, 0.3);

        assert!(catalog.get_kb_metadata("nonexistent").is_none());
    }

    #[tokio::test]
    async fn get_kbs_for_entity_after_init() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let kbs = catalog.get_kbs_for_entity("Document");
        assert_eq!(kbs, vec!["main"]);

        let kbs = catalog.get_kbs_for_entity("Ghost");
        assert!(kbs.is_empty());
    }

    #[tokio::test]
    async fn get_entity_def_and_relation_def() {
        let catalog = make_catalog();

        assert!(catalog.get_entity_def("Document").is_some());
        assert!(catalog.get_entity_def("Ghost").is_none());
        assert!(catalog.get_relation_def("REFERENCES").is_some());
        assert!(catalog.get_relation_def("GHOST").is_none());
    }

    // ── queue stats ────────────────────────────────────────────────────

    #[tokio::test]
    async fn has_pending_and_stats() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        assert!(!catalog.has_pending());
        assert_eq!(catalog.queue_stats().total_queued, 0);

        let body = "B";
        catalog
            .create("Document", make_doc_data("A", body))
            .unwrap();

        let enqueued = ops_enqueued_per_create(body);
        assert!(catalog.has_pending());
        let stats = catalog.queue_stats();
        assert_eq!(stats.total_queued, enqueued);
        assert_eq!(stats.pending, enqueued);

        catalog.drain().await;

        assert!(!catalog.has_pending());
        let stats = catalog.queue_stats();
        assert_eq!(stats.total_processed, ops_per_create(body));
    }

    // ── flush_insertions ───────────────────────────────────────────────

    #[tokio::test]
    async fn flush_insertions_only() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let body = "Flush test";
        let data = make_doc_data("Partial", body);
        let entity_ref = catalog.create("Document", data).unwrap();

        let n_chunks = count_chunks(body);

        // Flush prio <= 1: ChunkOp (prio 0) → expands → then inserts (prio 1)
        // Processed: 1 ChunkOp + 1 entity insert + N chunk inserts
        let result = catalog.flush_insertions().await;
        assert_eq!(result.processed, 1 + 1 + n_chunks);
        assert!(entity_ref.is_ready());

        // Links (prio 2) + embeds (prio 3) still pending
        assert!(catalog.has_pending());

        // Drain the rest: N links (prio 2) + N embeds (prio 3)
        let result = catalog.drain().await;
        assert_eq!(result.processed, n_chunks * 2);
        assert!(!catalog.has_pending());
    }

    // ── filter_condition priority ─────────────────────────────────────

    #[tokio::test]
    async fn search_filter_condition_takes_priority() {
        use crate::filter::{FilterCondition, FilterOp, FilterValue};
        use crate::search::SearchOptions;

        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

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
        let response = catalog.search("main", "test", opts).await.unwrap();
        assert!(response.results.is_empty());
    }
}

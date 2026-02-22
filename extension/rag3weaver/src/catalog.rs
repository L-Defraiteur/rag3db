//! Catalog: CRUD facade assembling all rag3weaver pipeline components.
//!
//! The `Catalog` struct is the main entry point. It owns the database connection,
//! embedder, operation queue, and event bus. After `initialize()`, it provides
//! synchronous `create()`/`link()` methods that enqueue operations, and async
//! `drain()` to process them.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;

use crate::config::{CatalogConfig, ChunkingConfig, EntityDef, FieldType, RelationDef, SearchMode};
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::embedder::Embedder;
use crate::events::{CatalogEvent, EventBus};
use crate::search;
use crate::hash::content_hash;
use crate::ops::{CatalogOp, EmbedOp, InsertOp, LinkOp, RefOrUuid};
use crate::queue::{FlushResult, OperationItem, OperationQueue, Processor, QueueStats};
use crate::refs::{EntityRef, RelationRef};
use crate::schema::{entity_has_chunks, generate_full_schema, generate_insert_cypher};
use crate::uuid::hashsafe_uuid;
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
    config: CatalogConfig,
    queue: OperationQueue,
    event_bus: EventBus,
    kb_metadata: HashMap<String, KBMetadata>,
    initialized: bool,
    embedding_cache: HashMap<String, Vec<f32>>,
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
            config,
            queue: OperationQueue::new(queue_config),
            event_bus: EventBus::new(64),
            kb_metadata: HashMap::new(),
            initialized: false,
            embedding_cache: HashMap::new(),
        }
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
        self.queue.register_processor(
            "insert",
            Box::new(InsertProcessor {
                conn: self.conn.clone(),
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
        full_data.insert("_uuid".to_string(), CypherValue::String(uuid));
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

        // Create EmbedOps for each KB this entity belongs to
        let mut ops: Vec<CatalogOp> = vec![insert_op];
        let kb_names: Vec<String> = self
            .get_kbs_for_entity(entity_name)
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        for kb_name in &kb_names {
            let texts = self.build_embed_texts(entity_name, kb_name, &data);
            if !texts.is_empty() {
                ops.push(CatalogOp::Embed(EmbedOp {
                    entity_ref: entity_ref.clone(),
                    kb_name: kb_name.clone(),
                    texts,
                }));
            }
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

        // Check entity exists and get current hash
        let cypher = format!(
            "MATCH (n:{entity_name} {{_uuid: $uuid}}) RETURN n._content_hash AS hash"
        );
        let result = self
            .conn
            .execute_with_params(&cypher, &[QueryParam::new("uuid", uuid)])
            .await
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        if result.is_empty() {
            return Err(CatalogError::NotFound {
                entity: entity_name.to_string(),
                uuid: uuid.to_string(),
            });
        }

        let new_content = self.build_content_text(entity_name, &data);
        let new_hash = content_hash(&new_content);
        let old_hash = result.rows[0]
            .get(0)
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content_changed = old_hash != new_hash;

        // Build SET clause
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
        if content_changed {
            set_parts.push("n._content_hash = $new_hash".to_string());
            params.push(QueryParam::new("new_hash", new_hash));
        }

        if !set_parts.is_empty() {
            let cypher = format!(
                "MATCH (n:{entity_name} {{_uuid: $uuid}}) SET {}",
                set_parts.join(", ")
            );
            self.conn
                .execute_with_params(&cypher, &params)
                .await
                .map_err(|e| CatalogError::DbError(e.to_string()))?;
        }

        // Re-embed if content changed
        let mut reembedded = false;
        if content_changed {
            let kb_names: Vec<String> = self
                .get_kbs_for_entity(entity_name)
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            for kb_name in &kb_names {
                let texts = self.build_embed_texts(entity_name, kb_name, &data);
                if !texts.is_empty() {
                    let (entity_ref, resolver) = EntityRef::new(entity_name);
                    resolver.resolve(uuid.to_string());
                    self.queue.enqueue(CatalogOp::Embed(EmbedOp {
                        entity_ref,
                        kb_name: kb_name.clone(),
                        texts,
                    }));
                }
            }
            reembedded = true;
        }

        self.event_bus.emit(CatalogEvent::EntityUpdated {
            entity: entity_name.to_string(),
            uuid: uuid.to_string(),
            reembedded,
            chunks_created: 0,
            chunks_deleted: 0,
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
            chunks_created: 0,
            chunks_deleted: 0,
        })
    }

    pub async fn delete(
        &mut self,
        entity_name: &str,
        uuid: &str,
    ) -> Result<DeleteResult, CatalogError> {
        self.check_initialized()?;
        self.check_entity(entity_name)?;

        // Delete chunks if entity has chunked fields
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
        self.queue.drain().await
    }

    /// Parallel drain: inserts + embeds via rayon::join, then links sequentially.
    /// Uses block_on internally (no async runtime needed). WASM-only.
    #[cfg(feature = "wasm-emscripten")]
    pub fn drain_parallel(&mut self, pool: &rayon::ThreadPool) -> FlushResult {
        use crate::queue::run_processor;

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

        // Phase 1: inserts + embeds in parallel
        let (r_insert, r_embed) = pool.install(|| {
            rayon::join(
                || run_processor(insert_proc.as_deref(), &mut inserts),
                || run_processor(embed_proc.as_deref(), &mut embeds),
            )
        });

        // Phase 2: links sequential (need resolved UUIDs from inserts)
        let r_link = run_processor(link_proc.as_deref(), &mut links);

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
                )
                .await?;
                (vec![], br)
            }
        };

        let vector_count = vector_results.len();
        let bm25_count = bm25_results.len();

        let mut fused = search::fuse_results(
            &vector_results,
            &bm25_results,
            options.hybrid_strategy,
            keyword_weight,
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

// ─── InsertProcessor ───────────────────────────────────────────────────────

struct InsertProcessor {
    conn: Arc<dyn DbConnection>,
}

#[async_trait]
impl Processor for InsertProcessor {
    async fn process(&self, items: &mut [OperationItem]) -> Result<(), String> {
        for item in items.iter_mut() {
            if let CatalogOp::Insert(ref mut insert) = item.op {
                let mut columns: Vec<&str> =
                    insert.data.keys().map(|k| k.as_str()).collect();
                columns.sort();

                let cypher = generate_insert_cypher(&insert.entity_name, &columns);
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

                self.conn
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
    async fn process(&self, items: &mut [OperationItem]) -> Result<(), String> {
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
    async fn process(&self, items: &mut [OperationItem]) -> Result<(), String> {
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

        // Phase 3: Store each embedding on its entity node
        for (work, vector) in works.iter().zip(vectors.iter()) {
            if vector.len() != self.embedding_dim {
                return Err(format!(
                    "embedding dimension mismatch: expected {}, got {}",
                    self.embedding_dim,
                    vector.len()
                ));
            }

            let cypher = format!(
                "MATCH (n:{} {{_uuid: $uuid}}) SET n.{} = $embedding",
                work.entity_name, work.embedding_col
            );

            let embedding_value = CypherValue::List(
                vector
                    .iter()
                    .map(|&f| CypherValue::Float(f as f64))
                    .collect(),
            );
            let params = vec![
                QueryParam::new("uuid", work.uuid.clone()),
                QueryParam {
                    name: "embedding".to_string(),
                    value: embedding_value,
                },
            ];

            self.conn
                .execute_with_params(&cypher, &params)
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

        let data = make_doc_data("Title", "Body text");
        catalog.create("Document", data).unwrap();

        let stats = catalog.queue_stats();
        // 1 insert + 1 embed (Document has 1 KB "main")
        assert_eq!(stats.total_queued, 2);
        assert_eq!(stats.pending, 2);
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

        let data = make_doc_data("Test Doc", "Content here");
        let entity_ref = catalog.create("Document", data).unwrap();

        assert!(!entity_ref.is_ready());

        let result = catalog.drain().await;
        assert_eq!(result.processed, 2); // 1 insert + 1 embed
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

        let data1 = make_doc_data("Doc A", "Body A");
        let data2 = make_doc_data("Doc B", "Body B");
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
        // 2 inserts + 2 embeds + 1 link = 5
        assert_eq!(result.processed, 5);
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

        catalog
            .create("Document", make_doc_data("A", "B"))
            .unwrap();

        assert!(catalog.has_pending());
        let stats = catalog.queue_stats();
        assert_eq!(stats.total_queued, 2); // insert + embed
        assert_eq!(stats.pending, 2);

        catalog.drain().await;

        assert!(!catalog.has_pending());
        let stats = catalog.queue_stats();
        assert_eq!(stats.total_processed, 2);
    }

    // ── flush_insertions ───────────────────────────────────────────────

    #[tokio::test]
    async fn flush_insertions_only() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let data = make_doc_data("Partial", "Flush test");
        let entity_ref = catalog.create("Document", data).unwrap();

        // Flush only inserts (priority 1)
        let result = catalog.flush_insertions().await;
        assert_eq!(result.processed, 1); // only the insert
        assert!(entity_ref.is_ready());

        // Embed still pending
        assert!(catalog.has_pending());

        // Drain the rest
        let result = catalog.drain().await;
        assert_eq!(result.processed, 1); // the embed
        assert!(!catalog.has_pending());
    }
}

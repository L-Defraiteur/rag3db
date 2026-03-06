//! Catalog: CRUD facade assembling all rag3weaver pipeline components.
//!
//! The `Catalog` struct is the main entry point. It owns the database connection,
//! embedder, operation queue, and event bus. After `initialize()`, it provides
//! synchronous `create()`/`link()` methods that enqueue operations, and async
//! `drain()` to process them.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::config::{CatalogConfig, ChunkingConfig, EntityDef, FieldType, RelationDef};
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::embedder::{DualEmbedder, Embedder, SparseEmbedder};
use crate::events::{CatalogEvent, EventBus};
use crate::filter::{FilterCondition, FilterParser};
use crate::search;
use crate::hash::content_hash;
use crate::node_id_cache::NodeIdCache;
use crate::ops::{AggregateOp, CatalogOp, ChunkOp, DualEmbedOp, EmbedOp, InsertOp, LinkOp, SparseEmbedOp, RefOrUuid};
use crate::queue::{FlushResult, QueueStats};
use crate::refs::{EntityRef, RelationRef};
use crate::schema::{entity_has_chunks, generate_full_schema, resolve_entity_kbs};
use crate::chunker::{Chunker, ChunkerConfig};
use crate::uuid::{chunk_uuid, hashsafe_uuid};
use crate::validator::{validate_schema, KBFieldRef};
use crate::dataflow::graph::DataflowGraph;
use crate::dataflow::ingestion_nodes::{
    AggregateBatchNode, ChunkBatchNode, DualEmbedBatchNode, EmbedBatchNode,
    InsertBatchNode, LinkBatchNode, SparseEmbedBatchNode,
};
use crate::dataflow::runtime::DataflowRuntime;

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

/// Cumulative drain statistics (not reset on clear).
#[derive(Debug, Default)]
struct DrainStats {
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
    pending_ops: Vec<CatalogOp>,
    drain_stats: DrainStats,
    event_bus: EventBus,
    kb_metadata: HashMap<String, KBMetadata>,
    initialized: bool,
    embedding_cache: HashMap<String, Vec<f32>>,
    /// Cache mapping entity UUIDs to rag3db internal node IDs.
    /// Populated by InsertBatchNode on each INSERT via RETURN ID(n).
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
        Self {
            conn: Arc::from(conn),
            embedder: Arc::from(embedder),
            sparse_embedder: None,
            dual_embedder: None,
            config,
            pending_ops: Vec::new(),
            drain_stats: DrainStats::default(),
            event_bus: EventBus::new(64),
            kb_metadata: HashMap::new(),
            initialized: false,
            embedding_cache: HashMap::new(),
            node_id_cache: Arc::new(RwLock::new(NodeIdCache::new())),
            chunker_cache: HashMap::new(),
        }
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

        // Create sparse vector indexes via extension for KBs that have sparse=true.
        // Sparse embeddings live on {KB}_Index_Chunk (one index per KB).
        if self.sparse_embedder.is_some() || self.dual_embedder.is_some() {
            for (kb_name, kb_config) in &self.config.knowledge_bases {
                if kb_config.signals.sparse() {
                    let target = format!("{kb_name}_Index_Chunk");
                    let indices_col = format!("{kb_name}_sparse_indices");
                    let weights_col = format!("{kb_name}_sparse_weights");
                    let cypher = format!(
                        "CALL CREATE_SPARSE_VECTOR_INDEX('{target}', '{indices_col}', '{weights_col}')"
                    );
                    // Ignore errors (index may already exist)
                    let _ = self.conn.execute(&cypher).await;
                }
            }
        }

        self.initialized = true;
        Ok(())
    }

    // ── CRUD (synchronous, enqueue operations) ─────────────────────────

    pub fn create(
        &mut self,
        entity_name: &str,
        data: BTreeMap<String, CypherValue>,
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

        // Enqueue InsertOp for the entity + KB Index ops (if titleFor any KB).
        let mut ops: Vec<CatalogOp> = vec![insert_op];

        // For each KB where this entity has titleFor, create Index entry + AggregateOp.
        let entity_kbs = resolve_entity_kbs(entity_def);
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
            // Sentinel hash: empty string forces AggregateProcessor to always run on first drain.
            // The real hash is computed by AggregateProcessor after aggregating all content.
            index_data.insert("_content_hash".to_string(), CypherValue::String(String::new()));
            index_data.insert("_title".to_string(), CypherValue::String(title_text));
            index_data.insert("_content".to_string(), CypherValue::String(content_text));

            let (index_ref, index_resolver) = EntityRef::new(&index_table);
            ops.push(CatalogOp::Insert(InsertOp::new(
                index_table.clone(),
                index_data,
                index_resolver,
                index_ref.clone(),
            )));

            // LinkOp: {Entity}_IN_{KB}
            let in_rel_name = format!("{entity_name}_IN_{kb_name}");
            let (in_rel_ref, in_rel_resolver) = RelationRef::new(&in_rel_name);
            ops.push(CatalogOp::Link(LinkOp::new(
                in_rel_name,
                RefOrUuid::Ref(entity_ref.clone()),
                RefOrUuid::Ref(index_ref),
                BTreeMap::new(),
                in_rel_resolver,
                in_rel_ref,
            )));

            // AggregateOp (deferred: will rebuild _content + chunks at drain time)
            ops.push(CatalogOp::Aggregate(AggregateOp {
                index_entry_uuid: index_uuid,
                kb_name: kb_name.clone(),
                title_entity: entity_name.to_string(),
                source_uuid: uuid.clone(),
            }));
        }

        self.drain_stats.total_queued += ops.len();
        self.pending_ops.extend(ops);
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

        let op = CatalogOp::Link(LinkOp::new(
            rel_name.to_string(),
            from_ref.clone(),
            to_ref.clone(),
            properties,
            resolver,
            relation_ref.clone(),
        ));

        let mut ops: Vec<CatalogOp> = vec![op];

        // Incremental: if this relation connects a content entity to a title entity
        // for a KB, enqueue an AggregateOp so the title entity's index is rebuilt.
        // Only when UUIDs are already resolved (incremental case). In batch mode,
        // UUIDs are pending EntityRefs and create() already enqueued AggregateOps.
        for (kb_name, kb_meta) in &self.kb_metadata {
            let title_entity = &kb_meta.title.entity;
            // Check if one side is the title entity and the other contributes content
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
                ops.push(CatalogOp::Aggregate(AggregateOp {
                    index_entry_uuid: index_uuid,
                    kb_name: kb_name.clone(),
                    title_entity: title_entity.clone(),
                    source_uuid: t_uuid,
                }));
            }
        }

        self.drain_stats.total_queued += ops.len();
        self.pending_ops.extend(ops);
        Ok(relation_ref)
    }

    // ── Direct DB reads ────────────────────────────────────────────────

    pub async fn get(
        &self,
        entity_name: &str,
        uuid: &str,
    ) -> Result<Option<BTreeMap<String, CypherValue>>, CatalogError> {
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
        data: BTreeMap<String, CypherValue>,
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

        // Read old hash first (separate query — Kuzu SET+RETURN returns post-SET values)
        let old_hash_query = format!(
            "MATCH (n:{entity_name} {{_uuid: $uuid}}) RETURN n._content_hash"
        );
        let old_result = self
            .conn
            .execute_with_params(&old_hash_query, &[QueryParam::new("uuid", uuid)])
            .await
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        if old_result.is_empty() {
            return Err(CatalogError::NotFound {
                entity: entity_name.to_string(),
                uuid: uuid.to_string(),
            });
        }

        let old_hash = old_result.rows[0]
            .get(0)
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content_changed = old_hash != new_hash;

        // Apply the SET
        let set_cypher = format!(
            "MATCH (n:{entity_name} {{_uuid: $uuid}}) SET {}",
            set_parts.join(", ")
        );
        self.conn
            .execute_with_params(&set_cypher, &params)
            .await
            .map_err(|e| CatalogError::DbError(e.to_string()))?;

        // If content changed, enqueue AggregateOps for all KBs this entity contributes to.
        // The AggregateProcessor will handle deleting old chunks, re-chunking, and re-embedding.
        let mut reembedded = false;
        let chunks_deleted = 0usize;
        let chunks_created = 0usize;
        if content_changed {
            let entity_def = &self.config.entities[entity_name];
            let entity_kbs = resolve_entity_kbs(entity_def);

            let mut ops = Vec::new();
            for (kb_name, mapping) in &entity_kbs {
                if mapping.title_field.is_some() {
                    // This entity is the title entity for this KB → aggregate its index entry
                    let index_uuid = hashsafe_uuid(
                        &format!("{kb_name}_Index"),
                        &[entity_name, uuid],
                    );
                    ops.push(CatalogOp::Aggregate(AggregateOp {
                        index_entry_uuid: index_uuid,
                        kb_name: kb_name.clone(),
                        title_entity: entity_name.to_string(),
                        source_uuid: uuid.to_string(),
                    }));
                    reembedded = true;
                } else {
                    // contentFor-only: find linked title entities and re-aggregate them.
                    let kb_meta = match self.kb_metadata.get(kb_name.as_str()) {
                        Some(m) => m,
                        None => continue,
                    };
                    let title_entity = &kb_meta.title.entity;
                    if let Some((rel_name, title_is_from)) = self.find_relation_to_entity(title_entity, entity_name) {
                        let (match_pattern, return_field) = if title_is_from {
                            (format!("MATCH (t:{title_entity})-[:{rel_name}]->(e:{entity_name} {{_uuid: $uuid}})"), "t")
                        } else {
                            (format!("MATCH (e:{entity_name} {{_uuid: $uuid}})-[:{rel_name}]->(t:{title_entity})"), "t")
                        };
                        let query = format!("{match_pattern} RETURN {return_field}._uuid");
                        let title_results = self
                            .conn
                            .execute_with_params(&query, &[QueryParam::new("uuid", uuid)])
                            .await
                            .map_err(|e| CatalogError::DbError(e.to_string()))?;
                        for row in &title_results.rows {
                            if let Some(title_uuid) = row.first().and_then(|v| v.as_str()) {
                                let index_uuid = hashsafe_uuid(
                                    &format!("{kb_name}_Index"),
                                    &[title_entity, title_uuid],
                                );
                                ops.push(CatalogOp::Aggregate(AggregateOp {
                                    index_entry_uuid: index_uuid,
                                    kb_name: kb_name.clone(),
                                    title_entity: title_entity.clone(),
                                    source_uuid: title_uuid.to_string(),
                                }));
                                reembedded = true;
                            }
                        }
                    }
                }
            }
            if !ops.is_empty() {
                self.drain_stats.total_queued += ops.len();
        self.pending_ops.extend(ops);
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

        // Delete KB Index entries and their chunks for KBs where this entity has titleFor.
        // For contentFor-only KBs, delete SOURCED chunks and re-aggregate the title entities.
        let mut chunks_deleted = 0usize;
        let mut aggregate_ops: Vec<CatalogOp> = Vec::new();
        let entity_def = &self.config.entities[entity_name];
        let entity_kbs = resolve_entity_kbs(entity_def);
        for (kb_name, mapping) in &entity_kbs {
            if mapping.title_field.is_some() {
                // This entity is the title entity for this KB → delete its index entry + chunks
                let index_uuid = hashsafe_uuid(
                    &format!("{kb_name}_Index"),
                    &[entity_name, uuid],
                );
                // Delete chunks linked to this index entry
                let chunk_table = format!("{kb_name}_Index_Chunk");
                let del_chunks = format!(
                    "MATCH (c:{chunk_table} {{_parent_uuid: $idx_uuid}}) \
                     DETACH DELETE c RETURN count(c) AS cnt"
                );
                let result = self
                    .conn
                    .execute_with_params(&del_chunks, &[QueryParam::new("idx_uuid", index_uuid.clone())])
                    .await
                    .map_err(|e| CatalogError::DbError(e.to_string()))?;
                chunks_deleted += result
                    .rows
                    .get(0)
                    .and_then(|r| r.get(0))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as usize;

                // Delete the index entry itself
                let index_table = format!("{kb_name}_Index");
                let del_idx = format!(
                    "MATCH (idx:{index_table} {{_uuid: $idx_uuid}}) DETACH DELETE idx"
                );
                let _ = self
                    .conn
                    .execute_with_params(&del_idx, &[QueryParam::new("idx_uuid", index_uuid)])
                    .await;
            } else {
                // contentFor-only: delete SOURCED chunks from this entity, then re-aggregate
                // the linked title entities so their _content is rebuilt without this entity.
                let kb_meta = match self.kb_metadata.get(kb_name.as_str()) {
                    Some(m) => m,
                    None => continue,
                };
                let title_entity = &kb_meta.title.entity;

                // Delete chunks this entity SOURCED for this KB
                let sourced_rel = format!("{entity_name}_SOURCED_{kb_name}");
                let chunk_table = format!("{kb_name}_Index_Chunk");
                let del_sourced = format!(
                    "MATCH (e:{entity_name} {{_uuid: $uuid}})-[:{sourced_rel}]->(c:{chunk_table}) \
                     DETACH DELETE c RETURN count(c) AS cnt"
                );
                let result = self
                    .conn
                    .execute_with_params(&del_sourced, &[QueryParam::new("uuid", uuid)])
                    .await
                    .map_err(|e| CatalogError::DbError(e.to_string()))?;
                chunks_deleted += result
                    .rows
                    .get(0)
                    .and_then(|r| r.get(0))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as usize;

                // Find linked title entities → enqueue AggregateOps to rebuild their content
                if let Some((rel_name, title_is_from)) = self.find_relation_to_entity(title_entity, entity_name) {
                    let (match_pattern, return_field) = if title_is_from {
                        // title -[rel]-> content_entity
                        (format!("MATCH (t:{title_entity})-[:{rel_name}]->(e:{entity_name} {{_uuid: $uuid}})"), "t")
                    } else {
                        // content_entity -[rel]-> title
                        (format!("MATCH (e:{entity_name} {{_uuid: $uuid}})-[:{rel_name}]->(t:{title_entity})"), "t")
                    };
                    let query = format!("{match_pattern} RETURN {return_field}._uuid");
                    let title_results = self
                        .conn
                        .execute_with_params(&query, &[QueryParam::new("uuid", uuid)])
                        .await
                        .map_err(|e| CatalogError::DbError(e.to_string()))?;
                    for row in &title_results.rows {
                        if let Some(title_uuid) = row.first().and_then(|v| v.as_str()) {
                            let index_uuid = hashsafe_uuid(
                                &format!("{kb_name}_Index"),
                                &[title_entity, title_uuid],
                            );
                            aggregate_ops.push(CatalogOp::Aggregate(AggregateOp {
                                index_entry_uuid: index_uuid,
                                kb_name: kb_name.clone(),
                                title_entity: title_entity.clone(),
                                source_uuid: title_uuid.to_string(),
                            }));
                        }
                    }
                }
            }
        }

        if !aggregate_ops.is_empty() {
            self.drain_stats.total_queued += aggregate_ops.len();
            self.pending_ops.extend(aggregate_ops);
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

        // Sparse vector extension handles delete via hooks — no manual removal needed.

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

    /// Build a dataflow graph from all pending operations in the queue.
    ///
    /// Partitions ops by type, creates batch nodes, and wires them with
    /// edges that encode the dependency ordering (insert → link → aggregate → embed).
    /// Returns `(graph, op_count)` where `op_count` is the total number of ops taken.
    fn build_ingestion_graph(&mut self) -> (DataflowGraph, usize) {
        let ops = std::mem::take(&mut self.pending_ops);
        if ops.is_empty() {
            return (DataflowGraph::new(), 0);
        }

        let op_count = ops.len();
        let mut graph = DataflowGraph::new();

        // Partition ops by type
        let mut chunks: Vec<crate::ops::ChunkOp> = Vec::new();
        let mut inserts: Vec<InsertOp> = Vec::new();
        let mut links: Vec<LinkOp> = Vec::new();
        let mut aggregates: Vec<AggregateOp> = Vec::new();
        let mut embeds: Vec<EmbedOp> = Vec::new();
        let mut sparse_embeds: Vec<SparseEmbedOp> = Vec::new();
        let mut dual_embeds: Vec<DualEmbedOp> = Vec::new();

        for op in ops {
            match op {
                CatalogOp::Chunk(o) => chunks.push(o),
                CatalogOp::Insert(o) => inserts.push(o),
                CatalogOp::Link(o) => links.push(o),
                CatalogOp::Aggregate(o) => aggregates.push(o),
                CatalogOp::Embed(o) => embeds.push(o),
                CatalogOp::SparseEmbed(o) => sparse_embeds.push(o),
                CatalogOp::DualEmbed(o) => dual_embeds.push(o),
            }
        }

        let has_chunks = !chunks.is_empty();
        let has_inserts = !inserts.is_empty();
        let has_links = !links.is_empty();
        let has_aggregates = !aggregates.is_empty();

        // 1. ChunkBatchNode (DynamicNode, priority 0)
        if has_chunks {
            self.warm_chunker_cache();
            graph.add_dynamic_node(Box::new(ChunkBatchNode::new(
                self.config.clone(),
                self.kb_metadata.clone(),
                std::mem::take(&mut self.chunker_cache),
                self.sparse_embedder.is_some() || self.dual_embedder.is_some(),
                self.dual_embedder.is_some(),
                chunks,
                self.conn.clone(),
                self.node_id_cache.clone(),
                self.embedder.clone(),
                self.sparse_embedder.clone(),
                self.dual_embedder.clone(),
                self.config.embedding_dim,
            ))).unwrap();
        }

        // 2. InsertBatchNode (priority 1)
        if has_inserts {
            graph.add_node(Box::new(InsertBatchNode::new(
                "inserts".to_string(),
                inserts,
                self.conn.clone(),
                self.node_id_cache.clone(),
            ))).unwrap();
            if has_chunks {
                graph.connect("chunk_batch", "done", "inserts", "trigger").unwrap();
            }
        }

        // 3. LinkBatchNode (priority 2)
        if has_links {
            graph.add_node(Box::new(LinkBatchNode::new(
                "links".to_string(),
                links,
                self.conn.clone(),
            ))).unwrap();
            if has_inserts {
                graph.connect("inserts", "done", "links", "trigger").unwrap();
            } else if has_chunks {
                graph.connect("chunk_batch", "done", "links", "trigger").unwrap();
            }
        }

        // 4. AggregateBatchNode (DynamicNode, priority 2.5)
        if has_aggregates {
            self.warm_chunker_cache();
            graph.add_dynamic_node(Box::new(AggregateBatchNode::new(
                aggregates,
                self.conn.clone(),
                self.config.clone(),
                self.kb_metadata.clone(),
                std::mem::take(&mut self.chunker_cache),
                self.sparse_embedder.is_some() || self.dual_embedder.is_some(),
                self.dual_embedder.is_some(),
                self.node_id_cache.clone(),
                self.embedder.clone(),
                self.sparse_embedder.clone(),
                self.dual_embedder.clone(),
                self.config.embedding_dim,
            ))).unwrap();
            if has_links {
                graph.connect("links", "done", "aggregate_batch", "trigger").unwrap();
            } else if has_inserts {
                graph.connect("inserts", "done", "aggregate_batch", "trigger").unwrap();
            } else if has_chunks {
                graph.connect("chunk_batch", "done", "aggregate_batch", "trigger").unwrap();
            }
        }

        // 5. Embed nodes (priority 3) — depend on the last non-embed node
        let embed_trigger = if has_aggregates {
            Some("aggregate_batch")
        } else if has_links {
            Some("links")
        } else if has_inserts {
            Some("inserts")
        } else if has_chunks {
            Some("chunk_batch")
        } else {
            None
        };

        if !embeds.is_empty() {
            graph.add_node(Box::new(EmbedBatchNode::new(
                "embeds".to_string(),
                embeds,
                self.conn.clone(),
                self.embedder.clone(),
                self.config.embedding_dim,
            ))).unwrap();
            if let Some(trigger) = embed_trigger {
                graph.connect(trigger, "done", "embeds", "trigger").unwrap();
            }
        }

        if !sparse_embeds.is_empty() {
            if let Some(ref sparse_emb) = self.sparse_embedder {
                graph.add_node(Box::new(SparseEmbedBatchNode::new(
                    "sparse_embeds".to_string(),
                    sparse_embeds,
                    self.conn.clone(),
                    sparse_emb.clone(),
                ))).unwrap();
                if let Some(trigger) = embed_trigger {
                    graph.connect(trigger, "done", "sparse_embeds", "trigger").unwrap();
                }
            }
        }

        if !dual_embeds.is_empty() {
            if let Some(ref dual_emb) = self.dual_embedder {
                graph.add_node(Box::new(DualEmbedBatchNode::new(
                    "dual_embeds".to_string(),
                    dual_embeds,
                    self.conn.clone(),
                    dual_emb.clone(),
                    self.config.embedding_dim,
                    32,
                ))).unwrap();
                if let Some(trigger) = embed_trigger {
                    graph.connect(trigger, "done", "dual_embeds", "trigger").unwrap();
                }
            }
        }

        (graph, op_count)
    }

    /// Drain all pending operations via the dataflow runtime.
    pub async fn drain(&mut self) -> FlushResult {
        let (mut graph, op_count) = self.build_ingestion_graph();
        if graph.nodes.is_empty() {
            return FlushResult::default();
        }

        let node_count = graph.nodes.len();
        // max_iterations: generous bound — DynamicNodes may expand the graph
        let runtime = DataflowRuntime::new(node_count + 20);
        match runtime.execute(&mut graph).await {
            Ok(_output) => {
                self.drain_stats.total_processed += op_count;
                self.drain_stats.flush_count += 1;
                FlushResult { processed: op_count, failed: 0, persisted: 0 }
            }
            Err(e) => {
                self.event_bus.emit(CatalogEvent::Error {
                    context: "drain".to_string(),
                    message: format!("ingestion dataflow failed: {e}"),
                });
                self.drain_stats.total_failed += op_count;
                self.drain_stats.flush_count += 1;
                FlushResult { processed: 0, failed: op_count, persisted: 0 }
            }
        }
    }

    /// Synchronous drain via block_on (WASM-only).
    /// Uses the same dataflow pipeline as `drain()`.
    #[cfg(feature = "wasm-emscripten")]
    pub fn drain_parallel(&mut self, _pool: &rayon::ThreadPool) -> FlushResult {
        futures::executor::block_on(self.drain())
    }

    /// Flush only InsertOps via a minimal dataflow graph.
    /// Leaves all other ops (Link, Aggregate, Embed, etc.) in `pending_ops`.
    pub async fn flush_insertions(&mut self) -> FlushResult {
        let all_ops = std::mem::take(&mut self.pending_ops);
        let (insert_ops, rest): (Vec<_>, Vec<_>) = all_ops
            .into_iter()
            .partition(|op| matches!(op, CatalogOp::Insert(_)));
        self.pending_ops = rest;

        if insert_ops.is_empty() {
            return FlushResult::default();
        }

        let op_count = insert_ops.len();
        let inserts: Vec<InsertOp> = insert_ops
            .into_iter()
            .filter_map(|op| match op {
                CatalogOp::Insert(i) => Some(i),
                _ => None,
            })
            .collect();

        let mut graph = DataflowGraph::new();
        graph
            .add_node(Box::new(InsertBatchNode::new(
                "inserts".to_string(),
                inserts,
                self.conn.clone(),
                self.node_id_cache.clone(),
            )))
            .unwrap();

        let runtime = DataflowRuntime::new(5);
        match runtime.execute(&mut graph).await {
            Ok(_) => {
                self.drain_stats.total_processed += op_count;
                self.drain_stats.flush_count += 1;
                FlushResult { processed: op_count, failed: 0, persisted: 0 }
            }
            Err(e) => {
                self.event_bus.emit(CatalogEvent::Error {
                    context: "flush_insertions".to_string(),
                    message: format!("insert-only dataflow failed: {e}"),
                });
                self.drain_stats.total_failed += op_count;
                self.drain_stats.flush_count += 1;
                FlushResult { processed: 0, failed: op_count, persisted: 0 }
            }
        }
    }

    pub fn has_pending(&self) -> bool {
        !self.pending_ops.is_empty()
    }

    pub fn queue_stats(&self) -> QueueStats {
        QueueStats {
            pending: self.pending_ops.len(),
            total_queued: self.drain_stats.total_queued,
            total_processed: self.drain_stats.total_processed,
            total_failed: self.drain_stats.total_failed,
            flush_count: self.drain_stats.flush_count,
            ..Default::default()
        }
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
    pub async fn execute_raw(&self, cypher: &str) -> Result<crate::connection::QueryResult, CatalogError> {
        self.conn.execute(cypher).await.map_err(|e| CatalogError::DbError(e.to_string()))
    }

    // ── Event bus ──────────────────────────────────────────────────────

    pub fn subscribe(&self) -> async_broadcast::Receiver<CatalogEvent> {
        self.event_bus.subscribe()
    }

    // ── Node ID cache ─────────────────────────────────────────────────

    /// Access the shared node ID cache (uuid → internal rag3db node ID).
    /// Populated automatically by InsertBatchNode on each INSERT.
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

        let pending_count = self.pending_ops.len();

        // Consistency
        match options.consistency {
            search::Consistency::Strict => {
                self.drain().await;
            }
            search::Consistency::Eventual => {
                if self.has_pending() {
                    self.flush_insertions().await;
                }
            }
            search::Consistency::Immediate => {}
        }

        // Resolve signals: per-query override > KB default
        let kb_config = self
            .config
            .knowledge_bases
            .get(kb_name)
            .cloned()
            .unwrap_or_default();
        let signals = options.signals.unwrap_or(kb_config.signals);

        let search_limit = (options.limit + options.offset) * 2;
        // All searches target {KB}_Index / {KB}_Index_Chunk (not entity tables)
        let entity = format!("{kb_name}_Index");
        let vector_entity = format!("{kb_name}_Index_Chunk");
        // BM25 fields are fixed on the index table
        let bm25_fields: Vec<String> = vec!["_title".to_string(), "_content".to_string()];

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

        // For BM25 search: resolve ALL filters to allowed_ids via title entity.
        // Filters are resolved against the KB's title entity (e.g. Directory for TreeKB),
        // then JOINed to {KB}_Index to get the matching index entry offsets.
        // Cross-entity filters (e.g. "File.extension") are handled by FilterParser's "." notation.
        let allowed_ids = if let Some(ref cond) = condition {
            let title_entity = &kb.title.entity;
            let in_rel = format!("{title_entity}_IN_{kb_name}");

            let mut parser = FilterParser::new(&self.config.relations);
            let parsed = parser
                .parse_condition(cond, title_entity, "t")
                .map_err(|e| CatalogError::FilterError(e.to_string()))?;

            if !parsed.where_clauses.is_empty() {
                let match_extra = if parsed.match_clauses.is_empty() {
                    String::new()
                } else {
                    format!(" {}", parsed.match_clauses.join(" "))
                };
                let cypher = format!(
                    "MATCH (t:{title_entity})-[:{in_rel}]->(idx:{entity}){match_extra} \
                     WHERE {} RETURN OFFSET(id(idx))",
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

        // KB Index always has chunks ({KB}_Index_Chunk)
        let is_chunked = true;

        // Enrichment fields: return index entry data (_title, _content, _source_entity, _source_uuid)
        let enrich_fields: Vec<String> = vec![
            "_title".to_string(),
            "_content".to_string(),
            "_source_entity".to_string(),
            "_source_uuid".to_string(),
            "_content_hash".to_string(),
        ];

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
                    .await
                    .map_err(|e| CatalogError::EmbedError(e.to_string()))?;
                (
                    dense_vecs.into_iter().next().unwrap_or_default(),
                    sparse_vecs.into_iter().next(),
                )
            } else {
                // Fallback: separate embedders
                let dense = search::embed_query(self.embedder.as_ref(), query, &mut self.embedding_cache).await?;
                let sparse = if let Some(ref sparse_emb) = self.sparse_embedder {
                    sparse_emb.embed_sparse(&[query.to_string()]).await
                        .map_err(|e| CatalogError::EmbedError(e.to_string()))?
                        .into_iter().next()
                } else { None };
                (dense, sparse)
            }
        } else if need_dense {
            let dense = if let Some(ref dual_emb) = self.dual_embedder {
                let (dense_vecs, _) = dual_emb.embed_dual(&[query.to_string()]).await
                    .map_err(|e| CatalogError::EmbedError(e.to_string()))?;
                dense_vecs.into_iter().next().unwrap_or_default()
            } else {
                search::embed_query(self.embedder.as_ref(), query, &mut self.embedding_cache).await?
            };
            (dense, None)
        } else if need_sparse {
            let sparse = if let Some(ref dual_emb) = self.dual_embedder {
                let (_, sparse_vecs) = dual_emb.embed_dual(&[query.to_string()]).await
                    .map_err(|e| CatalogError::EmbedError(e.to_string()))?;
                sparse_vecs.into_iter().next()
            } else if let Some(ref sparse_emb) = self.sparse_embedder {
                sparse_emb.embed_sparse(&[query.to_string()]).await
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
            search::search_vector(
                self.conn.as_ref(),
                &vector_entity,
                kb_name,
                &embedding,
                search_limit,
                filter_where.as_deref(),
                &filter_params,
                filter_match.as_deref(),
            )
            .await?
        } else {
            vec![]
        };

        if let Some(ref mut d) = diag { d.vector_ms = t_vector.elapsed().as_millis() as u64; }

        let t_bm25 = Instant::now();
        let bm25_results = if signals.bm25() {
            if is_chunked {
                search::search_bm25_chunked(
                    self.conn.as_ref(), &entity, &vector_entity, query, &bm25_fields,
                    options.bm25_mode, options.fuzzy_distance, search_limit,
                    allowed_ids.as_deref(), &enrich_fields, options.result_mode,
                    diag.as_mut(),
                ).await?
            } else {
                search::search_bm25(
                    self.conn.as_ref(), &entity, query, &bm25_fields,
                    options.bm25_mode, options.fuzzy_distance, search_limit,
                    allowed_ids.as_deref(), &enrich_fields,
                ).await?
            }
        } else {
            vec![]
        };

        if let Some(ref mut d) = diag { d.bm25_ms = t_bm25.elapsed().as_millis() as u64; }

        let vector_count = vector_results.len();
        let bm25_count = bm25_results.len();

        let t_sparse = Instant::now();
        let sparse_results = if let Some(qv) = query_sparse {
            let sparse_fields = if is_chunked { &[][..] } else { &enrich_fields };
            search::search_sparse_cypher(
                self.conn.as_ref(),
                &vector_entity,
                &qv,
                search_limit,
                sparse_fields,
            )
            .await?
        } else {
            vec![]
        };
        if let Some(ref mut d) = diag { d.sparse_ms = t_sparse.elapsed().as_millis() as u64; }
        let sparse_count = sparse_results.len();

        // Resolve chunk-level results to parent-level with ChunkInfo + enrichment
        let t_resolve = Instant::now();
        let vector_results = if is_chunked && !vector_results.is_empty() {
            search::resolve_vector_chunks(
                self.conn.as_ref(), &vector_entity, &entity, vector_results, &enrich_fields,
                options.result_mode,
            ).await?
        } else { vector_results };
        let sparse_results = if is_chunked && !sparse_results.is_empty() {
            search::resolve_vector_chunks(
                self.conn.as_ref(), &vector_entity, &entity, sparse_results, &enrich_fields,
                options.result_mode,
            ).await?
        } else { sparse_results };

        if let Some(ref mut d) = diag { d.resolve_ms = t_resolve.elapsed().as_millis() as u64; }

        let t_fuse = Instant::now();
        let fusion_config = options.fusion.as_ref()
            .cloned()
            .unwrap_or_else(|| kb_config.fusion_config());
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
            search::enrich_results_with_data(
                self.conn.as_ref(), &entity, &enrich_fields, &mut fused,
            ).await?;
        }

        // SourceResolved: resolve index entries → source entities
        if options.result_mode == search::ResultMode::SourceResolved {
            self.resolve_to_source_entities(&mut fused).await?;
        }

        if let Some(ref mut d) = diag { d.enrich_ms = t_enrich.elapsed().as_millis() as u64; }

        let total_ms = search_start.elapsed().as_millis() as u64;
        if let Some(ref mut d) = diag { d.total_ms = total_ms; }

        self.event_bus.emit(CatalogEvent::SearchCompleted {
            kb: kb_name.to_string(),
            results: fused.len(),
            duration_ms: total_ms,
        });

        Ok(search::SearchResponse {
            results: fused,
            meta: search::SearchMeta {
                query: query.to_string(),
                kb: kb_name.to_string(),
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
    async fn resolve_to_source_entities(
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
            let uuid_list = deduped
                .iter()
                .map(|u| format!("'{}'", u.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(", ");
            let cypher = format!(
                "MATCH (n:{entity_name}) WHERE n._uuid IN [{uuid_list}] RETURN n"
            );
            let result = self.conn
                .execute(&cypher)
                .await
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

    /// Find a relation connecting two entity types. Returns (rel_name, entity_a_is_from).
    fn find_relation_to_entity(&self, entity_a: &str, entity_b: &str) -> Option<(String, bool)> {
        for (rel_name, rel_def) in &self.config.relations {
            if rel_def.from == entity_a && rel_def.to == entity_b {
                return Some((rel_name.clone(), true));
            }
            if rel_def.from == entity_b && rel_def.to == entity_a {
                return Some((rel_name.clone(), false));
            }
        }
        None
    }

    fn build_content_text(
        &self,
        entity_name: &str,
        data: &BTreeMap<String, CypherValue>,
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

    /// Build a configured [`DataflowGraph`] for search with strategy.
    ///
    /// Use with [`DataflowRuntime`] for event observation:
    /// ```ignore
    /// let mut graph = Catalog::build_dataflow_graph(catalog, kb, q, strategy).await;
    /// let runtime = DataflowRuntime::new(10);
    /// let mut rx = runtime.subscribe();
    /// let output = runtime.execute(&mut graph).await?;
    /// ```
    pub async fn build_dataflow_graph(
        catalog: Arc<tokio::sync::Mutex<Catalog>>,
        kb_name: &str,
        query: &str,
        strategy: crate::search_strategy::SearchStrategy,
    ) -> crate::dataflow::DataflowGraph {
        use crate::dataflow::*;

        let conn = { catalog.lock().await.conn.clone() };
        let mut graph = DataflowGraph::new();

        // Source node
        graph
            .add_node(Box::new(QuerySourceNode::new(
                kb_name,
                query,
                &strategy.search,
            )))
            .unwrap();

        // Primary search
        graph
            .add_node(Box::new(PrimarySearchNode::new(catalog.clone())))
            .unwrap();
        graph
            .connect("query_source", "query", "primary_search", "query")
            .unwrap();

        if !strategy.expansions.is_empty() {
            // Expansion (dynamic — emits FetchRelated + Compose at runtime)
            graph
                .add_dynamic_node(Box::new(ExpansionNode::new(
                    conn,
                    strategy.expansions,
                )))
                .unwrap();
            graph
                .connect("primary_search", "results", "expansion", "results")
                .unwrap();
        }

        graph
    }

    /// Run a search with reactive expansion (graph traversal after search).
    ///
    /// This is an associated function taking `Arc<Mutex<Catalog>>` so that
    /// nodes can call `catalog.search()`.
    ///
    /// For event observation, use [`Self::build_dataflow_graph()`] +
    /// [`DataflowRuntime::subscribe()`] + [`DataflowRuntime::execute()`].
    pub async fn search_with_strategy(
        catalog: Arc<tokio::sync::Mutex<Catalog>>,
        kb_name: &str,
        query: &str,
        strategy: crate::search_strategy::SearchStrategy,
    ) -> Result<crate::search_strategy::SearchStrategyResponse, CatalogError> {
        use crate::dataflow::PortValue;

        let max_rounds = strategy.max_rounds;
        let has_expansions = !strategy.expansions.is_empty();
        let mut graph =
            Self::build_dataflow_graph(catalog, kb_name, query, strategy).await;

        let runtime = crate::dataflow::DataflowRuntime::new(max_rounds);
        let output = runtime
            .execute(&mut graph)
            .await
            .map_err(|e| CatalogError::DbError(e))?;

        // Results from terminal node
        let results_node = if has_expansions {
            "compose"
        } else {
            "primary_search"
        };
        let results = output
            .get(results_node, "results")
            .and_then(|v| match v {
                PortValue::Results(r) => Some(r.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let meta = output
            .get("primary_search", "meta")
            .and_then(|v| match v {
                PortValue::Meta(m) => Some(m.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                CatalogError::DbError(
                    "search_with_strategy: no meta after processing".into(),
                )
            })?;

        Ok(crate::search_strategy::SearchStrategyResponse { results, meta })
    }
}

// ─── compute_chunk_ops (standalone) ────────────────────────────────────────

/// Build InsertOps, LinkOps, EmbedOps, SparseEmbedOps for all chunks of an entity.
/// Standalone function (no &self) for use by ChunkProcessor in parallel via rayon.
pub fn compute_chunk_ops(
    entity_name: &str,
    parent_uuid: &str,
    entity_ref: &EntityRef,
    data: &BTreeMap<String, CypherValue>,
    config: &CatalogConfig,
    kb_metadata: &HashMap<String, KBMetadata>,
    chunker_cache: &HashMap<ChunkerConfig, Chunker>,
    has_sparse: bool,
    has_dual: bool,
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

        let kb_signals = kb_config.map(|c| c.signals).unwrap_or(search::SearchSignals::HYBRID);
        let kb_sparse = kb_signals.sparse() && has_sparse;
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
            if !field_def.is_chunked() {
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

                let mut chunk_data = BTreeMap::new();
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
                    BTreeMap::new(),
                    link_resolver,
                    link_ref,
                )));

                if has_dual && kb_signals.vector() && kb_sparse {
                    // Single forward pass for both dense + sparse
                    ops.push(CatalogOp::DualEmbed(DualEmbedOp {
                        entity_ref: chunk_ref,
                        kb_name: kb_name.to_string(),
                        texts: vec![embed_text],
                    }));
                } else {
                    if kb_signals.vector() {
                        ops.push(CatalogOp::Embed(EmbedOp {
                            entity_ref: chunk_ref.clone(),
                            kb_name: kb_name.to_string(),
                            texts: vec![embed_text.clone()],
                        }));
                    }

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
    }

    ops
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
    fn count_chunks(body: &str) -> usize {
        let chunker = Chunker::new(ChunkerConfig::default());
        chunker.chunk(body).len()
    }

    /// Ops enqueued at create() time: 1 entity insert + 1 ChunkOp.
    fn ops_enqueued_per_create(_body: &str) -> usize {
        // 1 InsertOp(entity) + 1 InsertOp({KB}_Index) + 1 LinkOp(_IN_) + 1 AggregateOp
        4
    }

    /// Total ops processed after drain():
    /// 2 inserts (entity + index) + 1 link (_IN_) + 1 aggregate (no-op stub).
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

        let err = catalog.create("Ghost", BTreeMap::new()).unwrap_err();
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
            .link("REFERENCES", "uuid-a", "uuid-b", BTreeMap::new())
            .unwrap();

        assert_eq!(rel_ref.relation(), "REFERENCES");
        assert!(!rel_ref.is_ready());
    }

    #[tokio::test]
    async fn link_unknown_relation_errors() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        let err = catalog
            .link("GHOST_REL", "a", "b", BTreeMap::new())
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
                BTreeMap::new(),
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
        assert_eq!(kb.signals, search::SearchSignals::HYBRID);
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

        // Flush prio <= 1.0: 2 InsertOps (entity + {KB}_Index)
        let result = catalog.flush_insertions().await;
        assert_eq!(result.processed, 2);
        assert!(entity_ref.is_ready());

        // LinkOp (prio 2.0) + AggregateOp (prio 2.5) still pending
        assert!(catalog.has_pending());

        // Drain the rest: 1 link + 1 aggregate
        let result = catalog.drain().await;
        assert_eq!(result.processed, 2);
        assert!(!catalog.has_pending());
    }

    // ── filter_condition priority ─────────────────────────────────────

    #[tokio::test]
    async fn search_filter_condition_takes_priority() {
        use crate::filter::{FilterCondition, FilterValue};
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

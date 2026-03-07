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
use crate::records::{AggregateRecord, DrainStats, EntityRecord, FlushResult, PendingWork, RefOrUuid, RelationRecord};
use crate::refs::{EntityRef, RelationRef};
use crate::schema::{generate_full_schema, resolve_entity_kbs};
use crate::chunker::{Chunker, ChunkerConfig};
use crate::uuid::hashsafe_uuid;
use crate::validator::{validate_schema, KBFieldRef};
use crate::dataflow::checkpoint::CheckpointStore;
use crate::dataflow::node_factories::register_builtins;
use crate::dataflow::node_registry::NodeRegistry;
use crate::dataflow::checkpoint_store::CypherCheckpointStore;
use crate::dataflow::graph::DataflowGraph;
use crate::dataflow::port::{BatchPayload, PortType, PortValue};
use crate::dataflow::record_nodes::{
    ChunkKBNode, EmbedRecordNode, FlushFTSNode, GatherKBNode, InsertRecordNode, LinkRecordNode,
    UpdateKBNode,
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
    initialized: bool,
    embedding_cache: HashMap<String, Vec<f32>>,
    /// Cache mapping entity UUIDs to rag3db internal node IDs.
    /// Populated by InsertRecordNode on each INSERT via RETURN ID(n).
    node_id_cache: Arc<RwLock<NodeIdCache>>,
    /// Cached chunkers keyed by config to avoid re-instantiation.
    chunker_cache: HashMap<ChunkerConfig, Chunker>,
    /// Checkpoint store for crash-recovery of drain executions.
    checkpoint_store: Option<Arc<dyn CheckpointStore>>,
    /// Fail injection for testing: if set, the named node will fail during checkpoint execution.
    fail_node: Option<String>,
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
            initialized: false,
            embedding_cache: HashMap::new(),
            node_id_cache: Arc::new(RwLock::new(NodeIdCache::new())),
            chunker_cache: HashMap::new(),
            checkpoint_store: None,
            fail_node: None,
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

    /// Set a custom checkpoint store. Must be called before `initialize()`.
    /// If set, `initialize()` will skip creating the default `CypherCheckpointStore`.
    pub fn set_checkpoint_store(&mut self, store: Arc<dyn CheckpointStore>) {
        self.checkpoint_store = Some(store);
    }

    /// Set a node name that should fail during checkpoint execution (testing only).
    /// The named node will return an injected error instead of executing.
    pub fn set_fail_node(&mut self, node_name: Option<&str>) {
        self.fail_node = node_name.map(|s| s.to_string());
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

        // 7. Initialize checkpoint store for crash-recovery (unless already set by tests)
        if self.checkpoint_store.is_none() {
            let cp_store: Arc<dyn CheckpointStore> =
                Arc::new(CypherCheckpointStore::new(self.conn.clone()));
            cp_store
                .initialize()
                .await
                .map_err(|e| CatalogError::DbError(e))?;
            self.checkpoint_store = Some(cp_store);
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
            // Sentinel hash: empty string forces GatherKBNode to always run on first drain.
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

        // If content changed, enqueue AggregateRecords for all KBs this entity contributes to.
        // GatherKBNode will handle deleting old chunks, re-chunking, and re-embedding.
        let mut reembedded = false;
        let chunks_deleted = 0usize;
        let chunks_created = 0usize;
        if content_changed {
            let entity_def = &self.config.entities[entity_name];
            let entity_kbs = resolve_entity_kbs(entity_def);

            for (kb_name, mapping) in &entity_kbs {
                if mapping.title_field.is_some() {
                    // This entity is the title entity for this KB → aggregate its index entry
                    let index_uuid = hashsafe_uuid(
                        &format!("{kb_name}_Index"),
                        &[entity_name, uuid],
                    );
                    self.pending.aggregates.push(AggregateRecord {
                        index_entry_uuid: index_uuid,
                        kb_name: kb_name.clone(),
                        title_entity: entity_name.to_string(),
                        source_uuid: uuid.to_string(),
                    });
                    self.drain_counters.total_queued += 1;
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
                                self.pending.aggregates.push(AggregateRecord {
                                    index_entry_uuid: index_uuid,
                                    kb_name: kb_name.clone(),
                                    title_entity: title_entity.clone(),
                                    source_uuid: title_uuid.to_string(),
                                });
                                self.drain_counters.total_queued += 1;
                                reembedded = true;
                            }
                        }
                    }
                }
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

                // Find linked title entities → enqueue AggregateRecords to rebuild their content
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
                            self.pending.aggregates.push(AggregateRecord {
                                index_entry_uuid: index_uuid,
                                kb_name: kb_name.clone(),
                                title_entity: title_entity.clone(),
                                source_uuid: title_uuid.to_string(),
                            });
                            self.drain_counters.total_queued += 1;
                        }
                    }
                }
            }
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

    /// Build a dataflow graph from all pending records.
    ///
    /// Consumes `self.pending` (PendingWork) and builds a record-based graph:
    ///
    /// ```text
    /// entities → InsertRecordNode("inserts")
    ///                 └── done → LinkRecordNode("links") ← relations
    ///                               └── done → GatherKBNode("gather_kb") ← aggregates
    ///                                             └── kb_content → UpdateKBNode("update_kb")
    ///                                                                  └── kb_content → ChunkKBNode("chunk_kb")
    ///                                                                                      ├── entities → InsertRecordNode("agg_inserts")
    ///                                                                                      ├── relations → LinkRecordNode("agg_links")
    ///                                                                                      └── agg_inserts ── done → EmbedRecordNode("agg_embeds")
    /// ```
    ///
    /// No ChunkRecordNode (entity-level chunks unused by search — future Mermaid template).
    /// No EmbedRecordNode on raw entities (only KB_Index_Chunk are searched).
    fn build_ingestion_graph(&mut self) -> (DataflowGraph, ServiceRegistry, usize) {
        let pending = std::mem::take(&mut self.pending);
        if pending.is_empty() {
            return (DataflowGraph::new(), ServiceRegistry::new(), 0);
        }

        let op_count = pending.total_count();
        let has_entities = !pending.entities.is_empty();
        let has_relations = !pending.relations.is_empty();
        let has_aggregates = !pending.aggregates.is_empty();

        // Capture unique KB names before aggregates are consumed by the graph
        let flush_kb_names: Vec<String> = if has_aggregates {
            let mut seen = HashSet::new();
            pending.aggregates.iter()
                .filter_map(|a| if seen.insert(a.kb_name.clone()) { Some(a.kb_name.clone()) } else { None })
                .collect()
        } else {
            vec![]
        };

        let mut graph = DataflowGraph::new();

        // 1. InsertRecordNode("inserts") — raw entities
        if has_entities {
            graph.add_node(Box::new(InsertRecordNode::new("inserts"))).unwrap();
            graph.set_initial_input("inserts", "entities",
                PortValue::Batch(BatchPayload::new(PortType::Entities, pending.entities)));
        }

        // 2. LinkRecordNode("links") — raw relations, triggered after inserts
        if has_relations {
            graph.add_node(Box::new(LinkRecordNode::new("links"))).unwrap();
            graph.set_initial_input("links", "relations",
                PortValue::Batch(BatchPayload::new(PortType::Relations, pending.relations)));
            if has_entities {
                graph.connect("inserts", "done", "links", "trigger").unwrap();
            }
        }

        // 3. KB pipeline: gather → update → chunk, triggered after links
        if has_aggregates {
            self.warm_chunker_cache();

            graph.add_node(Box::new(GatherKBNode::new("gather_kb"))).unwrap();
            graph.set_initial_input("gather_kb", "aggregates",
                PortValue::Batch(BatchPayload::new(PortType::Aggregates, pending.aggregates)));
            if has_relations {
                graph.connect("links", "done", "gather_kb", "trigger").unwrap();
            } else if has_entities {
                graph.connect("inserts", "done", "gather_kb", "trigger").unwrap();
            }

            graph.add_node(Box::new(UpdateKBNode::new("update_kb"))).unwrap();
            graph.connect("gather_kb", "kb_content", "update_kb", "kb_content").unwrap();

            graph.add_node(Box::new(ChunkKBNode::new("chunk_kb"))).unwrap();
            graph.connect("update_kb", "kb_content", "chunk_kb", "kb_content").unwrap();

            // Downstream standard: insert chunks → link chunks → embed chunks
            graph.add_node(Box::new(InsertRecordNode::new("agg_inserts"))).unwrap();
            graph.connect("chunk_kb", "entities", "agg_inserts", "entities").unwrap();

            graph.add_node(Box::new(LinkRecordNode::new("agg_links"))).unwrap();
            graph.connect("chunk_kb", "relations", "agg_links", "relations").unwrap();
            graph.connect("agg_inserts", "done", "agg_links", "trigger").unwrap();

            graph.add_node(Box::new(EmbedRecordNode::new("agg_embeds", 32))).unwrap();
            graph.connect("agg_inserts", "inserted", "agg_embeds", "entities").unwrap();
            graph.connect("agg_links", "done", "agg_embeds", "trigger").unwrap();

            // Flush FTS indexes in parallel with chunk/insert/embed pipeline
            graph.add_node(Box::new(FlushFTSNode::new("flush_fts"))).unwrap();
            graph.connect("update_kb", "done", "flush_fts", "trigger").unwrap();
        }

        // Build ServiceRegistry with all shared services
        let mut services = ServiceRegistry::new();
        services.register::<Arc<dyn DbConnection>>("conn", Arc::new(self.conn.clone()));
        services.register::<RwLock<NodeIdCache>>("node_id_cache", self.node_id_cache.clone());
        services.register::<Arc<dyn Embedder>>("embedder", Arc::new(self.embedder.clone()));
        services.register::<usize>("embedding_dim", Arc::new(self.config.embedding_dim));
        services.register::<CatalogConfig>("config", Arc::new(self.config.clone()));
        services.register::<HashMap<String, KBMetadata>>("kb_metadata", Arc::new(self.kb_metadata.clone()));
        services.register::<bool>("has_sparse", Arc::new(
            self.sparse_embedder.is_some() || self.dual_embedder.is_some(),
        ));
        services.register::<bool>("has_dual", Arc::new(self.dual_embedder.is_some()));

        if has_aggregates {
            services.register::<HashMap<ChunkerConfig, Chunker>>(
                "chunker_cache",
                Arc::new(std::mem::take(&mut self.chunker_cache)),
            );
            services.register::<Vec<String>>("flush_kb_names", Arc::new(flush_kb_names));
        }
        if let Some(ref sparse_emb) = self.sparse_embedder {
            services.register::<Arc<dyn SparseEmbedder>>("sparse_embedder", Arc::new(sparse_emb.clone()));
        }
        if let Some(ref dual_emb) = self.dual_embedder {
            services.register::<Arc<dyn DualEmbedder>>("dual_embedder", Arc::new(dual_emb.clone()));
        }
        if let Some(ref fail_node) = self.fail_node {
            services.register::<String>("fail_node", Arc::new(fail_node.clone()));
        }

        (graph, services, op_count)
    }

    /// Drain all pending operations via the dataflow runtime with checkpoint persistence.
    pub async fn drain(&mut self) -> FlushResult {
        let (mut graph, services, op_count) = self.build_ingestion_graph();
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
                .await
        } else {
            runtime.execute(&mut graph).await
        };

        match result {
            Ok(_output) => {
                self.drain_counters.total_processed += op_count;
                self.drain_counters.flush_count += 1;
                FlushResult { processed: op_count, failed: 0 }
            }
            Err(e) => {
                self.event_bus.emit(CatalogEvent::Error {
                    context: "drain".to_string(),
                    message: format!("ingestion dataflow failed: {e}"),
                });
                self.drain_counters.total_failed += op_count;
                self.drain_counters.flush_count += 1;
                FlushResult { processed: 0, failed: op_count }
            }
        }
    }

    /// Synchronous drain via block_on (WASM-only).
    /// Uses the same dataflow pipeline as `drain()`.
    #[cfg(feature = "wasm-emscripten")]
    pub fn drain_parallel(&mut self, _pool: &rayon::ThreadPool) -> FlushResult {
        futures::executor::block_on(self.drain())
    }

    /// Flush only entity inserts via a minimal dataflow graph.
    /// Leaves relations and aggregates in `pending` for a later `drain()`.
    pub async fn flush_insertions(&mut self) -> FlushResult {
        let entities = std::mem::take(&mut self.pending.entities);
        if entities.is_empty() {
            return FlushResult::default();
        }

        let op_count = entities.len();
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(InsertRecordNode::new("inserts"))).unwrap();
        graph.set_initial_input("inserts", "entities",
            PortValue::Batch(BatchPayload::new(PortType::Entities, entities)));

        let mut services = ServiceRegistry::new();
        services.register::<Arc<dyn DbConnection>>("conn", Arc::new(self.conn.clone()));
        services.register::<RwLock<NodeIdCache>>("node_id_cache", self.node_id_cache.clone());

        let runtime = DataflowRuntime::with_services(5, services);
        match runtime.execute(&mut graph).await {
            Ok(_) => {
                self.drain_counters.total_processed += op_count;
                self.drain_counters.flush_count += 1;
                FlushResult { processed: op_count, failed: 0 }
            }
            Err(e) => {
                self.event_bus.emit(CatalogEvent::Error {
                    context: "flush_insertions".to_string(),
                    message: format!("insert-only dataflow failed: {e}"),
                });
                self.drain_counters.total_failed += op_count;
                self.drain_counters.flush_count += 1;
                FlushResult { processed: 0, failed: op_count }
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
    pub async fn drain_resume(&mut self, execution_id: &str) -> Result<FlushResult, CatalogError> {
        let store = self
            .checkpoint_store
            .clone()
            .ok_or(CatalogError::NotInitialized)?;

        // Load the checkpoint to get the graph definition
        let checkpoint = store
            .load_execution(execution_id)
            .await
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
        services.register::<Arc<dyn DbConnection>>("conn", Arc::new(self.conn.clone()));
        services.register::<RwLock<NodeIdCache>>("node_id_cache", self.node_id_cache.clone());
        services.register::<Arc<dyn Embedder>>("embedder", Arc::new(self.embedder.clone()));
        services.register::<usize>("embedding_dim", Arc::new(self.config.embedding_dim));
        services.register::<CatalogConfig>("config", Arc::new(self.config.clone()));
        services.register::<HashMap<String, KBMetadata>>(
            "kb_metadata",
            Arc::new(self.kb_metadata.clone()),
        );
        services.register::<bool>(
            "has_sparse",
            Arc::new(self.sparse_embedder.is_some() || self.dual_embedder.is_some()),
        );
        services.register::<bool>("has_dual", Arc::new(self.dual_embedder.is_some()));

        // Chunker cache: rebuild for KB nodes
        self.warm_chunker_cache();
        services.register::<HashMap<ChunkerConfig, Chunker>>(
            "chunker_cache",
            Arc::new(std::mem::take(&mut self.chunker_cache)),
        );

        // flush_kb_names: extract from graph node names (GatherKBNode uses kb_metadata)
        let flush_kb_names: Vec<String> = self.kb_metadata.keys().cloned().collect();
        services.register::<Vec<String>>("flush_kb_names", Arc::new(flush_kb_names));

        if let Some(ref sparse_emb) = self.sparse_embedder {
            services.register::<Arc<dyn SparseEmbedder>>(
                "sparse_embedder",
                Arc::new(sparse_emb.clone()),
            );
        }
        if let Some(ref dual_emb) = self.dual_embedder {
            services
                .register::<Arc<dyn DualEmbedder>>("dual_embedder", Arc::new(dual_emb.clone()));
        }
        if let Some(ref fail_node) = self.fail_node {
            services.register::<String>("fail_node", Arc::new(fail_node.clone()));
        }

        let node_count = graph.nodes.len();
        let runtime = DataflowRuntime::with_services(node_count + 20, services);

        match runtime
            .execute_with_checkpoint(&mut graph, store.as_ref(), execution_id)
            .await
        {
            Ok(_) => {
                self.drain_counters.flush_count += 1;
                Ok(FlushResult {
                    processed: node_count,
                    failed: 0,
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
                })
            }
        }
    }

    /// Check for incomplete checkpoint executions (status=Running).
    ///
    /// Returns execution IDs that can be passed to `drain_resume()`.
    pub async fn check_pending_checkpoints(&self) -> Result<Vec<String>, CatalogError> {
        let store = self
            .checkpoint_store
            .as_ref()
            .ok_or(CatalogError::NotInitialized)?;
        store
            .find_incomplete()
            .await
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
    pub async fn execute_raw(&self, cypher: &str) -> Result<crate::connection::QueryResult, CatalogError> {
        self.conn.execute(cypher).await.map_err(|e| CatalogError::DbError(e.to_string()))
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

        let pending_count = self.pending.total_count();

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

    /// Build a configured [`DataflowGraph`] + [`ServiceRegistry`] for search with strategy.
    ///
    /// Use with [`DataflowRuntime`] for event observation:
    /// ```ignore
    /// let (mut graph, services) = Catalog::build_dataflow_graph(catalog, kb, q, strategy).await;
    /// let runtime = DataflowRuntime::with_services(10, services);
    /// let mut rx = runtime.subscribe();
    /// let output = runtime.execute(&mut graph).await?;
    /// ```
    pub async fn build_dataflow_graph(
        catalog: Arc<tokio::sync::Mutex<Catalog>>,
        kb_name: &str,
        query: &str,
        strategy: crate::search_strategy::SearchStrategy,
    ) -> (crate::dataflow::DataflowGraph, crate::dataflow::ServiceRegistry) {
        use crate::dataflow::*;
        use crate::dataflow::services::ConnService;

        let mut graph = DataflowGraph::new();

        // Services
        let mut services = ServiceRegistry::new();
        let conn = catalog.lock().await.conn_arc();
        services.register::<tokio::sync::Mutex<Catalog>>("catalog", catalog.clone());
        services.register("conn", std::sync::Arc::new(ConnService(conn)));

        // Source node
        graph
            .add_node(Box::new(QuerySourceNode::new(
                kb_name,
                query,
                &strategy.search,
            )))
            .unwrap();

        // Primary search (catalog resolved via service)
        graph
            .add_node(Box::new(PrimarySearchNode::new("primary_search")))
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
    pub async fn search_with_strategy(
        catalog: Arc<tokio::sync::Mutex<Catalog>>,
        kb_name: &str,
        query: &str,
        strategy: crate::search_strategy::SearchStrategy,
    ) -> Result<crate::search_strategy::SearchStrategyResponse, CatalogError> {
        use crate::dataflow::PortValue;

        let max_rounds = strategy.max_rounds;
        let has_expansions = !strategy.expansions.is_empty();
        let (mut graph, services) =
            Self::build_dataflow_graph(catalog, kb_name, query, strategy).await;

        let runtime = crate::dataflow::DataflowRuntime::with_services(max_rounds, services);
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

        let stats = catalog.drain_stats();
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

    // ── drain stats ────────────────────────────────────────────────────

    #[tokio::test]
    async fn has_pending_and_stats() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

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

        catalog.drain().await;

        assert!(!catalog.has_pending());
        let stats = catalog.drain_stats();
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

    // ── Phase A: Shadow records tests ─────────────────────────────────

    #[tokio::test]
    async fn create_populates_pending_work() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

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

    #[tokio::test]
    async fn link_populates_pending_work() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

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

    #[tokio::test]
    async fn drain_clears_pending_work() {
        let mut catalog = make_catalog();
        catalog.initialize().await.unwrap();

        catalog.create("Document", make_doc_data("Test", "body")).unwrap();
        assert!(!catalog.pending_work().is_empty());

        // drain() clears pending work
        let _ = catalog.drain().await;
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

    #[tokio::test]
    async fn checkpoint_drain_marks_completed() {
        let (mut catalog, store) = make_catalog_with_mock_checkpoint();
        catalog.initialize().await.unwrap();

        catalog.create("Document", make_doc_data("Test", "Body")).unwrap();
        let result = catalog.drain().await;
        assert_eq!(result.failed, 0);
        assert!(result.processed > 0);

        // Checkpoint should be marked completed (no pending checkpoints)
        let pending = catalog.check_pending_checkpoints().await.unwrap();
        assert!(pending.is_empty(), "checkpoint should be cleaned up after successful drain");
    }

    #[tokio::test]
    async fn checkpoint_resume_nonexistent_returns_error() {
        let (mut catalog, _store) = make_catalog_with_mock_checkpoint();
        catalog.initialize().await.unwrap();

        let err = catalog.drain_resume("nonexistent-exec-id").await;
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(msg.contains("not found"), "expected 'not found' error, got: {msg}");
    }

    #[tokio::test]
    async fn checkpoint_resume_already_completed_is_noop() {
        let (mut catalog, store) = make_catalog_with_mock_checkpoint();
        catalog.initialize().await.unwrap();

        // Do a normal drain to create a completed checkpoint
        catalog.create("Document", make_doc_data("Test", "Body")).unwrap();
        let result = catalog.drain().await;
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
        let resume_result = catalog.drain_resume(&exec_id).await.unwrap();
        // execute_with_checkpoint returns Ok(DataflowOutput::empty()) for completed,
        // so drain_resume sees Ok → reports processed
        assert_eq!(resume_result.failed, 0);
    }

    #[tokio::test]
    async fn checkpoint_check_pending_empty_initially() {
        let (mut catalog, _store) = make_catalog_with_mock_checkpoint();
        catalog.initialize().await.unwrap();

        let pending = catalog.check_pending_checkpoints().await.unwrap();
        assert!(pending.is_empty());
    }
}

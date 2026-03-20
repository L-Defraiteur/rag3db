//! Record-based ingestion nodes (Phase B — doc 23).
//!
//! These replace the op-based batch nodes with typed record inputs.
//! The graph topology encodes the execution plan — records carry only data.
//! All nodes are **idempotent** — safe to re-run after crash or partial execution.
//!
//! - [`InsertRecordNode`] — UNWIND MERGE on `_uuid` from `Vec<EntityRecord>`
//! - [`LinkRecordNode`] — UNWIND MATCH+MERGE from `Vec<RelationRecord>`
//! - [`KBEmbedNode`] — unified embedding with `_embed_hash` skip (saves GPU)
//! - [`ChunkRecordNode`] — parallel chunking for simple entities (entity_configs)
//! - [`EmbedNode`] — embedding for simple entities (configurable columns)
//! - [`KBChunkRecordNode`] — parallel chunking for KB entities (kb_metadata)
//! - [`KBGatherNode`] — read DB, detect content changes, output changed KBContentRecords
//! - [`KBUpdateNode`] — update KB_Index entries + delete stale chunks
//! - [`KBChunkNode`] — generate chunk entities + relations from aggregated content
//! - [`DeleteRecordNode`] — batch cascade-delete entities + chunks from Vec<DeleteRecord>
//! - [`UpdateRecordNode`] — batch field update + change detection from Vec<UpdateRecord>
//! - [`RechunkDeleteNode`] — delete old chunks before re-chunking (pass-through)

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};

use async_trait::async_trait;

use crate::catalog::KBMetadata;
use crate::events::{CatalogEvent, EventBus};
use crate::chunker::{Chunker, ChunkerConfig};
use crate::config::CatalogConfig;
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::embedder::{DualEmbedder, Embedder, SparseEmbedder};
use crate::hash::content_hash;
use crate::node_id_cache::{InternalNodeId, NodeIdCache};
use crate::records::{
    AggregateRecord, DeleteRecord, DeleteResult, EntityRecord, KBContentRecord,
    RecordSourceContent, RefOrUuid, RelationRecord, UpdateRecord, UpdateResult, UpdateStatus,
};
use crate::refs::{EntityRef, RelationRef};
use crate::search;
use crate::sparse_index::SparseVector;
use crate::schema::resolve_entity_kbs;
use crate::uuid::{chunk_uuid, hashsafe_uuid};

use super::node::{Node, NodeContext};
use super::port::{BatchPayload, PortDef, PortType, PortValue};

// ─── InsertRecordNode ───────────────────────────────────────────────────────

/// Batch INSERT from `Vec<EntityRecord>`: UNWIND MERGE on `_uuid` grouped by
/// `(entity_name, column_set)`, resolves EntityRefs, caches node IDs.
/// Idempotent: re-running with the same `_uuid` updates instead of duplicating.
///
/// **Input**: `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
/// **Output**: `done` — Empty signal, `inserted` — entities with resolved refs
/// **Services**: `conn` (DbConnection), `node_id_cache` (RwLock<NodeIdCache>)
pub struct InsertRecordNode {
    name: String,
    undo_data: Option<serde_json::Value>,
}

impl InsertRecordNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), undo_data: None }
    }
}

#[async_trait]
impl Node for InsertRecordNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "InsertRecordNode"
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "entities", port_type: PortType::Entities, required: true },
            PortDef { name: "trigger", port_type: PortType::Empty, required: false },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "done", port_type: PortType::Empty, required: false },
            PortDef { name: "inserted", port_type: PortType::Entities, required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut items: Vec<EntityRecord> = match ctx.take_input("entities") {
            Some(PortValue::Batch(payload)) => payload
                .take::<EntityRecord>()
                .ok_or("InsertRecordNode: failed to extract Vec<EntityRecord>")?,
            _ => return Err("InsertRecordNode: missing 'entities' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("InsertRecordNode: 'conn' service not registered")?;
        let node_id_cache = ctx.service::<RwLock<NodeIdCache>>("node_id_cache")
            .ok_or("InsertRecordNode: 'node_id_cache' service not registered")?;

        // Group by (entity_name, sorted column_set) for UNWIND batching.
        let mut groups: HashMap<(String, Vec<String>), Vec<usize>> = HashMap::new();
        for (i, rec) in items.iter().enumerate() {
            let mut columns: Vec<String> = rec.data.keys().cloned().collect();
            columns.sort();
            groups
                .entry((rec.entity_name.clone(), columns))
                .or_default()
                .push(i);
        }

        ctx.log_metric("items", items.len());
        ctx.log_metric("groups", groups.len());
        ctx.log_metric("group_summary", groups.iter()
            .map(|((name, _), idxs)| format!("{}×{}", name, idxs.len()))
            .collect::<Vec<_>>());

        for ((entity_name, columns), indices) in &groups {
            let col_refs: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();

            // Build batch upsert via dialect (idempotent MERGE/INSERT ON CONFLICT)
            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .ok_or("InsertRecordNode: 'dialect' service not registered")?;
            let cypher = dialect.batch_upsert(entity_name, &col_refs);

            // Build items list param
            let items_param = CypherValue::List(
                indices
                    .iter()
                    .map(|&i| {
                        let rec = &items[i];
                        let mut map = BTreeMap::new();
                        for col in &col_refs {
                            map.insert(
                                col.to_string(),
                                rec.data
                                    .get(*col)
                                    .cloned()
                                    .unwrap_or(CypherValue::Null),
                            );
                        }
                        CypherValue::Map(map)
                    })
                    .collect(),
            );

            let result = conn
                .execute_with_params(
                    &cypher,
                    &[QueryParam {
                        name: "items".to_string(),
                        value: items_param,
                    }],
                )
                .await
                .map_err(|e| e.to_string())?;

            // Build UUID → node_id map for safe matching (don't rely on row order)
            let mut uuid_to_node_id: HashMap<String, String> = HashMap::new();
            for row in &result.rows {
                if let (Some(id_val), Some(uuid_val)) = (row.first(), row.get(1)) {
                    if let (Some(id_str), Some(uuid_str)) = (id_val.as_str(), uuid_val.as_str()) {
                        uuid_to_node_id.insert(uuid_str.to_string(), id_str.to_string());
                    }
                }
            }

            // Resolve refs + cache node IDs
            for &i in indices {
                let rec = &mut items[i];
                let uuid = rec
                    .data
                    .get("_uuid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(id_str) = uuid_to_node_id.get(&uuid) {
                    if let Some(node_id) = InternalNodeId::parse(id_str) {
                        if let Ok(mut cache) = node_id_cache.write() {
                            cache.insert(&uuid, node_id);
                        }
                    }
                }

                if let Some(resolver) = rec.take_resolver() {
                    resolver.resolve(uuid);
                }
            }
        }

        // Capture undo data: entity_name → [uuid] for DELETE
        let mut undo_groups: HashMap<String, Vec<String>> = HashMap::new();
        for rec in &items {
            if let Some(uuid) = rec.data.get("_uuid").and_then(|v| v.as_str()) {
                undo_groups.entry(rec.entity_name.clone()).or_default().push(uuid.to_string());
            }
        }
        self.undo_data = Some(serde_json::json!(undo_groups));

        ctx.set_output("done", PortValue::Empty);
        ctx.set_output("inserted", PortValue::Batch(
            BatchPayload::new(PortType::Entities, items),
        ));
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<serde_json::Value> {
        self.undo_data.clone()
    }

    async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String> {
        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("InsertRecordNode undo: 'conn' service not registered")?;

        let groups = undo_ctx.as_object()
            .ok_or("InsertRecordNode undo: expected object")?;

        for (entity_name, uuids) in groups {
            let uuid_list: Vec<&str> = uuids.as_array()
                .ok_or("InsertRecordNode undo: expected array of uuids")?
                .iter()
                .filter_map(|v| v.as_str())
                .collect();

            if uuid_list.is_empty() { continue; }

            let uuid_params = CypherValue::List(
                uuid_list.iter().map(|u| CypherValue::String(u.to_string())).collect()
            );
            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .ok_or("InsertRecordNode undo: 'dialect' service not registered")?;
            let cypher = dialect.batch_cascade_delete(entity_name);
            conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "uuids".into(), value: uuid_params }],
            ).await.map_err(|e| format!("InsertRecordNode undo failed: {e}"))?;
        }
        Ok(())
    }
}

// ─── LinkRecordNode ─────────────────────────────────────────────────────────

/// Batch LINK from `Vec<RelationRecord>`: resolves from/to refs,
/// UNWIND MATCH+MERGE grouped by `(rel_name, property_keys)`.
/// Idempotent: re-running with the same from/to/rel_name skips existing relations.
///
/// **Input**: `relations` — `BatchPayload<RelationRecord>` (PortType::Relations)
/// **Output**: `done` — Empty signal
/// **Services**: `conn` (DbConnection)
pub struct LinkRecordNode {
    name: String,
    undo_data: Option<serde_json::Value>,
}

impl LinkRecordNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), undo_data: None }
    }
}

#[async_trait]
impl Node for LinkRecordNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "LinkRecordNode"
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "relations", port_type: PortType::Relations, required: true },
            PortDef { name: "trigger", port_type: PortType::Empty, required: false },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "done",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut items: Vec<RelationRecord> = match ctx.take_input("relations") {
            Some(PortValue::Batch(payload)) => payload
                .take::<RelationRecord>()
                .ok_or("LinkRecordNode: failed to extract Vec<RelationRecord>")?,
            _ => return Err("LinkRecordNode: missing 'relations' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("LinkRecordNode: 'conn' service not registered")?;

        // Resolve all refs first (should be instant — InsertRecordNode already completed)
        struct ResolvedLink {
            from_uuid: String,
            to_uuid: String,
            index: usize,
        }
        let mut resolved: Vec<ResolvedLink> = Vec::with_capacity(items.len());
        for (i, rel) in items.iter_mut().enumerate() {
            let from_uuid = rel
                .from
                .resolve()
                .await
                .map_err(|e| format!("link from resolution failed: {e}"))?;
            let to_uuid = rel
                .to
                .resolve()
                .await
                .map_err(|e| format!("link to resolution failed: {e}"))?;
            resolved.push(ResolvedLink { from_uuid, to_uuid, index: i });
        }

        // Group by (rel_name, sorted property keys) for UNWIND batching.
        let mut groups: HashMap<(String, Vec<String>), Vec<usize>> = HashMap::new();
        for (ri, rl) in resolved.iter().enumerate() {
            let rel = &items[rl.index];
            let mut prop_keys: Vec<String> = rel.properties.keys().cloned().collect();
            prop_keys.sort();
            groups
                .entry((rel.rel_name.clone(), prop_keys))
                .or_default()
                .push(ri);
        }

        ctx.log_metric("items", items.len());
        ctx.log_metric("groups", groups.len());
        ctx.log_metric("group_summary", groups.iter()
            .map(|((name, _), idxs)| format!("{}×{}", name, idxs.len()))
            .collect::<Vec<_>>());

        for ((rel_name, prop_keys), indices) in &groups {
            // Build batch link via dialect (idempotent — skip if relation already exists)
            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .ok_or("LinkRecordNode: 'dialect' service not registered")?;
            let prop_refs: Vec<&str> = prop_keys.iter().map(|s| s.as_str()).collect();
            let cypher = dialect.batch_link(rel_name, &prop_refs);

            let items_param = CypherValue::List(
                indices
                    .iter()
                    .map(|&ri| {
                        let rl = &resolved[ri];
                        let rel = &items[rl.index];
                        let mut map = BTreeMap::new();
                        map.insert(
                            "from_uuid".to_string(),
                            CypherValue::String(rl.from_uuid.clone()),
                        );
                        map.insert(
                            "to_uuid".to_string(),
                            CypherValue::String(rl.to_uuid.clone()),
                        );
                        for key in prop_keys {
                            map.insert(
                                key.clone(),
                                rel.properties.get(key).cloned().unwrap_or(CypherValue::Null),
                            );
                        }
                        CypherValue::Map(map)
                    })
                    .collect(),
            );

            conn
                .execute_with_params(
                    &cypher,
                    &[QueryParam {
                        name: "items".to_string(),
                        value: items_param,
                    }],
                )
                .await
                .map_err(|e| e.to_string())?;

            // Resolve relation refs
            for &ri in indices {
                let rl = &resolved[ri];
                let rel = &mut items[rl.index];
                if let Some(resolver) = rel.take_resolver() {
                    resolver.resolve(rl.from_uuid.clone(), rl.to_uuid.clone());
                }
            }
        }

        // Capture undo data: rel_name → [{from, to}]
        let mut undo_groups: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
        for rl in &resolved {
            let rel = &items[rl.index];
            undo_groups.entry(rel.rel_name.clone()).or_default().push(
                serde_json::json!({"from": rl.from_uuid, "to": rl.to_uuid})
            );
        }
        self.undo_data = Some(serde_json::json!(undo_groups));

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<serde_json::Value> {
        self.undo_data.clone()
    }

    async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String> {
        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("LinkRecordNode undo: 'conn' service not registered")?;

        let groups = undo_ctx.as_object()
            .ok_or("LinkRecordNode undo: expected object")?;

        for (rel_name, links) in groups {
            let link_list = links.as_array()
                .ok_or("LinkRecordNode undo: expected array of links")?;

            let items_param = CypherValue::List(
                link_list.iter().filter_map(|link| {
                    let from = link["from"].as_str()?;
                    let to = link["to"].as_str()?;
                    let mut m = BTreeMap::new();
                    m.insert("from".into(), CypherValue::String(from.to_string()));
                    m.insert("to".into(), CypherValue::String(to.to_string()));
                    Some(CypherValue::Map(m))
                }).collect()
            );

            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .ok_or("LinkRecordNode undo: 'dialect' service not registered")?;
            let cypher = dialect.batch_delete_relation(rel_name);
            conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "items".into(), value: items_param }],
            ).await.map_err(|e| format!("LinkRecordNode undo failed: {e}"))?;
        }
        Ok(())
    }
}

// ─── KBEmbedNode ────────────────────────────────────────────────────────

/// Unified embedding node: takes `Vec<EntityRecord>`, determines which
/// embeddings to compute (dense/sparse/dual) based on KB config, calls
/// the appropriate embedder(s), and UNWIND SETs results back to DB.
/// Idempotent: compares `_embed_hash` before re-embedding — skips unchanged
/// content (saves GPU). Persists `_embed_hash` after each embedding.
///
/// **Input**: `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
/// **Output**: `done` — Empty signal
/// **Services**: `conn`, `embedder`, `embedding_dim`, `config`, `kb_metadata`,
///               optionally `sparse_embedder`, `dual_embedder`
pub struct KBEmbedNode {
    name: String,
    gpu_batch_size: usize,
    undo_data: Option<serde_json::Value>,
}

impl KBEmbedNode {
    pub fn new(name: impl Into<String>, gpu_batch_size: usize) -> Self {
        Self {
            name: name.into(),
            gpu_batch_size,
            undo_data: None,
        }
    }
}

/// Internal work item for embedding.
struct EmbedWork {
    uuid: String,
    text: String,
    text_hash: String,
    entity_name: String,
    kb_name: String,
}

#[async_trait]
impl Node for KBEmbedNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "KBEmbedNode"
    }
    fn node_config(&self) -> serde_json::Value {
        serde_json::json!({ "gpu_batch_size": self.gpu_batch_size })
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "entities", port_type: PortType::Entities, required: true },
            PortDef { name: "trigger", port_type: PortType::Empty, required: false },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "done",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut items: Vec<EntityRecord> = match ctx.take_input("entities") {
            Some(PortValue::Batch(payload)) => payload
                .take::<EntityRecord>()
                .ok_or("KBEmbedNode: failed to extract Vec<EntityRecord>")?,
            _ => return Err("KBEmbedNode: missing 'entities' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("KBEmbedNode: 'conn' service not registered")?;
        let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
            .ok_or("KBEmbedNode: 'dialect' service not registered")?;
        let config = ctx.service::<CatalogConfig>("config")
            .ok_or("KBEmbedNode: 'config' service not registered")?;
        let embedder = ctx.service::<Arc<dyn Embedder>>("embedder")
            .ok_or("KBEmbedNode: 'embedder' service not registered")?;
        let embedding_dim = *ctx.service::<usize>("embedding_dim")
            .ok_or("KBEmbedNode: 'embedding_dim' service not registered")?;
        let has_sparse_svc = ctx.service::<bool>("has_sparse").map(|v| *v).unwrap_or(false);
        let has_dual_svc = ctx.service::<bool>("has_dual").map(|v| *v).unwrap_or(false);
        let sparse_embedder = ctx.service::<Arc<dyn SparseEmbedder>>("sparse_embedder");
        let dual_embedder = ctx.service::<Arc<dyn DualEmbedder>>("dual_embedder");

        // For each chunk, extract the text to embed and determine signals.
        //
        // Chunks carry `_text`, `_kb_name`, and `_text_hash` in their data
        // (set by KBChunkNode / generate_chunk_records). The KB name determines
        // the embedding column name (`{kb}_embedding`) and the search signals
        // (vector / sparse / dual).
        let mut dense_works: Vec<EmbedWork> = Vec::new();
        let mut sparse_works: Vec<EmbedWork> = Vec::new();
        let mut dual_works: Vec<EmbedWork> = Vec::new();

        for rec in &mut items {
            let uuid = rec
                .entity_ref
                .ready()
                .await
                .map_err(|e| format!("embed ref resolution failed: {e}"))?;

            // Extract chunk fields
            let embed_text = match rec.data.get("_text").and_then(|v| v.as_str()) {
                Some(t) if !t.is_empty() => t.to_string(),
                _ => continue, // no text to embed
            };
            let kb_name = match rec.data.get("_kb_name").and_then(|v| v.as_str()) {
                Some(k) => k.to_string(),
                None => continue,
            };
            let text_hash = rec.data.get("_text_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| content_hash(&embed_text));

            let kb_config = config.knowledge_bases.get(&kb_name);
            let kb_signals = kb_config
                .map(|c| c.signals)
                .unwrap_or(search::SearchSignals::HYBRID);
            let kb_sparse = kb_signals.sparse() && has_sparse_svc;

            if has_dual_svc && kb_signals.vector() && kb_sparse && dual_embedder.is_some() {
                dual_works.push(EmbedWork {
                    uuid: uuid.clone(),
                    text: embed_text,
                    text_hash,
                    entity_name: rec.entity_name.clone(),
                    kb_name,
                });
            } else {
                if kb_signals.vector() {
                    dense_works.push(EmbedWork {
                        uuid: uuid.clone(),
                        text: embed_text.clone(),
                        text_hash: text_hash.clone(),
                        entity_name: rec.entity_name.clone(),
                        kb_name: kb_name.clone(),
                    });
                }
                if kb_sparse && sparse_embedder.is_some() {
                    sparse_works.push(EmbedWork {
                        uuid: uuid.clone(),
                        text: embed_text,
                        text_hash,
                        entity_name: rec.entity_name.clone(),
                        kb_name,
                    });
                }
            }
        }

        // ── Idempotence: skip chunks whose text hasn't changed ──
        // Each chunk has _text_hash (set at insertion) and _embed_hash (set when
        // embedding is written). If _embed_hash == _text_hash the embedding is
        // up-to-date. If _embed_hash is NULL the chunk was never embedded (e.g.
        // crash recovery). This two-field design provides granular persistence:
        // insertion and embedding are independently resumable.
        let all_uuids: HashSet<&str> = dense_works.iter()
            .chain(sparse_works.iter())
            .chain(dual_works.iter())
            .map(|w| w.uuid.as_str())
            .collect();

        let mut existing_hashes: HashMap<String, String> = HashMap::new();
        if !all_uuids.is_empty() {
            // Group by entity_name for efficient UNWIND queries
            let mut by_entity: HashMap<&str, Vec<&str>> = HashMap::new();
            for w in dense_works.iter().chain(sparse_works.iter()).chain(dual_works.iter()) {
                by_entity.entry(&w.entity_name).or_default().push(&w.uuid);
            }
            for (entity_name, uuids) in &by_entity {
                let unique: HashSet<&&str> = uuids.iter().collect();
                let items_param = CypherValue::List(
                    unique.iter().map(|&&u| {
                        let mut m = BTreeMap::new();
                        m.insert("uuid".into(), CypherValue::String(u.to_string()));
                        CypherValue::Map(m)
                    }).collect()
                );
                let cypher = dialect.embed_check_hashes(entity_name);
                if let Ok(result) = conn.execute_with_params(
                    &cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                ).await {
                    for row in &result.rows {
                        if let (Some(uuid), Some(hash)) = (
                            row.first().and_then(|v| v.as_str()),
                            row.get(1).and_then(|v| v.as_str()),
                        ) {
                            existing_hashes.insert(uuid.to_string(), hash.to_string());
                        }
                    }
                }
            }
        }

        let is_changed = |w: &EmbedWork| -> bool {
            match existing_hashes.get(&w.uuid) {
                Some(existing) => existing != &w.text_hash,
                None => true, // no _embed_hash = never embedded, must embed
            }
        };

        let pre_filter = dense_works.len() + sparse_works.len() + dual_works.len();
        dense_works.retain(is_changed);
        sparse_works.retain(is_changed);
        dual_works.retain(is_changed);
        let skipped = pre_filter - (dense_works.len() + sparse_works.len() + dual_works.len());

        ctx.log_metric("entities", items.len());
        ctx.log_metric("dense", dense_works.len());
        ctx.log_metric("sparse", sparse_works.len());
        ctx.log_metric("dual", dual_works.len());
        ctx.log_metric("skipped_unchanged", skipped);

        // ── Dense embedding ──
        if !dense_works.is_empty() {
            let texts: Vec<String> = dense_works.iter().map(|w| w.text.clone()).collect();
            let vectors = embedder
                .embed(&texts)
                .await
                .map_err(|e| format!("dense embedding failed: {e}"))?;

            if vectors.len() != dense_works.len() {
                return Err(format!(
                    "embedder returned {} vectors for {} texts",
                    vectors.len(), dense_works.len()
                ));
            }

            // Group by (entity_name, embedding_col)
            let mut groups: HashMap<(&str, String), Vec<(&EmbedWork, &Vec<f32>)>> = HashMap::new();
            for (work, vector) in dense_works.iter().zip(vectors.iter()) {
                if vector.len() != embedding_dim {
                    return Err(format!(
                        "embedding dimension mismatch: expected {}, got {}",
                        embedding_dim, vector.len()
                    ));
                }
                let col = format!("{}_embedding", work.kb_name);
                groups
                    .entry((&work.entity_name, col))
                    .or_default()
                    .push((work, vector));
            }

            for ((entity_name, col), group) in &groups {
                let items_param = CypherValue::List(
                    group.iter().map(|(work, vec)| {
                        let mut map = BTreeMap::new();
                        map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                        map.insert("hash".into(), CypherValue::String(work.text_hash.clone()));
                        map.insert("emb".into(), CypherValue::List(
                            vec.iter().map(|&f| CypherValue::Float(f as f64)).collect(),
                        ));
                        CypherValue::Map(map)
                    }).collect(),
                );

                let cypher = dialect.embed_set(entity_name, &col);

                conn.execute_with_params(
                    &cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                ).await.map_err(|e| e.to_string())?;
            }
        }

        // ── Sparse embedding → insert into SparseHandle ──
        let sparse_handles = ctx.service::<HashMap<String, Arc<sparse_vector::handle::SparseHandle>>>("sparse_handles");
        if !sparse_works.is_empty() {
            if let Some(ref sparse_emb) = sparse_embedder {
                let texts: Vec<String> = sparse_works.iter().map(|w| w.text.clone()).collect();
                let sparse_vecs = sparse_emb
                    .embed_sparse(&texts)
                    .await
                    .map_err(|e| format!("sparse embedding failed: {e}"))?;

                if sparse_vecs.len() != sparse_works.len() {
                    return Err(format!(
                        "sparse embedder returned {} vectors for {} texts",
                        sparse_vecs.len(), sparse_works.len()
                    ));
                }

                let mut groups: HashMap<(&str, &str), Vec<(&EmbedWork, &SparseVector)>> =
                    HashMap::new();
                for (work, sv) in sparse_works.iter().zip(sparse_vecs.iter()) {
                    groups
                        .entry((&work.entity_name, work.kb_name.as_str()))
                        .or_default()
                        .push((work, sv));
                }

                for ((entity_name, _kb_name), group) in &groups {
                    // Set _embed_hash + get offsets for handle.insert
                    let items_param = CypherValue::List(
                        group.iter().map(|(work, _)| {
                            let mut map = BTreeMap::new();
                            map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                            map.insert("hash".into(), CypherValue::String(work.text_hash.clone()));
                            CypherValue::Map(map)
                        }).collect(),
                    );

                    let cypher = dialect.embed_set_hash_returning_offset(entity_name);

                    let result = conn.execute_with_params(
                        &cypher,
                        &[QueryParam { name: "items".into(), value: items_param }],
                    ).await.map_err(|e| e.to_string())?;

                    // Insert into SparseHandle using offsets
                    if let Some(ref handles) = sparse_handles {
                        if let Some(handle) = handles.get(*entity_name) {
                            let uuid_to_sv: HashMap<&str, &SparseVector> = group.iter()
                                .map(|(w, sv)| (w.uuid.as_str(), *sv))
                                .collect();
                            for row in &result.rows {
                                if let (Some(uuid), Some(offset)) = (
                                    row.first().and_then(|v| v.as_str()),
                                    row.get(1).and_then(|v| v.as_i64()),
                                ) {
                                    if let Some(sv) = uuid_to_sv.get(uuid) {
                                        let sv_idx = sparse_vector::index::SparseVector::new(
                                            sv.indices.clone(), sv.values.clone(),
                                        );
                                        handle.insert(offset as u64, &sv_idx)
                                            .map_err(|e| format!("sparse insert failed: {e}"))?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Dual embedding (GPU mini-batches) ──
        if !dual_works.is_empty() {
            if let Some(ref dual_emb) = dual_embedder {
                let mut dense_results: Vec<(&EmbedWork, Vec<f32>)> = Vec::with_capacity(dual_works.len());
                let mut sparse_results: Vec<(&EmbedWork, SparseVector)> = Vec::with_capacity(dual_works.len());

                for chunk in dual_works.chunks(self.gpu_batch_size) {
                    let texts: Vec<String> = chunk.iter().map(|w| w.text.clone()).collect();
                    let (dense_vecs, sparse_vecs) = dual_emb
                        .embed_dual(&texts)
                        .await
                        .map_err(|e| format!("dual embed failed: {e}"))?;

                    if dense_vecs.len() != chunk.len() || sparse_vecs.len() != chunk.len() {
                        return Err(format!(
                            "dual embedder returned {}/{} vectors for {} texts",
                            dense_vecs.len(), sparse_vecs.len(), chunk.len()
                        ));
                    }

                    let base_idx = dense_results.len();
                    for (i, (dense, sparse)) in
                        dense_vecs.into_iter().zip(sparse_vecs.into_iter()).enumerate()
                    {
                        dense_results.push((&dual_works[base_idx + i], dense));
                        sparse_results.push((&dual_works[base_idx + i], sparse));
                    }
                }

                // UNWIND dense
                {
                    let mut groups: HashMap<(&str, String), Vec<(&EmbedWork, &Vec<f32>)>> = HashMap::new();
                    for (work, vec) in &dense_results {
                        if vec.len() != embedding_dim {
                            return Err(format!(
                                "embedding dimension mismatch: expected {}, got {}",
                                embedding_dim, vec.len()
                            ));
                        }
                        let col = format!("{}_embedding", work.kb_name);
                        groups.entry((&work.entity_name, col)).or_default().push((work, vec));
                    }

                    for ((entity_name, col), group) in &groups {
                        let items_param = CypherValue::List(
                            group.iter().map(|(work, vec)| {
                                let mut map = BTreeMap::new();
                                map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                                map.insert("hash".into(), CypherValue::String(work.text_hash.clone()));
                                map.insert("emb".into(), CypherValue::List(
                                    vec.iter().map(|&f| CypherValue::Float(f as f64)).collect(),
                                ));
                                CypherValue::Map(map)
                            }).collect(),
                        );

                        let cypher = dialect.embed_set(entity_name, &col);

                        conn.execute_with_params(
                            &cypher,
                            &[QueryParam { name: "items".into(), value: items_param }],
                        ).await.map_err(|e| e.to_string())?;
                    }
                }

                // Insert sparse into SparseHandle (offsets resolved via MATCH)
                {
                    let mut groups: HashMap<(&str, &str), Vec<(&EmbedWork, &SparseVector)>> =
                        HashMap::new();
                    for (work, sv) in &sparse_results {
                        groups
                            .entry((&work.entity_name, work.kb_name.as_str()))
                            .or_default()
                            .push((work, sv));
                    }

                    for ((entity_name, _kb_name), group) in &groups {
                        let items_param = CypherValue::List(
                            group.iter().map(|(work, _)| {
                                let mut map = BTreeMap::new();
                                map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                                CypherValue::Map(map)
                            }).collect(),
                        );

                        let cypher = dialect.embed_get_offset(entity_name);

                        let result = conn.execute_with_params(
                            &cypher,
                            &[QueryParam { name: "items".into(), value: items_param }],
                        ).await.map_err(|e| e.to_string())?;

                        if let Some(ref handles) = sparse_handles {
                            if let Some(handle) = handles.get(*entity_name) {
                                let uuid_to_sv: HashMap<&str, &SparseVector> = group.iter()
                                    .map(|(w, sv)| (w.uuid.as_str(), *sv))
                                    .collect();
                                for row in &result.rows {
                                    if let (Some(uuid), Some(offset)) = (
                                        row.first().and_then(|v| v.as_str()),
                                        row.get(1).and_then(|v| v.as_i64()),
                                    ) {
                                        if let Some(sv) = uuid_to_sv.get(uuid) {
                                            let sv_idx = sparse_vector::index::SparseVector::new(
                                                sv.indices.clone(), sv.values.clone(),
                                            );
                                            handle.insert(offset as u64, &sv_idx)
                                                .map_err(|e| format!("sparse insert failed: {e}"))?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Capture undo data: entity_name → [uuid] for all embedded uuids
        let mut undo_groups: HashMap<&str, Vec<&str>> = HashMap::new();
        for w in dense_works.iter().chain(sparse_works.iter()).chain(dual_works.iter()) {
            undo_groups.entry(&w.entity_name).or_default().push(&w.uuid);
        }
        let undo_map: HashMap<String, Vec<String>> = undo_groups.into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().map(|u| u.to_string()).collect()))
            .collect();
        if !undo_map.is_empty() {
            self.undo_data = Some(serde_json::json!(undo_map));
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<serde_json::Value> {
        self.undo_data.clone()
    }

    async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String> {
        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("KBEmbedNode undo: 'conn' service not registered")?;

        let groups = undo_ctx.as_object()
            .ok_or("KBEmbedNode undo: expected object")?;

        for (entity_name, uuids) in groups {
            let uuid_list: Vec<&str> = uuids.as_array()
                .ok_or("KBEmbedNode undo: expected array of uuids")?
                .iter()
                .filter_map(|v| v.as_str())
                .collect();

            if uuid_list.is_empty() { continue; }

            let uuid_params = CypherValue::List(
                uuid_list.iter().map(|u| CypherValue::String(u.to_string())).collect()
            );
            // Reset _embed_hash to trigger re-embedding on next ingestion
            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .ok_or("KBEmbedNode undo: 'dialect' service not registered")?;
            let cypher = dialect.batch_set_null(entity_name, "_embed_hash");
            conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "uuids".into(), value: uuid_params }],
            ).await.map_err(|e| format!("KBEmbedNode undo failed: {e}"))?;
        }
        Ok(())
    }
}

// ─── KBChunkRecordNode ────────────────────────────────────────────────────────

/// Parallel chunking: takes `Vec<EntityRecord>` (inserted entities), chunks
/// their content fields via rayon, outputs chunk entities + links.
///
/// **Input**: `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
/// **Output**: `done` — Empty, `chunks` — chunk entities, `chunk_links` — HAS_CHUNK relations
/// **Services**: `config`, `kb_metadata`, `chunker_cache`
pub struct KBChunkRecordNode {
    name: String,
}

impl KBChunkRecordNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Compute chunks for one entity, producing EntityRecords (chunks) and
    /// RelationRecords (HAS_CHUNK links). Pure CPU work — no DB queries.
    fn compute_chunks(
        entity_name: &str,
        parent_uuid: &str,
        entity_ref: &EntityRef,
        data: &BTreeMap<String, CypherValue>,
        config: &CatalogConfig,
        kb_metadata: &HashMap<String, KBMetadata>,
        chunker_cache: &HashMap<ChunkerConfig, Chunker>,
    ) -> (Vec<EntityRecord>, Vec<RelationRecord>) {
        use crate::schema::entity_has_chunks;

        let entity_def = match config.entities.get(entity_name) {
            Some(def) => def,
            None => return (vec![], vec![]),
        };
        if !entity_has_chunks(entity_def) {
            return (vec![], vec![]);
        }

        let kb_names: Vec<&String> = kb_metadata
            .iter()
            .filter(|(_, kb)| kb.entities.contains(entity_name))
            .map(|(name, _)| name)
            .collect();
        if kb_names.is_empty() {
            return (vec![], vec![]);
        }

        let mut chunk_entities: Vec<EntityRecord> = Vec::new();
        let mut chunk_relations: Vec<RelationRecord> = Vec::new();

        for kb_name in &kb_names {
            let kb_meta = match kb_metadata.get(*kb_name) {
                Some(kb) => kb,
                None => continue,
            };

            let chunking = &kb_meta.chunking;
            let chunker_key = ChunkerConfig {
                max_size: chunking.max_size,
                overlap: chunking.overlap,
                strategy: chunking.strategy.clone(),
            };
            let chunker = match chunker_cache.get(&chunker_key) {
                Some(c) => c,
                None => continue,
            };

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

                    let mut chunk_data = BTreeMap::new();
                    chunk_data.insert("_uuid".into(), CypherValue::String(c_uuid.clone()));
                    chunk_data.insert("_parent_uuid".into(), CypherValue::String(parent_uuid.to_string()));
                    chunk_data.insert("_parent_field".into(), CypherValue::String(field_name.clone()));
                    chunk_data.insert("_kb_name".into(), CypherValue::String(kb_name.to_string()));
                    chunk_data.insert("_text".into(), CypherValue::String(chunk.text.clone()));
                    chunk_data.insert("_text_hash".into(), CypherValue::String(content_hash(&chunk.text)));
                    chunk_data.insert("_index".into(), CypherValue::Int(chunk.index as i64));
                    chunk_data.insert("_start_char".into(), CypherValue::Int(chunk.start_byte as i64));
                    chunk_data.insert("_end_char".into(), CypherValue::Int(chunk.end_byte as i64));
                    chunk_data.insert("_start_line".into(), CypherValue::Int(chunk.start_line as i64));
                    chunk_data.insert("_end_line".into(), CypherValue::Int(chunk.end_line as i64));
                    chunk_data.insert("_core_start_char".into(), CypherValue::Int(chunk.core_start_byte as i64));
                    chunk_data.insert("_core_end_char".into(), CypherValue::Int(chunk.core_end_byte as i64));
                    chunk_data.insert("_core_start_line".into(), CypherValue::Int(chunk.core_start_line as i64));
                    chunk_data.insert("_core_end_line".into(), CypherValue::Int(chunk.core_end_line as i64));

                    let (chunk_ref, chunk_resolver) = EntityRef::new(&chunk_table);
                    chunk_resolver.resolve(c_uuid.clone());

                    chunk_entities.push(EntityRecord {
                        entity_name: chunk_table.clone(),
                        data: chunk_data,
                        entity_ref: chunk_ref,
                        resolver: None, // resolver already consumed above
                    });

                    let (link_ref, link_resolver) = RelationRef::new(&rel_name);
                    chunk_relations.push(RelationRecord {
                        rel_name: rel_name.clone(),
                        from: RefOrUuid::Ref(entity_ref.clone()),
                        to: RefOrUuid::Uuid(c_uuid),
                        properties: BTreeMap::new(),
                        relation_ref: link_ref,
                        resolver: Some(link_resolver),
                    });
                }
            }
        }

        (chunk_entities, chunk_relations)
    }
}

#[async_trait]
impl Node for KBChunkRecordNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "KBChunkRecordNode"
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "entities", port_type: PortType::Entities, required: true },
            PortDef { name: "trigger", port_type: PortType::Empty, required: false },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "done", port_type: PortType::Empty, required: false },
            PortDef { name: "chunks", port_type: PortType::Entities, required: false },
            PortDef { name: "chunk_links", port_type: PortType::Relations, required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        use rayon::prelude::*;

        let items: Vec<EntityRecord> = match ctx.take_input("entities") {
            Some(PortValue::Batch(payload)) => payload
                .take::<EntityRecord>()
                .ok_or("KBChunkRecordNode: failed to extract Vec<EntityRecord>")?,
            _ => return Err("KBChunkRecordNode: missing 'entities' input".to_string()),
        };

        let config = ctx.service::<CatalogConfig>("config")
            .ok_or("KBChunkRecordNode: 'config' service not registered")?;
        let kb_metadata = ctx.service::<HashMap<String, KBMetadata>>("kb_metadata")
            .ok_or("KBChunkRecordNode: 'kb_metadata' service not registered")?;
        let chunker_cache = ctx.service::<HashMap<ChunkerConfig, Chunker>>("chunker_cache")
            .ok_or("KBChunkRecordNode: 'chunker_cache' service not registered")?;

        // Parallel chunking via rayon
        let all_results: Vec<(Vec<EntityRecord>, Vec<RelationRecord>)> = items
            .par_iter()
            .map(|rec| {
                let parent_uuid = rec.data
                    .get("_uuid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Self::compute_chunks(
                    &rec.entity_name,
                    parent_uuid,
                    &rec.entity_ref,
                    &rec.data,
                    &config,
                    &kb_metadata,
                    &chunker_cache,
                )
            })
            .collect();

        let mut all_chunk_entities: Vec<EntityRecord> = Vec::new();
        let mut all_chunk_relations: Vec<RelationRecord> = Vec::new();
        for (entities, relations) in all_results {
            all_chunk_entities.extend(entities);
            all_chunk_relations.extend(relations);
        }

        ctx.log_metric("entities", items.len());
        ctx.log_metric("chunks", all_chunk_entities.len());
        ctx.log_metric("chunk_links", all_chunk_relations.len());

        ctx.set_output("done", PortValue::Empty);
        ctx.set_output("chunks", PortValue::Batch(
            BatchPayload::new(PortType::Entities, all_chunk_entities),
        ));
        ctx.set_output("chunk_links", PortValue::Batch(
            BatchPayload::new(PortType::Relations, all_chunk_relations),
        ));
        Ok(())
    }
}

// ─── ChunkRecordNode (simple entities) ──────────────────────────────────────

/// Parallel chunking for simple entities (registerEntity API).
/// Uses `entity_configs` (not `kb_metadata`) to find content fields.
///
/// **Input**: `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
/// **Output**: `done` — Empty, `chunks` — chunk entities, `chunk_links` — CHUNKED_FROM relations
/// **Services**: `config`, `entity_configs`, `chunker_cache`
pub struct ChunkRecordNode {
    name: String,
}

impl ChunkRecordNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Compute chunks for one simple entity, producing EntityRecords (chunks) and
    /// RelationRecords (CHUNKED_FROM links). Pure CPU work — no DB queries.
    fn compute_chunks(
        entity_name: &str,
        parent_uuid: &str,
        entity_ref: &EntityRef,
        data: &BTreeMap<String, CypherValue>,
        entity_configs: &HashMap<String, crate::config::EntityConfig>,
        chunker_cache: &HashMap<ChunkerConfig, Chunker>,
    ) -> (Vec<EntityRecord>, Vec<RelationRecord>) {
        let entity_config = match entity_configs.get(entity_name) {
            Some(cfg) => cfg,
            None => return (vec![], vec![]),
        };

        let content_fields = entity_config.content_fields();
        if content_fields.is_empty() {
            return (vec![], vec![]);
        }

        // Get chunker from cache
        let chunker_key = ChunkerConfig {
            max_size: entity_config.chunking.max_size,
            overlap: entity_config.chunking.overlap,
            strategy: entity_config.chunking.strategy.clone(),
        };
        let chunker = match chunker_cache.get(&chunker_key) {
            Some(c) => c,
            None => return (vec![], vec![]),
        };

        // Get title from parent entity
        let title = entity_config.title_field()
            .and_then(|f| data.get(f))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let chunk_table = format!("{entity_name}_Chunk");
        let rel_name = format!("{entity_name}_CHUNKED_FROM");

        let mut chunk_entities: Vec<EntityRecord> = Vec::new();
        let mut chunk_relations: Vec<RelationRecord> = Vec::new();

        // Track _content_offset: offset of each field in the concatenation of all content fields
        // Concatenation order = content_fields (sorted alphabetically)
        // Separator = "\n\n" (2 chars) between fields
        let mut content_offset: i64 = 0;

        for (field_idx, field_name) in content_fields.iter().enumerate() {
            let field_text = match data.get(*field_name).and_then(|v| v.as_str()) {
                Some(s) if !s.is_empty() => s,
                _ => {
                    // Empty field still advances offset (0 chars + separator)
                    if field_idx > 0 {
                        content_offset += 2; // "\n\n" separator
                    }
                    continue;
                },
            };

            // Add separator offset for fields after the first
            if field_idx > 0 {
                content_offset += 2; // "\n\n" separator
            }

            let chunks = chunker.chunk(field_text);
            if chunks.is_empty() {
                content_offset += field_text.len() as i64;
                continue;
            }

            for chunk in &chunks {
                let c_uuid = chunk_uuid(parent_uuid, field_name, chunk.index);

                let mut chunk_data = BTreeMap::new();
                chunk_data.insert("_uuid".into(), CypherValue::String(c_uuid.clone()));
                chunk_data.insert("_parent_uuid".into(), CypherValue::String(parent_uuid.to_string()));
                chunk_data.insert("_parent_field".into(), CypherValue::String(field_name.to_string()));
                chunk_data.insert("_text".into(), CypherValue::String(chunk.text.clone()));
                chunk_data.insert("_title".into(), CypherValue::String(title.clone()));
                chunk_data.insert("_text_hash".into(), CypherValue::String(content_hash(&chunk.text)));
                chunk_data.insert("_embed_hash".into(), CypherValue::String(String::new()));
                chunk_data.insert("_index".into(), CypherValue::Int(chunk.index as i64));
                chunk_data.insert("_start_char".into(), CypherValue::Int(chunk.start_byte as i64));
                chunk_data.insert("_end_char".into(), CypherValue::Int(chunk.end_byte as i64));
                chunk_data.insert("_start_line".into(), CypherValue::Int(chunk.start_line as i64));
                chunk_data.insert("_end_line".into(), CypherValue::Int(chunk.end_line as i64));
                chunk_data.insert("_core_start_char".into(), CypherValue::Int(chunk.core_start_byte as i64));
                chunk_data.insert("_core_end_char".into(), CypherValue::Int(chunk.core_end_byte as i64));
                chunk_data.insert("_core_start_line".into(), CypherValue::Int(chunk.core_start_line as i64));
                chunk_data.insert("_core_end_line".into(), CypherValue::Int(chunk.core_end_line as i64));
                chunk_data.insert("_content_offset".into(), CypherValue::Int(content_offset));

                let (chunk_ref, chunk_resolver) = EntityRef::new(&chunk_table);
                chunk_resolver.resolve(c_uuid.clone());

                chunk_entities.push(EntityRecord {
                    entity_name: chunk_table.clone(),
                    data: chunk_data,
                    entity_ref: chunk_ref,
                    resolver: None,
                });

                let (link_ref, link_resolver) = RelationRef::new(&rel_name);
                chunk_relations.push(RelationRecord {
                    rel_name: rel_name.clone(),
                    from: RefOrUuid::Uuid(c_uuid),
                    to: RefOrUuid::Ref(entity_ref.clone()),
                    properties: BTreeMap::new(),
                    relation_ref: link_ref,
                    resolver: Some(link_resolver),
                });
            }

            content_offset += field_text.len() as i64;
        }

        (chunk_entities, chunk_relations)
    }
}

#[async_trait]
impl Node for ChunkRecordNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "ChunkRecordNode"
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "entities", port_type: PortType::Entities, required: true },
            PortDef { name: "trigger", port_type: PortType::Empty, required: false },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "done", port_type: PortType::Empty, required: false },
            PortDef { name: "chunks", port_type: PortType::Entities, required: false },
            PortDef { name: "chunk_links", port_type: PortType::Relations, required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        use rayon::prelude::*;

        let items: Vec<EntityRecord> = match ctx.take_input("entities") {
            Some(PortValue::Batch(payload)) => payload
                .take::<EntityRecord>()
                .ok_or("ChunkRecordNode: failed to extract Vec<EntityRecord>")?,
            _ => return Err("ChunkRecordNode: missing 'entities' input".to_string()),
        };

        let entity_configs = ctx.service::<HashMap<String, crate::config::EntityConfig>>("entity_configs")
            .ok_or("ChunkRecordNode: 'entity_configs' service not registered")?;
        let chunker_cache = ctx.service::<HashMap<ChunkerConfig, Chunker>>("chunker_cache")
            .ok_or("ChunkRecordNode: 'chunker_cache' service not registered")?;

        // Parallel chunking via rayon
        let all_results: Vec<(Vec<EntityRecord>, Vec<RelationRecord>)> = items
            .par_iter()
            .map(|rec| {
                let parent_uuid = rec.data
                    .get("_uuid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Self::compute_chunks(
                    &rec.entity_name,
                    parent_uuid,
                    &rec.entity_ref,
                    &rec.data,
                    &entity_configs,
                    &chunker_cache,
                )
            })
            .collect();

        let mut all_chunk_entities: Vec<EntityRecord> = Vec::new();
        let mut all_chunk_relations: Vec<RelationRecord> = Vec::new();
        for (entities, relations) in all_results {
            all_chunk_entities.extend(entities);
            all_chunk_relations.extend(relations);
        }

        ctx.log_metric("entities", items.len());
        ctx.log_metric("chunks", all_chunk_entities.len());
        ctx.log_metric("chunk_links", all_chunk_relations.len());

        ctx.set_output("done", PortValue::Empty);
        ctx.set_output("chunks", PortValue::Batch(
            BatchPayload::new(PortType::Entities, all_chunk_entities),
        ));
        ctx.set_output("chunk_links", PortValue::Batch(
            BatchPayload::new(PortType::Relations, all_chunk_relations),
        ));
        Ok(())
    }

    // Read-only: nothing to undo, but must return true so rollback doesn't fail
    fn can_undo(&self) -> bool { true }
}

// ─── EmbedNode (simple entities) ────────────────────────────────────────────

/// Embedding node for simple entities (registerEntity API).
/// Configurable column names, signals on the node. No KB dependency.
///
/// **Input**: `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
/// **Output**: `done` — Empty
/// **Services**: `conn`, `embedder`, `embedding_dim`, optionally `sparse_embedder`, `dual_embedder`
pub struct EmbedNode {
    name: String,
    text_field: String,
    embedding_col: String,
    sparse_col: String,
    signals: search::SearchSignals,
    gpu_batch_size: usize,
    undo_data: Option<serde_json::Value>,
}

impl EmbedNode {
    pub fn new(
        name: impl Into<String>,
        signals: search::SearchSignals,
        gpu_batch_size: usize,
    ) -> Self {
        Self {
            name: name.into(),
            text_field: "_text".into(),
            embedding_col: "embedding".into(),
            sparse_col: "sparse".into(),
            signals,
            gpu_batch_size,
            undo_data: None,
        }
    }

    pub fn with_columns(
        mut self,
        text_field: impl Into<String>,
        embedding_col: impl Into<String>,
        sparse_col: impl Into<String>,
    ) -> Self {
        self.text_field = text_field.into();
        self.embedding_col = embedding_col.into();
        self.sparse_col = sparse_col.into();
        self
    }
}

/// Internal work item for simple embedding.
struct SimpleEmbedWork {
    uuid: String,
    text: String,
    text_hash: String,
    entity_name: String,
}

#[async_trait]
impl Node for EmbedNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "EmbedNode"
    }
    fn node_config(&self) -> serde_json::Value {
        serde_json::json!({
            "gpu_batch_size": self.gpu_batch_size,
            "text_field": self.text_field,
            "embedding_col": self.embedding_col,
            "sparse_col": self.sparse_col,
        })
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "entities", port_type: PortType::Entities, required: true },
            PortDef { name: "trigger", port_type: PortType::Empty, required: false },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "done",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let mut items: Vec<EntityRecord> = match ctx.take_input("entities") {
            Some(PortValue::Batch(payload)) => payload
                .take::<EntityRecord>()
                .ok_or("EmbedNode: failed to extract Vec<EntityRecord>")?,
            _ => return Err("EmbedNode: missing 'entities' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("EmbedNode: 'conn' service not registered")?;
        let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
            .ok_or("EmbedNode: 'dialect' service not registered")?;
        let embedder = ctx.service::<Arc<dyn Embedder>>("embedder")
            .ok_or("EmbedNode: 'embedder' service not registered")?;
        let embedding_dim = *ctx.service::<usize>("embedding_dim")
            .ok_or("EmbedNode: 'embedding_dim' service not registered")?;
        let has_sparse_svc = ctx.service::<bool>("has_sparse").map(|v| *v).unwrap_or(false);
        let has_dual_svc = ctx.service::<bool>("has_dual").map(|v| *v).unwrap_or(false);
        let sparse_embedder = ctx.service::<Arc<dyn SparseEmbedder>>("sparse_embedder");
        let dual_embedder = ctx.service::<Arc<dyn DualEmbedder>>("dual_embedder");

        // Per-entity signals: read from entity_configs if available, fallback to self.signals.
        let entity_configs = ctx.service::<HashMap<String, crate::config::EntityConfig>>("entity_configs");
        let resolve_signals = |entity_name: &str| -> search::SearchSignals {
            entity_configs.as_ref()
                .and_then(|cfgs| cfgs.get(entity_name))
                .map(|ec| ec.signals)
                .unwrap_or(self.signals)
        };

        // Build work items from chunk entities, grouped by per-entity signals
        let mut dense_works: Vec<SimpleEmbedWork> = Vec::new();
        let mut sparse_works: Vec<SimpleEmbedWork> = Vec::new();
        let mut dual_works: Vec<SimpleEmbedWork> = Vec::new();

        for rec in &mut items {
            let uuid = rec
                .entity_ref
                .ready()
                .await
                .map_err(|e| format!("embed ref resolution failed: {e}"))?;

            let embed_text = match rec.data.get(&self.text_field).and_then(|v| v.as_str()) {
                Some(t) if !t.is_empty() => t.to_string(),
                _ => continue,
            };
            let text_hash = rec.data.get("_text_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| content_hash(&embed_text));

            // Resolve signals for the parent entity (chunk entity_name is "{Parent}_Chunk")
            let parent_entity = rec.entity_name.strip_suffix("_Chunk").unwrap_or(&rec.entity_name);
            let signals = resolve_signals(parent_entity);
            let want_vector = signals.vector();
            let want_sparse = signals.sparse() && has_sparse_svc;

            if has_dual_svc && want_vector && want_sparse && dual_embedder.is_some() {
                dual_works.push(SimpleEmbedWork {
                    uuid: uuid.clone(),
                    text: embed_text,
                    text_hash,
                    entity_name: rec.entity_name.clone(),
                });
            } else {
                if want_vector {
                    dense_works.push(SimpleEmbedWork {
                        uuid: uuid.clone(),
                        text: embed_text.clone(),
                        text_hash: text_hash.clone(),
                        entity_name: rec.entity_name.clone(),
                    });
                }
                if want_sparse && sparse_embedder.is_some() {
                    sparse_works.push(SimpleEmbedWork {
                        uuid: uuid.clone(),
                        text: embed_text,
                        text_hash,
                        entity_name: rec.entity_name.clone(),
                    });
                }
            }
        }

        // ── Idempotence: skip chunks whose text hasn't changed ──
        let all_uuids: HashSet<&str> = dense_works.iter()
            .chain(sparse_works.iter())
            .chain(dual_works.iter())
            .map(|w| w.uuid.as_str())
            .collect();

        let mut existing_hashes: HashMap<String, String> = HashMap::new();
        if !all_uuids.is_empty() {
            let mut by_entity: HashMap<&str, Vec<&str>> = HashMap::new();
            for w in dense_works.iter().chain(sparse_works.iter()).chain(dual_works.iter()) {
                by_entity.entry(&w.entity_name).or_default().push(&w.uuid);
            }
            for (entity_name, uuids) in &by_entity {
                let unique: HashSet<&&str> = uuids.iter().collect();
                let items_param = CypherValue::List(
                    unique.iter().map(|&&u| {
                        let mut m = BTreeMap::new();
                        m.insert("uuid".into(), CypherValue::String(u.to_string()));
                        CypherValue::Map(m)
                    }).collect()
                );
                let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                    .ok_or("EmbedNode: 'dialect' service not registered")?;
                let cypher = dialect.embed_check_hashes(entity_name);
                if let Ok(result) = conn.execute_with_params(
                    &cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                ).await {
                    for row in &result.rows {
                        if let (Some(uuid), Some(hash)) = (
                            row.first().and_then(|v| v.as_str()),
                            row.get(1).and_then(|v| v.as_str()),
                        ) {
                            existing_hashes.insert(uuid.to_string(), hash.to_string());
                        }
                    }
                }
            }
        }

        let is_changed = |w: &SimpleEmbedWork| -> bool {
            match existing_hashes.get(&w.uuid) {
                Some(existing) => existing != &w.text_hash,
                None => true,
            }
        };

        let pre_filter = dense_works.len() + sparse_works.len() + dual_works.len();
        dense_works.retain(is_changed);
        sparse_works.retain(is_changed);
        dual_works.retain(is_changed);
        let skipped = pre_filter - (dense_works.len() + sparse_works.len() + dual_works.len());

        ctx.log_metric("entities", items.len());
        ctx.log_metric("dense", dense_works.len());
        ctx.log_metric("sparse", sparse_works.len());
        ctx.log_metric("dual", dual_works.len());
        ctx.log_metric("skipped_unchanged", skipped);

        let embedding_col = &self.embedding_col;

        // ── Dense embedding (GPU mini-batches) ──
        if !dense_works.is_empty() {
            for chunk in dense_works.chunks(self.gpu_batch_size) {
                let texts: Vec<String> = chunk.iter().map(|w| w.text.clone()).collect();
                let vectors = embedder
                    .embed(&texts)
                    .await
                    .map_err(|e| format!("dense embedding failed: {e}"))?;

                if vectors.len() != chunk.len() {
                    return Err(format!(
                        "embedder returned {} vectors for {} texts",
                        vectors.len(), chunk.len()
                    ));
                }

                // Group by entity_name for batch UNWIND
                let mut groups: HashMap<&str, Vec<(&SimpleEmbedWork, &Vec<f32>)>> = HashMap::new();
                for (work, vector) in chunk.iter().zip(vectors.iter()) {
                    if vector.len() != embedding_dim {
                        return Err(format!(
                            "embedding dimension mismatch: expected {}, got {}",
                            embedding_dim, vector.len()
                        ));
                    }
                    groups.entry(&work.entity_name).or_default().push((work, vector));
                }

                for (entity_name, group) in &groups {
                    let items_param = CypherValue::List(
                        group.iter().map(|(work, vec)| {
                            let mut map = BTreeMap::new();
                            map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                            map.insert("hash".into(), CypherValue::String(work.text_hash.clone()));
                            map.insert("emb".into(), CypherValue::List(
                                vec.iter().map(|&f| CypherValue::Float(f as f64)).collect(),
                            ));
                            CypherValue::Map(map)
                        }).collect(),
                    );

                    let cypher = dialect.embed_set(entity_name, &embedding_col);

                    conn.execute_with_params(
                        &cypher,
                        &[QueryParam { name: "items".into(), value: items_param }],
                    ).await.map_err(|e| e.to_string())?;
                }
            }
        }

        // ── Sparse embedding (GPU mini-batches) → insert into SparseHandle ──
        let sparse_handles = ctx.service::<HashMap<String, Arc<sparse_vector::handle::SparseHandle>>>("sparse_handles");
        if !sparse_works.is_empty() {
            if let Some(ref sparse_emb) = sparse_embedder {
                for chunk in sparse_works.chunks(self.gpu_batch_size) {
                    let texts: Vec<String> = chunk.iter().map(|w| w.text.clone()).collect();
                    let sparse_vecs = sparse_emb
                        .embed_sparse(&texts)
                        .await
                        .map_err(|e| format!("sparse embedding failed: {e}"))?;

                    if sparse_vecs.len() != chunk.len() {
                        return Err(format!(
                            "sparse embedder returned {} vectors for {} texts",
                            sparse_vecs.len(), chunk.len()
                        ));
                    }

                    let mut groups: HashMap<&str, Vec<(&SimpleEmbedWork, &SparseVector)>> =
                        HashMap::new();
                    for (work, sv) in chunk.iter().zip(sparse_vecs.iter()) {
                        groups.entry(&work.entity_name).or_default().push((work, sv));
                    }

                    for (entity_name, group) in &groups {
                        // Set _embed_hash + get offsets for handle.insert
                        let items_param = CypherValue::List(
                            group.iter().map(|(work, _)| {
                                let mut map = BTreeMap::new();
                                map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                                map.insert("hash".into(), CypherValue::String(work.text_hash.clone()));
                                CypherValue::Map(map)
                            }).collect(),
                        );

                        let cypher = dialect.embed_set_hash_returning_offset(entity_name);

                        let result = conn.execute_with_params(
                            &cypher,
                            &[QueryParam { name: "items".into(), value: items_param }],
                        ).await.map_err(|e| e.to_string())?;

                        // Insert into SparseHandle using offsets
                        if let Some(ref handles) = sparse_handles {
                            if let Some(handle) = handles.get(*entity_name) {
                                let uuid_to_sv: HashMap<&str, &SparseVector> = group.iter()
                                    .map(|(w, sv)| (w.uuid.as_str(), *sv))
                                    .collect();
                                for row in &result.rows {
                                    if let (Some(uuid), Some(offset)) = (
                                        row.first().and_then(|v| v.as_str()),
                                        row.get(1).and_then(|v| v.as_i64()),
                                    ) {
                                        if let Some(sv) = uuid_to_sv.get(uuid) {
                                            let sv_idx = sparse_vector::index::SparseVector::new(
                                                sv.indices.clone(), sv.values.clone(),
                                            );
                                            handle.insert(offset as u64, &sv_idx)
                                                .map_err(|e| format!("sparse insert failed: {e}"))?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Dual embedding (GPU mini-batches) ──
        if !dual_works.is_empty() {
            if let Some(ref dual_emb) = dual_embedder {
                let mut dense_results: Vec<(&SimpleEmbedWork, Vec<f32>)> = Vec::with_capacity(dual_works.len());
                let mut sparse_results: Vec<(&SimpleEmbedWork, SparseVector)> = Vec::with_capacity(dual_works.len());

                for chunk in dual_works.chunks(self.gpu_batch_size) {
                    let texts: Vec<String> = chunk.iter().map(|w| w.text.clone()).collect();
                    let (dense_vecs, sparse_vecs) = dual_emb
                        .embed_dual(&texts)
                        .await
                        .map_err(|e| format!("dual embed failed: {e}"))?;

                    if dense_vecs.len() != chunk.len() || sparse_vecs.len() != chunk.len() {
                        return Err(format!(
                            "dual embedder returned {}/{} vectors for {} texts",
                            dense_vecs.len(), sparse_vecs.len(), chunk.len()
                        ));
                    }

                    let base_idx = dense_results.len();
                    for (i, (dense, sparse)) in
                        dense_vecs.into_iter().zip(sparse_vecs.into_iter()).enumerate()
                    {
                        dense_results.push((&dual_works[base_idx + i], dense));
                        sparse_results.push((&dual_works[base_idx + i], sparse));
                    }
                }

                // UNWIND dense (sets embedding + _embed_hash)
                {
                    let mut groups: HashMap<&str, Vec<(&SimpleEmbedWork, &Vec<f32>)>> = HashMap::new();
                    for (work, vec) in &dense_results {
                        if vec.len() != embedding_dim {
                            return Err(format!(
                                "embedding dimension mismatch: expected {}, got {}",
                                embedding_dim, vec.len()
                            ));
                        }
                        groups.entry(&work.entity_name).or_default().push((work, vec));
                    }

                    for (entity_name, group) in &groups {
                        let items_param = CypherValue::List(
                            group.iter().map(|(work, vec)| {
                                let mut map = BTreeMap::new();
                                map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                                map.insert("hash".into(), CypherValue::String(work.text_hash.clone()));
                                map.insert("emb".into(), CypherValue::List(
                                    vec.iter().map(|&f| CypherValue::Float(f as f64)).collect(),
                                ));
                                CypherValue::Map(map)
                            }).collect(),
                        );

                        let cypher = dialect.embed_set(entity_name, &embedding_col);

                        conn.execute_with_params(
                            &cypher,
                            &[QueryParam { name: "items".into(), value: items_param }],
                        ).await.map_err(|e| e.to_string())?;
                    }
                }

                // Insert sparse into SparseHandle (offsets resolved via MATCH)
                {
                    let mut groups: HashMap<&str, Vec<(&SimpleEmbedWork, &SparseVector)>> =
                        HashMap::new();
                    for (work, sv) in &sparse_results {
                        groups.entry(&work.entity_name).or_default().push((work, sv));
                    }

                    for (entity_name, group) in &groups {
                        // Get offsets for each uuid
                        let items_param = CypherValue::List(
                            group.iter().map(|(work, _)| {
                                let mut map = BTreeMap::new();
                                map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                                CypherValue::Map(map)
                            }).collect(),
                        );

                        let cypher = dialect.embed_get_offset(entity_name);

                        let result = conn.execute_with_params(
                            &cypher,
                            &[QueryParam { name: "items".into(), value: items_param }],
                        ).await.map_err(|e| e.to_string())?;

                        if let Some(ref handles) = sparse_handles {
                            if let Some(handle) = handles.get(*entity_name) {
                                let uuid_to_sv: HashMap<&str, &SparseVector> = group.iter()
                                    .map(|(w, sv)| (w.uuid.as_str(), *sv))
                                    .collect();
                                for row in &result.rows {
                                    if let (Some(uuid), Some(offset)) = (
                                        row.first().and_then(|v| v.as_str()),
                                        row.get(1).and_then(|v| v.as_i64()),
                                    ) {
                                        if let Some(sv) = uuid_to_sv.get(uuid) {
                                            let sv_idx = sparse_vector::index::SparseVector::new(
                                                sv.indices.clone(), sv.values.clone(),
                                            );
                                            handle.insert(offset as u64, &sv_idx)
                                                .map_err(|e| format!("sparse insert failed: {e}"))?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Capture undo data
        let mut undo_groups: HashMap<&str, Vec<&str>> = HashMap::new();
        for w in dense_works.iter().chain(sparse_works.iter()).chain(dual_works.iter()) {
            undo_groups.entry(&w.entity_name).or_default().push(&w.uuid);
        }
        let undo_map: HashMap<String, Vec<String>> = undo_groups.into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().map(|u| u.to_string()).collect()))
            .collect();
        if !undo_map.is_empty() {
            self.undo_data = Some(serde_json::json!(undo_map));
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<serde_json::Value> {
        self.undo_data.clone()
    }

    async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String> {
        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("EmbedNode undo: 'conn' service not registered")?;

        let groups = undo_ctx.as_object()
            .ok_or("EmbedNode undo: expected object")?;

        for (entity_name, uuids) in groups {
            let uuid_list: Vec<&str> = uuids.as_array()
                .ok_or("EmbedNode undo: expected array of uuids")?
                .iter()
                .filter_map(|v| v.as_str())
                .collect();

            if uuid_list.is_empty() { continue; }

            let uuid_params = CypherValue::List(
                uuid_list.iter().map(|u| CypherValue::String(u.to_string())).collect()
            );
            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .ok_or("EmbedNode undo: 'dialect' service not registered")?;
            let cypher = dialect.batch_set_null(entity_name, "_embed_hash");
            conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "uuids".into(), value: uuid_params }],
            ).await.map_err(|e| format!("EmbedNode undo failed: {e}"))?;
        }
        Ok(())
    }
}

// ─── KB Pipeline Nodes ──────────────────────────────────────────────────────
//
// These 3 nodes replace the monolithic AggregateRecordNode.
// Pipeline: KBGatherNode → KBUpdateNode → KBChunkNode
//
// - KBGatherNode: read titles + linked content + hashes, detect changes
// - KBUpdateNode: SET on KB_Index + DETACH DELETE stale chunks (pass-through)
// - KBChunkNode: generate chunk EntityRecords + RelationRecords (pure CPU)

/// Batch-collected state for one aggregate operation (internal to KBGatherNode).
struct RecordAggState {
    source_uuid: String,
    index_entry_uuid: String,
    title_text: String,
    sources: Vec<RecordSourceContent>,
    current_hash: String,
    found: bool,
}

/// Find the relation connecting title_entity to content_entity in config.
fn find_relation_to_entity(
    config: &CatalogConfig,
    title_entity: &str,
    content_entity: &str,
) -> Option<(String, bool)> {
    for (rel_name, rel_def) in &config.relations {
        if rel_def.from == title_entity && rel_def.to == content_entity {
            return Some((rel_name.clone(), true));
        }
        if rel_def.from == content_entity && rel_def.to == title_entity {
            return Some((rel_name.clone(), false));
        }
    }
    None
}

/// Generate chunk EntityRecords + RelationRecords from aggregated sources.
/// Pure CPU work — no DB queries.
fn generate_chunk_records(
    kb_name: &str,
    index_entry_uuid: &str,
    sources: &[RecordSourceContent],
    chunker: &Chunker,
    chunk_table: &str,
) -> (Vec<EntityRecord>, Vec<RelationRecord>) {
    let mut chunk_entities: Vec<EntityRecord> = Vec::new();
    let mut chunk_relations: Vec<RelationRecord> = Vec::new();
    let mut content_offset: usize = 0;

    for source in sources {
        let chunks = chunker.chunk(&source.text);
        for chunk in &chunks {
            let source_key = format!("{}:{}", source.entity_uuid, source.field_name);
            let c_uuid = chunk_uuid(index_entry_uuid, &source_key, chunk.index);

            let mut chunk_data = BTreeMap::new();
            chunk_data.insert("_uuid".into(), CypherValue::String(c_uuid.clone()));
            chunk_data.insert("_parent_uuid".into(), CypherValue::String(index_entry_uuid.to_string()));
            chunk_data.insert("_parent_field".into(), CypherValue::String(source.field_name.clone()));
            chunk_data.insert("_kb_name".into(), CypherValue::String(kb_name.to_string()));
            chunk_data.insert("_source_field".into(), CypherValue::String(source.field_name.clone()));
            chunk_data.insert("_source_entity".into(), CypherValue::String(source.entity_name.clone()));
            chunk_data.insert("_source_uuid".into(), CypherValue::String(source.entity_uuid.clone()));
            chunk_data.insert("_text".into(), CypherValue::String(chunk.text.clone()));
            chunk_data.insert("_text_hash".into(), CypherValue::String(content_hash(&chunk.text)));
            chunk_data.insert("_index".into(), CypherValue::Int(chunk.index as i64));
            chunk_data.insert("_start_char".into(), CypherValue::Int(chunk.start_byte as i64));
            chunk_data.insert("_end_char".into(), CypherValue::Int(chunk.end_byte as i64));
            chunk_data.insert("_start_line".into(), CypherValue::Int(chunk.start_line as i64));
            chunk_data.insert("_end_line".into(), CypherValue::Int(chunk.end_line as i64));
            chunk_data.insert("_core_start_char".into(), CypherValue::Int(chunk.core_start_byte as i64));
            chunk_data.insert("_core_end_char".into(), CypherValue::Int(chunk.core_end_byte as i64));
            chunk_data.insert("_core_start_line".into(), CypherValue::Int(chunk.core_start_line as i64));
            chunk_data.insert("_core_end_line".into(), CypherValue::Int(chunk.core_end_line as i64));
            chunk_data.insert("_content_offset".into(), CypherValue::Int(content_offset as i64));

            let (chunk_ref, chunk_resolver) = EntityRef::new(chunk_table);
            chunk_resolver.resolve(c_uuid.clone());

            chunk_entities.push(EntityRecord {
                entity_name: chunk_table.to_string(),
                data: chunk_data,
                entity_ref: chunk_ref,
                resolver: None,
            });

            // HAS_CHUNK link
            let has_chunk_rel = format!("{kb_name}_Index_HAS_CHUNK");
            let (link_ref, link_resolver) = RelationRef::new(&has_chunk_rel);
            chunk_relations.push(RelationRecord {
                rel_name: has_chunk_rel,
                from: RefOrUuid::Uuid(index_entry_uuid.to_string()),
                to: RefOrUuid::Uuid(c_uuid.clone()),
                properties: BTreeMap::new(),
                relation_ref: link_ref,
                resolver: Some(link_resolver),
            });

            // SOURCED link
            let sourced_rel = format!("{}_SOURCED_{kb_name}", source.entity_name);
            let (src_ref, src_resolver) = RelationRef::new(&sourced_rel);
            chunk_relations.push(RelationRecord {
                rel_name: sourced_rel,
                from: RefOrUuid::Uuid(source.entity_uuid.clone()),
                to: RefOrUuid::Uuid(c_uuid),
                properties: BTreeMap::new(),
                relation_ref: src_ref,
                resolver: Some(src_resolver),
            });
        }
        content_offset += source.text.len() + 1;
    }

    (chunk_entities, chunk_relations)
}

// ─── KBGatherNode ───────────────────────────────────────────────────────────

/// Reads title entities + linked content from DB, compares content hashes,
/// outputs only changed `KBContentRecord`s.
///
/// **Input**: `aggregates` — `BatchPayload<AggregateRecord>` (PortType::Aggregates)
/// **Output**: `kb_content` — changed records, `done` — Empty signal
/// **Services**: `conn`, `config`, `kb_metadata`
pub struct KBGatherNode {
    name: String,
}

impl KBGatherNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Process a batch of aggregates sharing the same (title_entity, kb_name).
    /// Steps 1-4: read titles, linked content, hashes, detect changes.
    async fn gather_batch(
        conn: &dyn DbConnection,
        config: &CatalogConfig,
        kb_meta: &KBMetadata,
        kb_name: &str,
        title_entity: &str,
        ops: &[&AggregateRecord],
    ) -> Result<(Vec<KBContentRecord>, usize, usize), String> {
        let index_table = format!("{kb_name}_Index");
        let title_field_name = &kb_meta.title.field;
        let mut n_queries: usize = 0;

        let title_content_fields: Vec<&String> = kb_meta
            .content
            .iter()
            .filter(|c| c.entity == title_entity)
            .map(|c| &c.field)
            .collect();

        let mut states: Vec<RecordAggState> = ops.iter().map(|op| RecordAggState {
            source_uuid: op.source_uuid.clone(),
            index_entry_uuid: op.index_entry_uuid.clone(),
            title_text: String::new(),
            sources: Vec::new(),
            current_hash: String::new(),
            found: false,
        }).collect();

        let uuid_to_idx: HashMap<String, usize> = states.iter()
            .enumerate()
            .map(|(i, s)| (s.source_uuid.clone(), i))
            .collect();
        let idx_uuid_to_idx: HashMap<String, usize> = states.iter()
            .enumerate()
            .map(|(i, s)| (s.index_entry_uuid.clone(), i))
            .collect();

        // ── Step 1: UNWIND read all titles ──
        {
            let items_param = CypherValue::List(
                states.iter().map(|s| {
                    let mut m = BTreeMap::new();
                    m.insert("uuid".into(), CypherValue::String(s.source_uuid.clone()));
                    CypherValue::Map(m)
                }).collect()
            );

            let mut return_fields = vec![
                "item.uuid AS _source_uuid".to_string(),
                format!("e.{title_field_name} AS _title_val"),
            ];
            for f in &title_content_fields {
                return_fields.push(format!("e.{f} AS {f}"));
            }

            let cypher = format!(
                "UNWIND $items AS item \
                 MATCH (e:{title_entity} {{_uuid: item.uuid}}) \
                 RETURN {}", return_fields.join(", ")
            );

            let result = conn
                .execute_with_params(
                    &cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                )
                .await
                .map_err(|e| e.to_string())?;
            n_queries += 1;

            let title_max_chars = kb_meta.chunking.title_max_chars;

            for row in &result.rows {
                let source_uuid = row[0].as_str().unwrap_or("");
                if let Some(&idx) = uuid_to_idx.get(source_uuid) {
                    states[idx].found = true;
                    let raw_title = row[1].as_str().unwrap_or("");
                    states[idx].title_text = if title_max_chars > 0 && raw_title.len() > title_max_chars {
                        raw_title.chars().take(title_max_chars).collect()
                    } else {
                        raw_title.to_string()
                    };

                    for (fi, f) in title_content_fields.iter().enumerate() {
                        let text = row.get(fi + 2).and_then(|v| v.as_str()).unwrap_or("");
                        if !text.is_empty() {
                            states[idx].sources.push(RecordSourceContent {
                                entity_name: title_entity.to_string(),
                                entity_uuid: source_uuid.to_string(),
                                field_name: f.to_string(),
                                text: text.to_string(),
                            });
                        }
                    }
                }
            }
        }

        // ── Step 2: UNWIND read linked content ──
        let other_content_entities: HashSet<&str> = kb_meta
            .content
            .iter()
            .map(|c| c.entity.as_str())
            .filter(|e| *e != title_entity)
            .collect();

        for content_entity_name in &other_content_entities {
            let relation = find_relation_to_entity(config, title_entity, content_entity_name);
            if let Some((rel_name, is_forward)) = relation {
                let entity_fields: Vec<&String> = kb_meta
                    .content
                    .iter()
                    .filter(|c| c.entity == *content_entity_name)
                    .map(|c| &c.field)
                    .collect();
                if entity_fields.is_empty() { continue; }

                let items_param = CypherValue::List(
                    states.iter().map(|s| {
                        let mut m = BTreeMap::new();
                        m.insert("uuid".into(), CypherValue::String(s.source_uuid.clone()));
                        CypherValue::Map(m)
                    }).collect()
                );

                let mut fields_return = vec![
                    "item.uuid AS _source_uuid".to_string(),
                    "c._uuid AS _content_uuid".to_string(),
                ];
                for f in &entity_fields {
                    fields_return.push(format!("c.{f} AS {f}"));
                }

                let cypher = if is_forward {
                    format!(
                        "UNWIND $items AS item \
                         MATCH (t:{title_entity} {{_uuid: item.uuid}})-[:{rel_name}]->(c:{content_entity_name}) \
                         RETURN {}", fields_return.join(", ")
                    )
                } else {
                    format!(
                        "UNWIND $items AS item \
                         MATCH (t:{title_entity} {{_uuid: item.uuid}})<-[:{rel_name}]-(c:{content_entity_name}) \
                         RETURN {}", fields_return.join(", ")
                    )
                };

                let result = conn
                    .execute_with_params(
                        &cypher,
                        &[QueryParam { name: "items".into(), value: items_param }],
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                n_queries += 1;

                for row in &result.rows {
                    let source_uuid = row[0].as_str().unwrap_or("");
                    let entity_uuid = row[1].as_str().unwrap_or("").to_string();
                    if let Some(&idx) = uuid_to_idx.get(source_uuid) {
                        for (fi, f) in entity_fields.iter().enumerate() {
                            let text = row.get(fi + 2).and_then(|v| v.as_str()).unwrap_or("");
                            if !text.is_empty() {
                                states[idx].sources.push(RecordSourceContent {
                                    entity_name: content_entity_name.to_string(),
                                    entity_uuid: entity_uuid.clone(),
                                    field_name: f.to_string(),
                                    text: text.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // ── Step 3: UNWIND read current hashes ──
        {
            let items_param = CypherValue::List(
                states.iter().map(|s| {
                    let mut m = BTreeMap::new();
                    m.insert("uuid".into(), CypherValue::String(s.index_entry_uuid.clone()));
                    CypherValue::Map(m)
                }).collect()
            );

            let cypher = format!(
                "UNWIND $items AS item \
                 MATCH (idx:{index_table} {{_uuid: item.uuid}}) \
                 RETURN item.uuid AS _idx_uuid, idx._content_hash AS _hash"
            );

            let result = conn
                .execute_with_params(
                    &cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                )
                .await
                .map_err(|e| e.to_string())?;
            n_queries += 1;

            for row in &result.rows {
                let idx_uuid = row[0].as_str().unwrap_or("");
                let hash = row[1].as_str().unwrap_or("");
                if let Some(&idx) = idx_uuid_to_idx.get(idx_uuid) {
                    states[idx].current_hash = hash.to_string();
                }
            }
        }

        // ── Step 4: Compute hashes, determine changed set ──
        let mut changed: Vec<KBContentRecord> = Vec::new();
        let mut skipped: usize = 0;

        for state in &mut states {
            if !state.found {
                skipped += 1;
                continue;
            }

            state.sources.sort_by(|a, b| {
                a.entity_name.cmp(&b.entity_name)
                    .then(a.entity_uuid.cmp(&b.entity_uuid))
                    .then(a.field_name.cmp(&b.field_name))
            });

            let content_text = state.sources
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let new_hash = content_hash(&format!("{}\n{content_text}", state.title_text));

            if state.current_hash == new_hash {
                skipped += 1;
                continue;
            }

            changed.push(KBContentRecord {
                index_entry_uuid: state.index_entry_uuid.clone(),
                kb_name: kb_name.to_string(),
                source_entity: title_entity.to_string(),
                source_uuid: state.source_uuid.clone(),
                title_text: state.title_text.clone(),
                content_text,
                new_hash,
                sources: std::mem::take(&mut state.sources),
            });
        }

        Ok((changed, skipped, n_queries))
    }
}

#[async_trait]
impl Node for KBGatherNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "KBGatherNode"
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "aggregates", port_type: PortType::Aggregates, required: false },
            PortDef { name: "trigger", port_type: PortType::Empty, required: false },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "kb_content", port_type: PortType::KBContent, required: false },
            PortDef { name: "done", port_type: PortType::Empty, required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        // Read from port (optional — might be connected to initial input)
        let mut items: Vec<AggregateRecord> = match ctx.take_input("aggregates") {
            Some(PortValue::Batch(payload)) => payload
                .take::<AggregateRecord>()
                .unwrap_or_default(),
            _ => vec![],
        };

        // Also read from shared service (populated by DeleteRecordNode + UpdateRecordNode)
        if let Some(pending_agg) = ctx.service::<Mutex<Vec<AggregateRecord>>>("pending_aggregates") {
            let mut service_items = pending_agg.lock().map_err(|e| format!("lock: {e}"))?;
            items.append(&mut *service_items);
        }

        if items.is_empty() {
            ctx.set_output("done", PortValue::Empty);
            ctx.set_output("kb_content", PortValue::Batch(
                BatchPayload::new(PortType::KBContent, Vec::<KBContentRecord>::new()),
            ));
            return Ok(());
        }

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("KBGatherNode: 'conn' service not registered")?;
        let config = ctx.service::<CatalogConfig>("config")
            .ok_or("KBGatherNode: 'config' service not registered")?;
        let kb_metadata = ctx.service::<HashMap<String, KBMetadata>>("kb_metadata")
            .ok_or("KBGatherNode: 'kb_metadata' service not registered")?;

        // Deduplicate by index_entry_uuid
        let mut seen = HashSet::new();
        let mut unique_ops: Vec<&AggregateRecord> = Vec::new();
        for agg in &items {
            if seen.insert(agg.index_entry_uuid.clone()) {
                unique_ops.push(agg);
            }
        }

        // Group by (title_entity, kb_name)
        let mut groups: HashMap<(&str, &str), Vec<&AggregateRecord>> = HashMap::new();
        for agg in &unique_ops {
            groups
                .entry((&agg.title_entity, &agg.kb_name))
                .or_default()
                .push(agg);
        }

        let mut all_changed: Vec<KBContentRecord> = Vec::new();
        let mut total_skipped: usize = 0;
        let mut total_queries: usize = 0;

        for ((_title_entity, kb_name), group_ops) in &groups {
            let kb_meta = match kb_metadata.get(*kb_name) {
                Some(m) => m,
                None => continue,
            };

            let (changed, skipped, n_queries) = Self::gather_batch(
                &**conn, &config, kb_meta, kb_name, _title_entity, &group_ops,
            ).await?;

            total_skipped += skipped;
            total_queries += n_queries;
            all_changed.extend(changed);
        }

        ctx.log_metric("ops", items.len());
        ctx.log_metric("unique_ops", seen.len());
        ctx.log_metric("groups", groups.len());
        ctx.log_metric("group_summary", groups.iter()
            .map(|((te, kb), ops)| format!("{te}@{kb}×{}", ops.len()))
            .collect::<Vec<_>>());
        ctx.log_metric("queries", total_queries);
        ctx.log_metric("skipped", total_skipped);
        ctx.log_metric("changed", all_changed.len());

        ctx.set_output("done", PortValue::Empty);
        ctx.set_output("kb_content", PortValue::Batch(
            BatchPayload::new(PortType::KBContent, all_changed),
        ));
        Ok(())
    }

    // Read-only: nothing to undo, but must return true so rollback doesn't fail
    fn can_undo(&self) -> bool { true }
}

// ─── KBUpdateNode ───────────────────────────────────────────────────────────

/// Updates `{KB}_Index` entries (SET _title, _content, _content_hash)
/// and deletes stale `{KB}_Index_Chunk` entities. Passes through
/// the input `KBContentRecord`s unchanged for downstream chunking.
///
/// **Input**: `kb_content` — `BatchPayload<KBContentRecord>` (PortType::KBContent)
/// **Output**: `kb_content` — same records (pass-through), `done` — Empty signal
/// **Services**: `conn`
pub struct KBUpdateNode {
    name: String,
    undo_data: Option<serde_json::Value>,
}

impl KBUpdateNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), undo_data: None }
    }
}

#[async_trait]
impl Node for KBUpdateNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "KBUpdateNode"
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "kb_content", port_type: PortType::KBContent, required: true },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "kb_content", port_type: PortType::KBContent, required: false },
            PortDef { name: "done", port_type: PortType::Empty, required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let items: Vec<KBContentRecord> = match ctx.take_input("kb_content") {
            Some(PortValue::Batch(payload)) => payload
                .take::<KBContentRecord>()
                .ok_or("KBUpdateNode: failed to extract Vec<KBContentRecord>")?,
            _ => return Err("KBUpdateNode: missing 'kb_content' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("KBUpdateNode: 'conn' service not registered")?;

        // Group by kb_name for UNWIND batching
        let mut groups: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, rec) in items.iter().enumerate() {
            groups.entry(&rec.kb_name).or_default().push(i);
        }

        let mut total_updated: usize = 0;
        let mut total_deleted: usize = 0;

        // Capture old values for undo before updating
        let mut undo_entries: Vec<serde_json::Value> = Vec::new();

        for (kb_name, indices) in &groups {
            let index_table = format!("{kb_name}_Index");
            let chunk_table = format!("{kb_name}_Index_Chunk");

            // Capture old values before update
            {
                let items_param = CypherValue::List(
                    indices.iter().map(|&i| {
                        let mut m = BTreeMap::new();
                        m.insert("uuid".into(), CypherValue::String(items[i].index_entry_uuid.clone()));
                        CypherValue::Map(m)
                    }).collect()
                );
                let cypher = format!(
                    "UNWIND $items AS item \
                     MATCH (idx:{index_table} {{_uuid: item.uuid}}) \
                     RETURN idx._uuid, idx._title, idx._content, idx._content_hash"
                );
                if let Ok(result) = conn.execute_with_params(
                    &cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                ).await {
                    for row in &result.rows {
                        let uuid = row[0].as_str().unwrap_or("").to_string();
                        let title = row[1].as_str().unwrap_or("").to_string();
                        let content = row[2].as_str().unwrap_or("").to_string();
                        let hash = row[3].as_str().unwrap_or("").to_string();
                        undo_entries.push(serde_json::json!({
                            "kb_name": kb_name,
                            "uuid": uuid,
                            "title": title,
                            "content": content,
                            "hash": hash,
                        }));
                    }
                }
            }

            // Step 5: UNWIND MERGE changed indexes (creates if new, updates if existing)
            {
                let items_param = CypherValue::List(
                    indices.iter().map(|&i| {
                        let rec = &items[i];
                        let mut m = BTreeMap::new();
                        m.insert("uuid".into(), CypherValue::String(rec.index_entry_uuid.clone()));
                        m.insert("title".into(), CypherValue::String(rec.title_text.clone()));
                        m.insert("content".into(), CypherValue::String(rec.content_text.clone()));
                        m.insert("hash".into(), CypherValue::String(rec.new_hash.clone()));
                        m.insert("source_entity".into(), CypherValue::String(rec.source_entity.clone()));
                        m.insert("source_uuid".into(), CypherValue::String(rec.source_uuid.clone()));
                        CypherValue::Map(m)
                    }).collect()
                );

                let cypher = format!(
                    "UNWIND $items AS item \
                     MERGE (idx:{index_table} {{_uuid: item.uuid}}) \
                     ON CREATE SET idx._title = item.title, idx._content = item.content, \
                     idx._content_hash = item.hash, idx._source_entity = item.source_entity, \
                     idx._source_uuid = item.source_uuid \
                     ON MATCH SET idx._title = item.title, idx._content = item.content, \
                     idx._content_hash = item.hash"
                );

                conn.execute_with_params(
                    &cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                ).await.map_err(|e| e.to_string())?;
                total_updated += indices.len();
            }

            // Step 5b: MERGE {Entity}_IN_{KB} relations for newly created indexes
            {
                // Group by source_entity for the IN relation
                let mut entity_groups: HashMap<&str, Vec<usize>> = HashMap::new();
                for &i in indices {
                    entity_groups.entry(&items[i].source_entity).or_default().push(i);
                }
                for (source_entity, eidx) in &entity_groups {
                    let in_rel = format!("{source_entity}_IN_{kb_name}");
                    let items_param = CypherValue::List(
                        eidx.iter().map(|&i| {
                            let rec = &items[i];
                            let mut m = BTreeMap::new();
                            m.insert("source_uuid".into(), CypherValue::String(rec.source_uuid.clone()));
                            m.insert("index_uuid".into(), CypherValue::String(rec.index_entry_uuid.clone()));
                            CypherValue::Map(m)
                        }).collect()
                    );
                    let cypher = format!(
                        "UNWIND $items AS item \
                         MATCH (e:{source_entity} {{_uuid: item.source_uuid}}) \
                         MATCH (idx:{index_table} {{_uuid: item.index_uuid}}) \
                         MERGE (e)-[:{in_rel}]->(idx)"
                    );
                    let _ = conn.execute_with_params(
                        &cypher,
                        &[QueryParam { name: "items".into(), value: items_param }],
                    ).await;
                }
            }

            // Step 6: UNWIND DELETE old chunks
            {
                let items_param = CypherValue::List(
                    indices.iter().map(|&i| {
                        let mut m = BTreeMap::new();
                        m.insert("uuid".into(), CypherValue::String(items[i].index_entry_uuid.clone()));
                        CypherValue::Map(m)
                    }).collect()
                );

                let cypher = format!(
                    "UNWIND $items AS item \
                     MATCH (c:{chunk_table} {{_parent_uuid: item.uuid}}) \
                     DETACH DELETE c"
                );

                let _ = conn.execute_with_params(
                    &cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                ).await;
                total_deleted += indices.len();
            }
        }

        if !undo_entries.is_empty() {
            self.undo_data = Some(serde_json::json!(undo_entries));
        }

        ctx.log_metric("items", items.len());
        ctx.log_metric("updated", total_updated);
        ctx.log_metric("deleted", total_deleted);

        ctx.set_output("done", PortValue::Empty);
        ctx.set_output("kb_content", PortValue::Batch(
            BatchPayload::new(PortType::KBContent, items),
        ));
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<serde_json::Value> {
        self.undo_data.clone()
    }

    async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String> {
        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("KBUpdateNode undo: 'conn' service not registered")?;

        let entries = undo_ctx.as_array()
            .ok_or("KBUpdateNode undo: expected array of entries")?;

        // Group by kb_name for UNWIND
        let mut groups: HashMap<String, Vec<&serde_json::Value>> = HashMap::new();
        for entry in entries {
            let kb_name = entry["kb_name"].as_str().unwrap_or("").to_string();
            groups.entry(kb_name).or_default().push(entry);
        }

        for (kb_name, entries) in &groups {
            let index_table = format!("{kb_name}_Index");

            let items_param = CypherValue::List(
                entries.iter().map(|e| {
                    let mut m = BTreeMap::new();
                    m.insert("uuid".into(), CypherValue::String(e["uuid"].as_str().unwrap_or("").to_string()));
                    m.insert("title".into(), CypherValue::String(e["title"].as_str().unwrap_or("").to_string()));
                    m.insert("content".into(), CypherValue::String(e["content"].as_str().unwrap_or("").to_string()));
                    m.insert("hash".into(), CypherValue::String(e["hash"].as_str().unwrap_or("").to_string()));
                    CypherValue::Map(m)
                }).collect()
            );

            let cypher = format!(
                "UNWIND $items AS item \
                 MATCH (idx:{index_table} {{_uuid: item.uuid}}) \
                 SET idx._title = item.title, idx._content = item.content, idx._content_hash = item.hash"
            );
            conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "items".into(), value: items_param }],
            ).await.map_err(|e| format!("KBUpdateNode undo failed: {e}"))?;
        }
        Ok(())
    }
}

// ─── KBChunkNode ────────────────────────────────────────────────────────────

/// Generates chunk EntityRecords + RelationRecords (HAS_CHUNK, SOURCED)
/// from changed KB content. Pure CPU work — no DB queries.
///
/// **Input**: `kb_content` — `BatchPayload<KBContentRecord>` (PortType::KBContent)
/// **Output**: `entities` — chunk records, `relations` — links, `done` — Empty
/// **Services**: `chunker_cache`
pub struct KBChunkNode {
    name: String,
}

impl KBChunkNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for KBChunkNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "KBChunkNode"
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "kb_content", port_type: PortType::KBContent, required: true },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "entities", port_type: PortType::Entities, required: false },
            PortDef { name: "relations", port_type: PortType::Relations, required: false },
            PortDef { name: "done", port_type: PortType::Empty, required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let items: Vec<KBContentRecord> = match ctx.take_input("kb_content") {
            Some(PortValue::Batch(payload)) => payload
                .take::<KBContentRecord>()
                .ok_or("KBChunkNode: failed to extract Vec<KBContentRecord>")?,
            _ => return Err("KBChunkNode: missing 'kb_content' input".to_string()),
        };

        let chunker_cache = ctx.service::<HashMap<ChunkerConfig, Chunker>>("chunker_cache")
            .ok_or("KBChunkNode: 'chunker_cache' service not registered")?;
        let kb_metadata = ctx.service::<HashMap<String, KBMetadata>>("kb_metadata")
            .ok_or("KBChunkNode: 'kb_metadata' service not registered")?;

        let mut all_entities: Vec<EntityRecord> = Vec::new();
        let mut all_relations: Vec<RelationRecord> = Vec::new();

        for rec in &items {
            let kb_meta = match kb_metadata.get(&rec.kb_name) {
                Some(m) => m,
                None => continue,
            };
            let chunking = &kb_meta.chunking;
            let chunker_key = ChunkerConfig {
                max_size: chunking.max_size,
                overlap: chunking.overlap,
                strategy: chunking.strategy.clone(),
            };
            let chunker = chunker_cache
                .get(&chunker_key)
                .expect("chunker must be pre-warmed");

            let chunk_table = format!("{}_Index_Chunk", rec.kb_name);
            let (entities, relations) = generate_chunk_records(
                &rec.kb_name, &rec.index_entry_uuid, &rec.sources, chunker, &chunk_table,
            );
            all_entities.extend(entities);
            all_relations.extend(relations);
        }

        ctx.log_metric("items", items.len());
        ctx.log_metric("chunks", all_entities.len());
        ctx.log_metric("relations", all_relations.len());

        ctx.set_output("done", PortValue::Empty);
        ctx.set_output("entities", PortValue::Batch(
            BatchPayload::new(PortType::Entities, all_entities),
        ));
        ctx.set_output("relations", PortValue::Batch(
            BatchPayload::new(PortType::Relations, all_relations),
        ));
        Ok(())
    }

    // Read-only: nothing to undo, but must return true so rollback doesn't fail
    fn can_undo(&self) -> bool { true }
}

// ─── KBFlushNode ───────────────────────────────────────────────────────────

/// Flushes Lucivy FTS indexes for configured tables.
///
/// Generic node — works with any table that has a Lucivy index.
/// Runs `CALL FLUSH_LUCIVY_INDEX('{table}')` to commit + reload the reader
/// so subsequent searches don't pay the lazy-flush cost.
///
/// **Config**: `tables` — list of table names to flush
/// **Input**: `trigger` — Empty signal (optional)
/// **Output**: `done` — Empty signal
/// **Services**: `conn` (DbConnection)
pub struct FlushNode {
    name: String,
    tables: Vec<String>,
    undo_data: Option<serde_json::Value>,
}

impl FlushNode {
    pub fn new(name: impl Into<String>, tables: Vec<String>) -> Self {
        Self { name: name.into(), tables, undo_data: None }
    }
}

#[async_trait]
impl Node for FlushNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "FlushNode"
    }
    fn node_config(&self) -> serde_json::Value {
        serde_json::json!({ "tables": self.tables })
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "trigger", port_type: PortType::Empty, required: false },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "done", port_type: PortType::Empty, required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("FlushNode: 'conn' service not registered")?;

        let mut flushed: usize = 0;
        for table in &self.tables {
            if conn.execute(&format!("CALL FLUSH_LUCIVY_INDEX('{table}')")).await.is_ok() {
                flushed += 1;
            }
        }

        // Capture tables for undo (re-flush)
        self.undo_data = Some(serde_json::json!(self.tables));

        ctx.log_metric("table_count", self.tables.len());
        ctx.log_metric("flushed", flushed);
        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<serde_json::Value> {
        self.undo_data.clone()
    }

    async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String> {
        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("FlushNode undo: 'conn' service not registered")?;

        let tables = undo_ctx.as_array()
            .ok_or("FlushNode undo: expected array of table names")?;

        for t in tables {
            if let Some(table) = t.as_str() {
                conn.execute(&format!("CALL FLUSH_LUCIVY_INDEX('{table}')"))
                    .await
                    .ok(); // Best-effort
            }
        }
        Ok(())
    }
}

// ─── SparseCommitNode ─────────────────────────────────────────────────────

/// Commits dirty sparse vector indexes for configured tables.
///
/// Same pattern as [`FlushNode`] for FTS — explicit commit via dataflow node.
/// Calls `handle.commit_inner()` on each configured table's `SparseHandle`.
///
/// **Config**: `tables` — list of table names to commit
/// **Input**: `trigger` — Empty signal (optional)
/// **Output**: `done` — Empty signal
/// **Services**: `sparse_handles` (HashMap<String, Arc<SparseHandle>>)
pub struct SparseCommitNode {
    name: String,
    tables: Vec<String>,
    undo_data: Option<serde_json::Value>,
}

impl SparseCommitNode {
    pub fn new(name: impl Into<String>, tables: Vec<String>) -> Self {
        Self { name: name.into(), tables, undo_data: None }
    }
}

#[async_trait]
impl Node for SparseCommitNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn node_type(&self) -> &'static str {
        "SparseCommitNode"
    }
    fn node_config(&self) -> serde_json::Value {
        serde_json::json!({ "tables": self.tables })
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "trigger", port_type: PortType::Empty, required: false },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "done", port_type: PortType::Empty, required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let handles = ctx.service::<HashMap<String, Arc<sparse_vector::handle::SparseHandle>>>("sparse_handles")
            .ok_or("SparseCommitNode: 'sparse_handles' service not registered")?;

        let mut committed: usize = 0;
        for table in &self.tables {
            if let Some(handle) = handles.get(table) {
                handle.commit_inner()
                    .map_err(|e| format!("SparseCommitNode: commit '{table}' failed: {e}"))?;
                committed += 1;
            }
        }

        self.undo_data = Some(serde_json::json!(self.tables));
        ctx.log_metric("table_count", self.tables.len());
        ctx.log_metric("committed", committed);
        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<serde_json::Value> {
        self.undo_data.clone()
    }

    async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String> {
        let handles = ctx.service::<HashMap<String, Arc<sparse_vector::handle::SparseHandle>>>("sparse_handles")
            .ok_or("SparseCommitNode undo: 'sparse_handles' service not registered")?;

        let tables = undo_ctx.as_array()
            .ok_or("SparseCommitNode undo: expected array of table names")?;

        for t in tables {
            if let Some(table) = t.as_str() {
                if let Some(handle) = handles.get(table) {
                    handle.commit_inner().ok(); // Best-effort
                }
            }
        }
        Ok(())
    }
}

// ─── RechunkDeleteNode ─────────────────────────────────────────────────────

/// Delete old chunks for entities about to be re-chunked.
///
/// Batch-deletes `{Entity}_Chunk` nodes by `_parent_uuid`, then passes the
/// same `Vec<EntityRecord>` through to output for downstream ChunkRecordNode.
///
/// **Input**: `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
/// **Output**: `entities` — same records (pass-through)
/// **Services**: `conn` (DbConnection)
pub struct RechunkDeleteNode {
    name: String,
}

impl RechunkDeleteNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for RechunkDeleteNode {
    fn name(&self) -> &str { &self.name }
    fn node_type(&self) -> &'static str { "RechunkDeleteNode" }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "entities", port_type: PortType::Entities, required: true },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "entities", port_type: PortType::Entities, required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let items: Vec<EntityRecord> = match ctx.take_input("entities") {
            Some(PortValue::Batch(payload)) => payload
                .take::<EntityRecord>()
                .ok_or("RechunkDeleteNode: failed to extract Vec<EntityRecord>")?,
            _ => return Err("RechunkDeleteNode: missing 'entities' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("RechunkDeleteNode: 'conn' service not registered")?;

        // Group by entity_name
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        for rec in &items {
            let uuid = rec.data.get("_uuid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            groups.entry(rec.entity_name.clone()).or_default().push(uuid);
        }

        let mut total_deleted: usize = 0;
        for (entity_name, uuids) in &groups {
            let chunk_table = format!("{entity_name}_Chunk");
            let uuid_list = CypherValue::List(
                uuids.iter().map(|u| CypherValue::String(u.clone())).collect(),
            );
            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .ok_or("RechunkDeleteNode: 'dialect' service not registered")?;
            let cypher = dialect.batch_cascade_delete_returning_count(&chunk_table, "_parent_uuid");
            let result = conn
                .execute_with_params(
                    &cypher,
                    &[QueryParam { name: "uuids".into(), value: uuid_list }],
                )
                .await
                .map_err(|e| e.to_string())?;
            for row in &result.rows {
                if let Some(cnt) = row.get(1).and_then(|v| v.as_i64()) {
                    total_deleted += cnt as usize;
                }
            }
        }

        ctx.log_metric("entities", items.len());
        ctx.log_metric("groups", groups.len());
        ctx.log_metric("chunks_deleted", total_deleted);

        ctx.set_output("entities", PortValue::Batch(
            BatchPayload::new(PortType::Entities, items),
        ));
        Ok(())
    }

    // Deletes old chunks before re-chunking; undo is a no-op because
    // re-ingestion will recreate chunks from the restored entity data.
    fn can_undo(&self) -> bool { true }
}

// ─── DeleteRecordNode ──────────────────────────────────────────────────────

/// Batch cascade-delete entities + their chunks from `Vec<DeleteRecord>`.
///
/// For each entity group, handles KB titleFor (index chunks + entries),
/// KB contentFor (SOURCED chunks + re-aggregation), simple entity chunks,
/// then deletes the entities themselves and removes from node_id_cache.
///
/// **Input**: `deletes` — `BatchPayload<DeleteRecord>` (PortType::Deletes)
/// **Output**: `done` — Empty signal
/// **Services**: `conn`, `node_id_cache`, `config`, `entity_configs`, `kb_metadata`,
///              `pending_aggregates`, `delete_results`
pub struct DeleteRecordNode {
    name: String,
    undo_data: Option<serde_json::Value>,
}

impl DeleteRecordNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), undo_data: None }
    }
}

#[async_trait]
impl Node for DeleteRecordNode {
    fn name(&self) -> &str { &self.name }
    fn node_type(&self) -> &'static str { "DeleteRecordNode" }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "deletes", port_type: PortType::Deletes, required: true },
            PortDef { name: "trigger", port_type: PortType::Empty, required: false },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "done", port_type: PortType::Empty, required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let items: Vec<DeleteRecord> = match ctx.take_input("deletes") {
            Some(PortValue::Batch(payload)) => payload
                .take::<DeleteRecord>()
                .ok_or("DeleteRecordNode: failed to extract Vec<DeleteRecord>")?,
            _ => return Err("DeleteRecordNode: missing 'deletes' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("DeleteRecordNode: 'conn' service not registered")?;
        let node_id_cache = ctx.service::<RwLock<NodeIdCache>>("node_id_cache")
            .ok_or("DeleteRecordNode: 'node_id_cache' service not registered")?;
        let config = ctx.service::<CatalogConfig>("config")
            .ok_or("DeleteRecordNode: 'config' service not registered")?;
        let entity_configs = ctx.service::<HashMap<String, crate::config::EntityConfig>>("entity_configs")
            .ok_or("DeleteRecordNode: 'entity_configs' service not registered")?;
        let kb_metadata = ctx.service::<HashMap<String, KBMetadata>>("kb_metadata")
            .ok_or("DeleteRecordNode: 'kb_metadata' service not registered")?;
        let pending_agg = ctx.service::<Mutex<Vec<AggregateRecord>>>("pending_aggregates")
            .ok_or("DeleteRecordNode: 'pending_aggregates' service not registered")?;
        let results_svc = ctx.service::<Mutex<Vec<DeleteResult>>>("delete_results")
            .ok_or("DeleteRecordNode: 'delete_results' service not registered")?;
        let event_bus = ctx.service::<EventBus>("event_bus");

        // Group by entity_name
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        for rec in &items {
            groups.entry(rec.entity_name.clone()).or_default().push(rec.uuid.clone());
        }

        let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
            .ok_or("DeleteRecordNode: 'dialect' service not registered")?;

        ctx.log_metric("items", items.len());
        ctx.log_metric("groups", groups.len());

        let mut all_results: Vec<DeleteResult> = Vec::new();
        let mut undo_groups: HashMap<String, Vec<BTreeMap<String, CypherValue>>> = HashMap::new();

        for (entity_name, uuids) in &groups {
            let entity_def = match config.entities.get(entity_name) {
                Some(def) => def,
                None => continue,
            };
            let entity_kbs = resolve_entity_kbs(entity_def);
            let mut per_uuid_chunks: HashMap<String, usize> = HashMap::new();

            // --- KB handling ---
            for (kb_name, mapping) in &entity_kbs {
                if mapping.title_field.is_some() {
                    // titleFor: delete index chunks + index entries
                    let index_uuids: Vec<String> = uuids.iter()
                        .map(|u| hashsafe_uuid(
                            &format!("{kb_name}_Index"),
                            &[entity_name.as_str(), u.as_str()],
                        ))
                        .collect();

                    let chunk_table = format!("{kb_name}_Index_Chunk");
                    let idx_list = CypherValue::List(
                        index_uuids.iter().map(|u| CypherValue::String(u.clone())).collect(),
                    );

                    // Delete index chunks
                    let del_chunks = dialect.batch_cascade_delete_returning_count(&chunk_table, "_parent_uuid");
                    let result = conn
                        .execute_with_params(
                            &del_chunks,
                            &[QueryParam { name: "uuids".into(), value: idx_list.clone() }],
                        )
                        .await
                        .map_err(|e| e.to_string())?;

                    let idx_to_entity: HashMap<&str, &str> = index_uuids.iter()
                        .zip(uuids.iter())
                        .map(|(i, u)| (i.as_str(), u.as_str()))
                        .collect();
                    for row in &result.rows {
                        if let (Some(idx_uuid), Some(cnt)) = (
                            row.get(0).and_then(|v| v.as_str()),
                            row.get(1).and_then(|v| v.as_i64()),
                        ) {
                            if let Some(&entity_uuid) = idx_to_entity.get(idx_uuid) {
                                *per_uuid_chunks.entry(entity_uuid.to_string()).or_default() += cnt as usize;
                            }
                        }
                    }

                    // Delete index entries
                    let index_table = format!("{kb_name}_Index");
                    let del_idx = dialect.batch_cascade_delete(&index_table);
                    let _ = conn
                        .execute_with_params(
                            &del_idx,
                            &[QueryParam { name: "uuids".into(), value: idx_list }],
                        )
                        .await;
                } else {
                    // contentFor: delete SOURCED chunks, enqueue re-aggregation
                    let kb_meta = match kb_metadata.get(kb_name.as_str()) {
                        Some(m) => m,
                        None => continue,
                    };
                    let title_entity = kb_meta.title.entity.clone();

                    let sourced_rel = format!("{entity_name}_SOURCED_{kb_name}");
                    let chunk_table = format!("{kb_name}_Index_Chunk");
                    let uuid_list = CypherValue::List(
                        uuids.iter().map(|u| CypherValue::String(u.clone())).collect(),
                    );

                    // Delete SOURCED chunks via dialect
                    let del_sourced = dialect.join_delete_returning_count(entity_name, &sourced_rel, &chunk_table);
                    let result = conn
                        .execute_with_params(
                            &del_sourced,
                            &[QueryParam { name: "uuids".into(), value: uuid_list.clone() }],
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    for row in &result.rows {
                        if let (Some(uuid), Some(cnt)) = (
                            row.get(0).and_then(|v| v.as_str()),
                            row.get(1).and_then(|v| v.as_i64()),
                        ) {
                            *per_uuid_chunks.entry(uuid.to_string()).or_default() += cnt as usize;
                        }
                    }

                    // Find linked title entities → push AggregateRecords
                    if let Some((rel_name, title_is_from)) =
                        find_relation_to_entity(&config, &title_entity, entity_name)
                    {
                        let match_pattern = if title_is_from {
                            dialect.join_select(
                                entity_name, &rel_name, &title_entity,
                                false, "_uuid", &["m._uuid"],
                            )
                        } else {
                            dialect.join_select(
                                entity_name, &rel_name, &title_entity,
                                true, "_uuid", &["m._uuid"],
                            )
                        };
                        let title_results = conn
                            .execute_with_params(
                                &match_pattern,
                                &[QueryParam { name: "uuids".into(), value: uuid_list }],
                            )
                            .await
                            .map_err(|e| e.to_string())?;

                        let mut new_aggregates: Vec<AggregateRecord> = Vec::new();
                        let mut seen = HashSet::new();
                        for row in &title_results.rows {
                            if let Some(title_uuid) = row.first().and_then(|v| v.as_str()) {
                                let index_uuid = hashsafe_uuid(
                                    &format!("{kb_name}_Index"),
                                    &[&title_entity, title_uuid],
                                );
                                if seen.insert(index_uuid.clone()) {
                                    new_aggregates.push(AggregateRecord {
                                        index_entry_uuid: index_uuid,
                                        kb_name: kb_name.clone(),
                                        title_entity: title_entity.clone(),
                                        source_uuid: title_uuid.to_string(),
                                    });
                                }
                            }
                        }
                        if !new_aggregates.is_empty() {
                            pending_agg.lock().map_err(|e| format!("pending_aggregates lock: {e}"))?
                                .extend(new_aggregates);
                        }
                    }
                }
            }

            // Simple entity: cascade-delete chunks
            if entity_configs.contains_key(entity_name) && entity_kbs.is_empty() {
                let chunk_table = format!("{entity_name}_Chunk");
                let uuid_list = CypherValue::List(
                    uuids.iter().map(|u| CypherValue::String(u.clone())).collect(),
                );
                let del_chunks = dialect.batch_cascade_delete_returning_count(&chunk_table, "_parent_uuid");
                let result = conn
                    .execute_with_params(
                        &del_chunks,
                        &[QueryParam { name: "uuids".into(), value: uuid_list }],
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                for row in &result.rows {
                    if let (Some(uuid), Some(cnt)) = (
                        row.get(0).and_then(|v| v.as_str()),
                        row.get(1).and_then(|v| v.as_i64()),
                    ) {
                        *per_uuid_chunks.entry(uuid.to_string()).or_default() += cnt as usize;
                    }
                }
            }

            // Read full entity data before delete (for undo + existence check)
            let uuid_list = CypherValue::List(
                uuids.iter().map(|u| CypherValue::String(u.clone())).collect(),
            );
            let existing: HashSet<String> = {
                let read = conn.execute_with_params(
                    &dialect.select_entity_all_by_uuids(entity_name),
                    &[QueryParam { name: "uuids".into(), value: uuid_list.clone() }],
                ).await.map_err(|e| e.to_string())?;
                let mut found = HashSet::new();
                for row in &read.rows {
                    if let Some(CypherValue::Map(props)) = row.first() {
                        if let Some(uuid) = props.get("_uuid").and_then(|v| v.as_str()) {
                            found.insert(uuid.to_string());
                            // Strip internal rag3db properties (_id, _label)
                            let clean: BTreeMap<String, CypherValue> = props.iter()
                                .filter(|(k, _)| k.as_str() != "_id" && k.as_str() != "_label")
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect();
                            undo_groups.entry(entity_name.clone())
                                .or_default()
                                .push(clean);
                        }
                    }
                }
                found
            };

            // Warn for nonexistent UUIDs
            for uuid in uuids.iter().filter(|u| !existing.contains(u.as_str())) {
                ctx.warn(format!("{entity_name} with uuid '{uuid}' not found, skipping"));
            }

            // Delete entities themselves
            let del_entities = dialect.batch_cascade_delete(entity_name);
            conn.execute_with_params(
                &del_entities,
                &[QueryParam { name: "uuids".into(), value: uuid_list }],
            )
            .await
            .map_err(|e| e.to_string())?;

            // Remove from node_id_cache
            if let Ok(mut cache) = node_id_cache.write() {
                for uuid in uuids {
                    cache.remove(uuid.as_str());
                }
            }

            // Build DeleteResults + emit EntityDeleted events
            for uuid in uuids {
                let chunks_deleted = per_uuid_chunks.get(uuid).copied().unwrap_or(0);
                all_results.push(DeleteResult {
                    uuid: uuid.clone(),
                    entity: entity_name.clone(),
                    chunks_deleted,
                    relations_deleted: 0,
                });
                if existing.contains(uuid.as_str()) {
                    if let Some(ref bus) = event_bus {
                        bus.emit(CatalogEvent::EntityDeleted {
                            entity: entity_name.clone(),
                            uuid: uuid.clone(),
                            chunks_deleted,
                        });
                    }
                }
            }
        }

        // Push results to shared service
        results_svc.lock().map_err(|e| format!("delete_results lock: {e}"))?
            .extend(all_results);

        // Capture undo data
        if !undo_groups.is_empty() {
            self.undo_data = Some(
                serde_json::to_value(&undo_groups)
                    .map_err(|e| format!("DeleteRecordNode: failed to serialize undo data: {e}"))?
            );
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<serde_json::Value> {
        self.undo_data.clone()
    }

    async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String> {
        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("DeleteRecordNode undo: 'conn' service not registered")?;

        let groups: HashMap<String, Vec<BTreeMap<String, CypherValue>>> =
            serde_json::from_value(undo_ctx)
                .map_err(|e| format!("DeleteRecordNode undo: failed to deserialize: {e}"))?;

        for (entity_name, items) in &groups {
            if items.is_empty() { continue; }

            // Re-create deleted entities via batch upsert (MERGE/INSERT)
            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .ok_or("DeleteRecordNode undo: 'dialect' service not registered")?;
            let columns: Vec<&str> = items[0].keys().map(|k| k.as_str()).collect();
            let cypher = dialect.batch_upsert(entity_name, &columns);

            let items_param = CypherValue::List(
                items.iter().map(|m| CypherValue::Map(m.clone())).collect()
            );
            conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "items".into(), value: items_param }],
            ).await.map_err(|e| format!("DeleteRecordNode undo failed: {e}"))?;

            ctx.info(format!(
                "restored {} {entity_name}(s), re-ingestion needed for chunks/embeddings",
                items.len()
            ));
        }
        Ok(())
    }
}

// ─── UpdateRecordNode ──────────────────────────────────────────────────────

/// Batch field update + change detection from `Vec<UpdateRecord>`.
///
/// Groups by (entity_name, sorted_field_keys), batch-reads old hashes,
/// detects content changes, batch SETs all fields. For changed items:
/// - KB entities: pushes AggregateRecords to `pending_aggregates`
/// - Simple entities: reads full data, builds EntityRecords → `rechunk_entities`
///
/// **Input**: `updates` — `BatchPayload<UpdateRecord>` (PortType::Updates)
/// **Output**: `done` — Empty signal, `rechunk_entities` — EntityRecords for simple entities
/// **Services**: `conn`, `config`, `entity_configs`, `kb_metadata`,
///              `pending_aggregates`, `update_results`
pub struct UpdateRecordNode {
    name: String,
    undo_data: Option<serde_json::Value>,
}

impl UpdateRecordNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), undo_data: None }
    }
}

#[async_trait]
impl Node for UpdateRecordNode {
    fn name(&self) -> &str { &self.name }
    fn node_type(&self) -> &'static str { "UpdateRecordNode" }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "updates", port_type: PortType::Updates, required: true },
            PortDef { name: "trigger", port_type: PortType::Empty, required: false },
        ]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "done", port_type: PortType::Empty, required: false },
            PortDef { name: "rechunk_entities", port_type: PortType::Entities, required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let raw_items: Vec<UpdateRecord> = match ctx.take_input("updates") {
            Some(PortValue::Batch(payload)) => payload
                .take::<UpdateRecord>()
                .ok_or("UpdateRecordNode: failed to extract Vec<UpdateRecord>")?,
            _ => return Err("UpdateRecordNode: missing 'updates' input".to_string()),
        };

        // ─── Merge duplicate updates for same (entity_name, uuid) ───
        let items: Vec<UpdateRecord> = {
            let mut seen: HashMap<(String, String), usize> = HashMap::new();
            let mut merged: Vec<UpdateRecord> = Vec::new();
            let mut merge_count = 0usize;
            for update in raw_items {
                let key = (update.entity_name.clone(), update.uuid.clone());
                if let Some(&idx) = seen.get(&key) {
                    merged[idx].data.extend(update.data);
                    merged[idx].new_content_hash = String::new(); // sentinel: force re-chunk
                    merge_count += 1;
                } else {
                    seen.insert(key, merged.len());
                    merged.push(update);
                }
            }
            if merge_count > 0 {
                ctx.info(format!("merged {merge_count} duplicate update(s) into existing records"));
            }
            merged
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("UpdateRecordNode: 'conn' service not registered")?;
        let config = ctx.service::<CatalogConfig>("config")
            .ok_or("UpdateRecordNode: 'config' service not registered")?;
        let entity_configs = ctx.service::<HashMap<String, crate::config::EntityConfig>>("entity_configs")
            .ok_or("UpdateRecordNode: 'entity_configs' service not registered")?;
        let kb_metadata = ctx.service::<HashMap<String, KBMetadata>>("kb_metadata")
            .ok_or("UpdateRecordNode: 'kb_metadata' service not registered")?;
        let pending_agg = ctx.service::<Mutex<Vec<AggregateRecord>>>("pending_aggregates")
            .ok_or("UpdateRecordNode: 'pending_aggregates' service not registered")?;
        let results_svc = ctx.service::<Mutex<Vec<UpdateResult>>>("update_results")
            .ok_or("UpdateRecordNode: 'update_results' service not registered")?;
        let event_bus = ctx.service::<EventBus>("event_bus");

        // Group by entity_name
        let mut entity_groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, rec) in items.iter().enumerate() {
            entity_groups.entry(rec.entity_name.clone()).or_default().push(i);
        }

        ctx.log_metric("items", items.len());
        ctx.log_metric("entity_groups", entity_groups.len());

        let mut all_results: Vec<UpdateResult> = Vec::new();
        let mut all_rechunk_entities: Vec<EntityRecord> = Vec::new();
        let mut undo_snapshots: HashMap<String, Vec<BTreeMap<String, CypherValue>>> = HashMap::new();

        for (entity_name, entity_indices) in &entity_groups {
            let entity_def = match config.entities.get(entity_name) {
                Some(def) => def,
                None => continue,
            };
            let entity_kbs = resolve_entity_kbs(entity_def);

            // 1. Batch-read old entity data (for hashes + undo)
            let uuid_list = CypherValue::List(
                entity_indices.iter()
                    .map(|&i| CypherValue::String(items[i].uuid.clone()))
                    .collect(),
            );
            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .ok_or("UpdateRecordNode: 'dialect' service not registered")?;
            let old_read_query = dialect.select_entity_all_by_uuids(entity_name);
            let old_result = conn
                .execute_with_params(
                    &old_read_query,
                    &[QueryParam { name: "uuids".into(), value: uuid_list }],
                )
                .await
                .map_err(|e| e.to_string())?;

            let mut old_hashes: HashMap<String, String> = HashMap::new();
            for row in &old_result.rows {
                if let Some(CypherValue::Map(props)) = row.first() {
                    if let (Some(uuid), Some(hash)) = (
                        props.get("_uuid").and_then(|v| v.as_str()),
                        props.get("_content_hash").and_then(|v| v.as_str()),
                    ) {
                        old_hashes.insert(uuid.to_string(), hash.to_string());
                        // Strip internal rag3db properties (_id, _label)
                        let clean: BTreeMap<String, CypherValue> = props.iter()
                            .filter(|(k, _)| k.as_str() != "_id" && k.as_str() != "_label")
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        undo_snapshots.entry(entity_name.clone())
                            .or_default()
                            .push(clean);
                    }
                }
            }

            // Warn for UUIDs not found in DB
            for &i in entity_indices {
                if !old_hashes.contains_key(&items[i].uuid) {
                    ctx.warn(format!(
                        "{entity_name} with uuid '{}' not found, update is a no-op",
                        items[i].uuid
                    ));
                }
            }

            // 2. Detect content changes
            let changed: Vec<bool> = entity_indices.iter()
                .map(|&i| {
                    let rec = &items[i];
                    old_hashes.get(&rec.uuid)
                        .map_or(true, |old| old != &rec.new_content_hash)
                })
                .collect();

            // 3. Batch SET via UNWIND — group by sorted field keys
            let mut set_groups: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
            for &i in entity_indices {
                let mut keys: Vec<String> = items[i].data.keys().cloned().collect();
                keys.sort();
                set_groups.entry(keys).or_default().push(i);
            }

            for (field_keys, indices) in &set_groups {
                let items_param = CypherValue::List(
                    indices.iter().map(|&i| {
                        let rec = &items[i];
                        let mut m = BTreeMap::new();
                        m.insert("_uuid".to_string(), CypherValue::String(rec.uuid.clone()));
                        m.insert("_content_hash".to_string(), CypherValue::String(rec.new_content_hash.clone()));
                        for (k, v) in &rec.data {
                            m.insert(k.clone(), v.clone());
                        }
                        CypherValue::Map(m)
                    }).collect(),
                );

                let mut update_cols: Vec<&str> = field_keys.iter().map(|s| s.as_str()).collect();
                update_cols.push("_content_hash");
                let set_cypher = dialect.batch_update_fields(entity_name, &update_cols);
                conn.execute_with_params(
                    &set_cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                )
                .await
                .map_err(|e| e.to_string())?;
            }

            // 4. Handle content changes
            let changed_uuids: HashSet<&str> = entity_indices.iter()
                .zip(changed.iter())
                .filter_map(|(&i, &c)| if c { Some(items[i].uuid.as_str()) } else { None })
                .collect();

            if !changed_uuids.is_empty() {
                // KB entities: enqueue AggregateRecords
                for (kb_name, mapping) in &entity_kbs {
                    if mapping.title_field.is_some() {
                        // titleFor: each changed entity needs re-aggregation
                        let mut new_aggregates: Vec<AggregateRecord> = Vec::new();
                        for uuid in &changed_uuids {
                            let index_uuid = hashsafe_uuid(
                                &format!("{kb_name}_Index"),
                                &[entity_name.as_str(), *uuid],
                            );
                            new_aggregates.push(AggregateRecord {
                                index_entry_uuid: index_uuid,
                                kb_name: kb_name.clone(),
                                title_entity: entity_name.clone(),
                                source_uuid: uuid.to_string(),
                            });
                        }
                        pending_agg.lock().map_err(|e| format!("pending_aggregates lock: {e}"))?
                            .extend(new_aggregates);
                    } else {
                        // contentFor: find linked title entities
                        let kb_meta = match kb_metadata.get(kb_name.as_str()) {
                            Some(m) => m,
                            None => continue,
                        };
                        let title_entity = kb_meta.title.entity.clone();
                        if let Some((rel_name, title_is_from)) =
                            find_relation_to_entity(&config, &title_entity, entity_name)
                        {
                            let uuid_param = CypherValue::List(
                                changed_uuids.iter().map(|u| CypherValue::String(u.to_string())).collect(),
                            );
                            let match_pattern = if title_is_from {
                                dialect.join_select(
                                    entity_name, &rel_name, &title_entity,
                                    false, "_uuid", &["m._uuid"],
                                )
                            } else {
                                dialect.join_select(
                                    entity_name, &rel_name, &title_entity,
                                    true, "_uuid", &["m._uuid"],
                                )
                            };
                            let title_results = conn
                                .execute_with_params(
                                    &match_pattern,
                                    &[QueryParam { name: "uuids".into(), value: uuid_param }],
                                )
                                .await
                                .map_err(|e| e.to_string())?;

                            let mut new_aggregates: Vec<AggregateRecord> = Vec::new();
                            let mut seen = HashSet::new();
                            for row in &title_results.rows {
                                if let Some(title_uuid) = row.first().and_then(|v| v.as_str()) {
                                    let index_uuid = hashsafe_uuid(
                                        &format!("{kb_name}_Index"),
                                        &[&title_entity, title_uuid],
                                    );
                                    if seen.insert(index_uuid.clone()) {
                                        new_aggregates.push(AggregateRecord {
                                            index_entry_uuid: index_uuid,
                                            kb_name: kb_name.clone(),
                                            title_entity: title_entity.clone(),
                                            source_uuid: title_uuid.to_string(),
                                        });
                                    }
                                }
                            }
                            if !new_aggregates.is_empty() {
                                pending_agg.lock().map_err(|e| format!("pending_aggregates lock: {e}"))?
                                    .extend(new_aggregates);
                            }
                        }
                    }
                }

                // Simple entities: read full data → build EntityRecords for rechunking
                if entity_configs.contains_key(entity_name) && entity_kbs.is_empty() {
                    let uuid_param = CypherValue::List(
                        changed_uuids.iter().map(|u| CypherValue::String(u.to_string())).collect(),
                    );
                    let read_cypher = dialect.select_entity_all_by_uuids(entity_name);
                    let read_result = conn
                        .execute_with_params(
                            &read_cypher,
                            &[QueryParam { name: "uuids".into(), value: uuid_param }],
                        )
                        .await
                        .map_err(|e| e.to_string())?;

                    for row in &read_result.rows {
                        if let Some(first) = row.first() {
                            let props = match first {
                                CypherValue::Map(m) => m.clone(),
                                _ => continue,
                            };
                            let uuid = match props.get("_uuid").and_then(|v| v.as_str()) {
                                Some(u) => u.to_string(),
                                None => continue,
                            };
                            all_rechunk_entities.push(EntityRecord {
                                entity_name: entity_name.clone(),
                                data: props,
                                entity_ref: EntityRef::pre_resolved(entity_name, &uuid, &uuid),
                                resolver: None,
                            });
                        }
                    }
                }
            }

            // 5. Build UpdateResults + emit EntityUpdated events
            for (idx_in_group, &i) in entity_indices.iter().enumerate() {
                let content_changed = changed[idx_in_group];
                let uuid = &items[i].uuid;
                let found = old_hashes.contains_key(uuid);
                let reembedded = found && content_changed && (
                    !entity_kbs.is_empty() || entity_configs.contains_key(entity_name)
                );
                let status = if !found {
                    UpdateStatus::Updated // no old hash → considered "changed" (no-op in DB)
                } else if content_changed {
                    UpdateStatus::Updated
                } else {
                    UpdateStatus::Unchanged
                };
                all_results.push(UpdateResult {
                    uuid: uuid.clone(),
                    entity: entity_name.clone(),
                    status,
                    reembedded,
                    chunks_created: 0,
                    chunks_deleted: 0,
                });
                if found {
                    if let Some(ref bus) = event_bus {
                        bus.emit(CatalogEvent::EntityUpdated {
                            entity: entity_name.clone(),
                            uuid: uuid.clone(),
                            reembedded,
                            chunks_created: 0,
                            chunks_deleted: 0,
                        });
                    }
                }
            }
        }

        // Push results to shared service
        results_svc.lock().map_err(|e| format!("update_results lock: {e}"))?
            .extend(all_results);

        // Capture undo data (old entity snapshots)
        if !undo_snapshots.is_empty() {
            self.undo_data = Some(
                serde_json::to_value(&undo_snapshots)
                    .map_err(|e| format!("UpdateRecordNode: failed to serialize undo data: {e}"))?
            );
        }

        ctx.log_metric("rechunk_entities", all_rechunk_entities.len());

        // Always emit rechunk_entities (even empty) so downstream rechunk pipeline
        // receives its input and doesn't deadlock.
        ctx.set_output("rechunk_entities", PortValue::Batch(
            BatchPayload::new(PortType::Entities, all_rechunk_entities),
        ));
        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }

    fn can_undo(&self) -> bool { true }

    fn undo_context(&self) -> Option<serde_json::Value> {
        self.undo_data.clone()
    }

    async fn undo(&mut self, ctx: &mut NodeContext, undo_ctx: serde_json::Value) -> Result<(), String> {
        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("UpdateRecordNode undo: 'conn' service not registered")?;

        let groups: HashMap<String, Vec<BTreeMap<String, CypherValue>>> =
            serde_json::from_value(undo_ctx)
                .map_err(|e| format!("UpdateRecordNode undo: failed to deserialize: {e}"))?;

        for (entity_name, items) in &groups {
            if items.is_empty() { continue; }

            // Build SET clause from columns (restore all old values)
            let columns: Vec<&str> = items[0].keys().map(|k| k.as_str()).collect();
            let other_cols: Vec<&str> = columns.iter()
                .filter(|c| **c != "_uuid")
                .copied()
                .collect();
            if other_cols.is_empty() { continue; }

            let dialect = ctx.service::<Arc<dyn crate::dialect::SchemaDialect>>("dialect")
                .ok_or("UpdateRecordNode undo: 'dialect' service not registered")?;
            let cypher = dialect.batch_update_fields(entity_name, &other_cols);

            let items_param = CypherValue::List(
                items.iter().map(|m| CypherValue::Map(m.clone())).collect()
            );
            conn.execute_with_params(
                &cypher,
                &[QueryParam { name: "items".into(), value: items_param }],
            ).await.map_err(|e| format!("UpdateRecordNode undo failed: {e}"))?;

            ctx.info(format!(
                "restored {} {entity_name}(s) to pre-update state",
                items.len()
            ));
        }
        Ok(())
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{EntityConfig, SimpleFieldDef, FieldType};
    use crate::chunker::{Chunker, ChunkerConfig};

    fn make_entity_config() -> EntityConfig {
        let mut fields = HashMap::new();
        fields.insert("name".into(), SimpleFieldDef {
            field_type: FieldType::String,
            is_title: true,
            is_content: false,
            ..Default::default()
        });
        fields.insert("description".into(), SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        });
        fields.insert("details".into(), SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        });
        fields.insert("price".into(), SimpleFieldDef {
            field_type: FieldType::Double,
            is_title: false,
            is_content: false,
            ..Default::default()
        });
        EntityConfig {
            fields,
            ..Default::default()
        }
    }

    fn make_entity_configs() -> HashMap<String, EntityConfig> {
        let mut map = HashMap::new();
        map.insert("Product".into(), make_entity_config());
        map
    }

    fn make_chunker_cache(config: &EntityConfig) -> HashMap<ChunkerConfig, Chunker> {
        let key = ChunkerConfig {
            max_size: config.chunking.max_size,
            overlap: config.chunking.overlap,
            strategy: config.chunking.strategy.clone(),
        };
        let mut cache = HashMap::new();
        cache.insert(key.clone(), Chunker::new(key));
        cache
    }

    fn make_product_data(name: &str, description: &str, details: &str) -> BTreeMap<String, CypherValue> {
        let mut data = BTreeMap::new();
        data.insert("_uuid".into(), CypherValue::String("test-uuid-123".into()));
        data.insert("name".into(), CypherValue::String(name.into()));
        data.insert("description".into(), CypherValue::String(description.into()));
        data.insert("details".into(), CypherValue::String(details.into()));
        data.insert("price".into(), CypherValue::Float(29.99));
        data
    }

    // ── ChunkRecordNode::compute_chunks tests ──

    #[test]
    fn chunk_simple_entity_produces_chunks() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Red Shoes", "A nice pair of red shoes.", "Made in Italy.");
        let (chunks, links) = ChunkRecordNode::compute_chunks(
            "Product", "test-uuid-123", &entity_ref, &data, &configs, &cache,
        );

        // Should produce chunks for both content fields (description + details)
        assert!(!chunks.is_empty(), "should produce at least one chunk");
        assert_eq!(chunks.len(), links.len(), "each chunk should have a link");
    }

    #[test]
    fn chunk_entity_names_correct() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "Description text.", "Details text.");
        let (chunks, links) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        for chunk in &chunks {
            assert_eq!(chunk.entity_name, "Product_Chunk");
        }
        for link in &links {
            assert_eq!(link.rel_name, "Product_CHUNKED_FROM");
        }
    }

    #[test]
    fn chunk_has_title_from_parent() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Red Shoes", "A product description.", "Some details.");
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        for chunk in &chunks {
            let title = chunk.data.get("_title").and_then(|v| v.as_str()).unwrap();
            assert_eq!(title, "Red Shoes");
        }
    }

    #[test]
    fn chunk_has_embed_hash_empty() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "Description.", "Details.");
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        for chunk in &chunks {
            let embed_hash = chunk.data.get("_embed_hash").and_then(|v| v.as_str()).unwrap();
            assert_eq!(embed_hash, "", "_embed_hash should be empty initially");
        }
    }

    #[test]
    fn chunk_has_text_hash() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "Some description text.", "");
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        for chunk in &chunks {
            let hash = chunk.data.get("_text_hash").and_then(|v| v.as_str()).unwrap();
            assert!(!hash.is_empty(), "_text_hash should not be empty");
        }
    }

    #[test]
    fn chunk_parent_field_set_correctly() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "Description text.", "Details text.");
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        let fields: HashSet<String> = chunks.iter()
            .filter_map(|c| c.data.get("_parent_field").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();

        // content_fields are sorted: ["description", "details"]
        assert!(fields.contains("description"), "should have description chunks");
        assert!(fields.contains("details"), "should have details chunks");
    }

    #[test]
    fn chunk_content_offset_multi_fields() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        // content_fields sorted = ["description", "details"]
        let desc = "A short description.";
        let details = "Some details here.";
        let data = make_product_data("Shoes", desc, details);
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        // description chunks should have _content_offset = 0
        let desc_chunks: Vec<_> = chunks.iter()
            .filter(|c| c.data.get("_parent_field").and_then(|v| v.as_str()) == Some("description"))
            .collect();
        for c in &desc_chunks {
            let offset = c.data.get("_content_offset").and_then(|v| v.as_i64()).unwrap();
            assert_eq!(offset, 0, "description chunks should have offset 0");
        }

        // details chunks should have _content_offset = len(description) + 2 ("\n\n")
        let expected_details_offset = desc.len() as i64 + 2;
        let details_chunks: Vec<_> = chunks.iter()
            .filter(|c| c.data.get("_parent_field").and_then(|v| v.as_str()) == Some("details"))
            .collect();
        for c in &details_chunks {
            let offset = c.data.get("_content_offset").and_then(|v| v.as_i64()).unwrap();
            assert_eq!(offset, expected_details_offset,
                "details chunks should have offset = {} + 2 = {}", desc.len(), expected_details_offset);
        }
    }

    #[test]
    fn chunk_unknown_entity_returns_empty() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Unknown");

        let data = make_product_data("X", "text", "text");
        let (chunks, links) = ChunkRecordNode::compute_chunks(
            "Unknown", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        assert!(chunks.is_empty());
        assert!(links.is_empty());
    }

    #[test]
    fn chunk_empty_content_returns_empty() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "", "");
        let (chunks, links) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        assert!(chunks.is_empty(), "empty content should produce no chunks");
        assert!(links.is_empty());
    }

    #[test]
    fn chunk_uuid_deterministic() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);

        let data = make_product_data("Shoes", "Description.", "Details.");

        let (entity_ref1, _) = EntityRef::new("Product");
        let (chunks1, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref1, &data, &configs, &cache,
        );

        let (entity_ref2, _) = EntityRef::new("Product");
        let (chunks2, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref2, &data, &configs, &cache,
        );

        assert_eq!(chunks1.len(), chunks2.len());
        for (c1, c2) in chunks1.iter().zip(chunks2.iter()) {
            let uuid1 = c1.data.get("_uuid").and_then(|v| v.as_str()).unwrap();
            let uuid2 = c2.data.get("_uuid").and_then(|v| v.as_str()).unwrap();
            assert_eq!(uuid1, uuid2, "chunk UUIDs should be deterministic");
        }
    }

    #[test]
    fn chunk_has_all_required_fields() {
        let configs = make_entity_configs();
        let config = configs.get("Product").unwrap();
        let cache = make_chunker_cache(config);
        let (entity_ref, _resolver) = EntityRef::new("Product");

        let data = make_product_data("Shoes", "A product description.", "");
        let (chunks, _) = ChunkRecordNode::compute_chunks(
            "Product", "uuid-1", &entity_ref, &data, &configs, &cache,
        );

        assert!(!chunks.is_empty());
        let chunk = &chunks[0];
        let required = [
            "_uuid", "_parent_uuid", "_parent_field", "_text", "_title",
            "_text_hash", "_embed_hash", "_index", "_start_char", "_end_char",
            "_start_line", "_end_line", "_core_start_char", "_core_end_char",
            "_core_start_line", "_core_end_line", "_content_offset",
        ];
        for field in &required {
            assert!(chunk.data.contains_key(*field), "missing field: {field}");
        }
    }

    // ── EmbedNode construction tests ──

    #[test]
    fn embed_node_default_columns() {
        let node = EmbedNode::new("test", search::SearchSignals::HYBRID, 64);
        assert_eq!(node.node_type(), "EmbedNode");
        assert_eq!(node.name(), "test");

        let config = node.node_config();
        assert_eq!(config["text_field"], "_text");
        assert_eq!(config["embedding_col"], "embedding");
        assert_eq!(config["sparse_col"], "sparse");
        assert_eq!(config["gpu_batch_size"], 64);
    }

    #[test]
    fn embed_node_custom_columns() {
        let node = EmbedNode::new("e", search::SearchSignals::VECTOR, 128)
            .with_columns("content", "my_emb", "my_sparse");
        let config = node.node_config();
        assert_eq!(config["text_field"], "content");
        assert_eq!(config["embedding_col"], "my_emb");
        assert_eq!(config["sparse_col"], "my_sparse");
    }

    #[test]
    fn embed_node_ports() {
        let node = EmbedNode::new("e", search::SearchSignals::HYBRID, 64);
        assert_eq!(node.inputs().len(), 2); // entities + trigger
        assert_eq!(node.outputs().len(), 1); // done
        assert_eq!(node.inputs()[0].name, "entities");
        assert_eq!(node.outputs()[0].name, "done");
    }
}

//! Record-based ingestion nodes (Phase B — doc 23).
//!
//! These replace the op-based batch nodes with typed record inputs.
//! The graph topology encodes the execution plan — records carry only data.
//! All nodes are **idempotent** — safe to re-run after crash or partial execution.
//!
//! - [`InsertRecordNode`] — UNWIND MERGE on `_uuid` from `Vec<EntityRecord>`
//! - [`LinkRecordNode`] — UNWIND MATCH+MERGE from `Vec<RelationRecord>`
//! - [`EmbedRecordNode`] — unified embedding with `_embed_hash` skip (saves GPU)
//! - [`ChunkRecordNode`] — parallel chunking, outputs chunk entities + links
//!   (entity-level chunks — future use with Mermaid templates: simple vs metaKB)
//! - [`GatherKBNode`] — read DB, detect content changes, output changed KBContentRecords
//! - [`UpdateKBNode`] — update KB_Index entries + delete stale chunks
//! - [`ChunkKBNode`] — generate chunk entities + relations from aggregated content

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::catalog::KBMetadata;
use crate::chunker::{Chunker, ChunkerConfig};
use crate::config::CatalogConfig;
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::embedder::{DualEmbedder, Embedder, SparseEmbedder};
use crate::hash::content_hash;
use crate::node_id_cache::{InternalNodeId, NodeIdCache};
use crate::ops::RefOrUuid;
use crate::records::{AggregateRecord, EntityRecord, KBContentRecord, RecordSourceContent, RelationRecord};
use crate::refs::{EntityRef, RelationRef};
use crate::search;
use crate::sparse_index::SparseVector;
use crate::uuid::chunk_uuid;

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
}

impl InsertRecordNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for InsertRecordNode {
    fn name(&self) -> &str {
        &self.name
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

            // Build UNWIND MERGE on _uuid + SET remaining columns (idempotent)
            let other_cols: Vec<&str> = col_refs.iter()
                .filter(|c| **c != "_uuid")
                .copied()
                .collect();
            let set_clause = if other_cols.is_empty() {
                String::new()
            } else {
                let assigns: String = other_cols.iter()
                    .map(|c| format!("n.{c} = item.{c}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(" SET {assigns}")
            };
            let cypher = format!(
                "UNWIND $items AS item \
                 MERGE (n:{entity_name} {{_uuid: item._uuid}}){set_clause} \
                 RETURN ID(n), item._uuid"
            );

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

        ctx.set_output("done", PortValue::Empty);
        ctx.set_output("inserted", PortValue::Batch(
            BatchPayload::new(PortType::Entities, items),
        ));
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
}

impl LinkRecordNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for LinkRecordNode {
    fn name(&self) -> &str {
        &self.name
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
            // Build UNWIND MATCH+MERGE (idempotent — skip if relation already exists)
            let prop_set = if prop_keys.is_empty() {
                String::new()
            } else {
                let assigns: String = prop_keys.iter()
                    .map(|k| format!("r.{k} = item.{k}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(" SET {assigns}")
            };
            let cypher = format!(
                "UNWIND $items AS item \
                 MATCH (a {{_uuid: item.from_uuid}}), (b {{_uuid: item.to_uuid}}) \
                 MERGE (a)-[r:{rel_name}]->(b){prop_set}"
            );

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

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── EmbedRecordNode ────────────────────────────────────────────────────────

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
pub struct EmbedRecordNode {
    name: String,
    gpu_batch_size: usize,
}

impl EmbedRecordNode {
    pub fn new(name: impl Into<String>, gpu_batch_size: usize) -> Self {
        Self {
            name: name.into(),
            gpu_batch_size,
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
impl Node for EmbedRecordNode {
    fn name(&self) -> &str {
        &self.name
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
                .ok_or("EmbedRecordNode: failed to extract Vec<EntityRecord>")?,
            _ => return Err("EmbedRecordNode: missing 'entities' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("EmbedRecordNode: 'conn' service not registered")?;
        let config = ctx.service::<CatalogConfig>("config")
            .ok_or("EmbedRecordNode: 'config' service not registered")?;
        let embedder = ctx.service::<Arc<dyn Embedder>>("embedder")
            .ok_or("EmbedRecordNode: 'embedder' service not registered")?;
        let embedding_dim = *ctx.service::<usize>("embedding_dim")
            .ok_or("EmbedRecordNode: 'embedding_dim' service not registered")?;
        let has_sparse_svc = ctx.service::<bool>("has_sparse").map(|v| *v).unwrap_or(false);
        let has_dual_svc = ctx.service::<bool>("has_dual").map(|v| *v).unwrap_or(false);
        let sparse_embedder = ctx.service::<Arc<dyn SparseEmbedder>>("sparse_embedder");
        let dual_embedder = ctx.service::<Arc<dyn DualEmbedder>>("dual_embedder");

        // For each chunk, extract the text to embed and determine signals.
        //
        // Chunks carry `_text`, `_kb_name`, and `_text_hash` in their data
        // (set by ChunkKBNode / generate_chunk_records). The KB name determines
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
                let cypher = format!(
                    "UNWIND $items AS item \
                     MATCH (n:{entity_name} {{_uuid: item.uuid}}) \
                     WHERE n._embed_hash IS NOT NULL \
                     RETURN n._uuid, n._embed_hash"
                );
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

                let cypher = format!(
                    "UNWIND $items AS item \
                     MATCH (n:{entity_name} {{_uuid: item.uuid}}) \
                     SET n.{col} = item.emb, n._embed_hash = item.hash"
                );

                conn.execute_with_params(
                    &cypher,
                    &[QueryParam { name: "items".into(), value: items_param }],
                ).await.map_err(|e| e.to_string())?;
            }
        }

        // ── Sparse embedding ──
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

                for ((entity_name, kb_name), group) in &groups {
                    let indices_col = format!("{kb_name}_sparse_indices");
                    let weights_col = format!("{kb_name}_sparse_weights");

                    let items_param = CypherValue::List(
                        group.iter().map(|(work, sv)| {
                            let mut map = BTreeMap::new();
                            map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                            map.insert("hash".into(), CypherValue::String(work.text_hash.clone()));
                            map.insert("indices".into(), CypherValue::List(
                                sv.indices.iter().map(|&i| CypherValue::Int(i as i64)).collect(),
                            ));
                            map.insert("weights".into(), CypherValue::List(
                                sv.values.iter().map(|&f| CypherValue::Float(f as f64)).collect(),
                            ));
                            CypherValue::Map(map)
                        }).collect(),
                    );

                    let cypher = format!(
                        "UNWIND $items AS item \
                         MATCH (n:{entity_name} {{_uuid: item.uuid}}) \
                         SET n.{indices_col} = item.indices, n.{weights_col} = item.weights, \
                         n._embed_hash = item.hash"
                    );

                    conn.execute_with_params(
                        &cypher,
                        &[QueryParam { name: "items".into(), value: items_param }],
                    ).await.map_err(|e| e.to_string())?;
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

                        let cypher = format!(
                            "UNWIND $items AS item \
                             MATCH (n:{entity_name} {{_uuid: item.uuid}}) \
                             SET n.{col} = item.emb, n._embed_hash = item.hash"
                        );

                        conn.execute_with_params(
                            &cypher,
                            &[QueryParam { name: "items".into(), value: items_param }],
                        ).await.map_err(|e| e.to_string())?;
                    }
                }

                // UNWIND sparse
                {
                    let mut groups: HashMap<(&str, &str), Vec<(&EmbedWork, &SparseVector)>> =
                        HashMap::new();
                    for (work, sv) in &sparse_results {
                        groups
                            .entry((&work.entity_name, work.kb_name.as_str()))
                            .or_default()
                            .push((work, sv));
                    }

                    for ((entity_name, kb_name), group) in &groups {
                        let indices_col = format!("{kb_name}_sparse_indices");
                        let weights_col = format!("{kb_name}_sparse_weights");

                        let items_param = CypherValue::List(
                            group.iter().map(|(work, sv)| {
                                let mut map = BTreeMap::new();
                                map.insert("uuid".into(), CypherValue::String(work.uuid.clone()));
                                map.insert("hash".into(), CypherValue::String(work.text_hash.clone()));
                                map.insert("indices".into(), CypherValue::List(
                                    sv.indices.iter().map(|&i| CypherValue::Int(i as i64)).collect(),
                                ));
                                map.insert("weights".into(), CypherValue::List(
                                    sv.values.iter().map(|&f| CypherValue::Float(f as f64)).collect(),
                                ));
                                CypherValue::Map(map)
                            }).collect(),
                        );

                        let cypher = format!(
                            "UNWIND $items AS item \
                             MATCH (n:{entity_name} {{_uuid: item.uuid}}) \
                             SET n.{indices_col} = item.indices, n.{weights_col} = item.weights, \
                             n._embed_hash = item.hash"
                        );

                        conn.execute_with_params(
                            &cypher,
                            &[QueryParam { name: "items".into(), value: items_param }],
                        ).await.map_err(|e| e.to_string())?;
                    }
                }
            }
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── ChunkRecordNode ────────────────────────────────────────────────────────

/// Parallel chunking: takes `Vec<EntityRecord>` (inserted entities), chunks
/// their content fields via rayon, outputs chunk entities + links.
///
/// **Input**: `entities` — `BatchPayload<EntityRecord>` (PortType::Entities)
/// **Output**: `done` — Empty, `chunks` — chunk entities, `chunk_links` — HAS_CHUNK relations
/// **Services**: `config`, `kb_metadata`, `chunker_cache`
pub struct ChunkRecordNode {
    name: String,
}

impl ChunkRecordNode {
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
impl Node for ChunkRecordNode {
    fn name(&self) -> &str {
        &self.name
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

        let config = ctx.service::<CatalogConfig>("config")
            .ok_or("ChunkRecordNode: 'config' service not registered")?;
        let kb_metadata = ctx.service::<HashMap<String, KBMetadata>>("kb_metadata")
            .ok_or("ChunkRecordNode: 'kb_metadata' service not registered")?;
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
        if !all_chunk_entities.is_empty() {
            ctx.set_output("chunks", PortValue::Batch(
                BatchPayload::new(PortType::Entities, all_chunk_entities),
            ));
        }
        if !all_chunk_relations.is_empty() {
            ctx.set_output("chunk_links", PortValue::Batch(
                BatchPayload::new(PortType::Relations, all_chunk_relations),
            ));
        }
        Ok(())
    }
}

// ─── KB Pipeline Nodes ──────────────────────────────────────────────────────
//
// These 3 nodes replace the monolithic AggregateRecordNode.
// Pipeline: GatherKBNode → UpdateKBNode → ChunkKBNode
//
// - GatherKBNode: read titles + linked content + hashes, detect changes
// - UpdateKBNode: SET on KB_Index + DETACH DELETE stale chunks (pass-through)
// - ChunkKBNode: generate chunk EntityRecords + RelationRecords (pure CPU)

/// Batch-collected state for one aggregate operation (internal to GatherKBNode).
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

// ─── GatherKBNode ───────────────────────────────────────────────────────────

/// Reads title entities + linked content from DB, compares content hashes,
/// outputs only changed `KBContentRecord`s.
///
/// **Input**: `aggregates` — `BatchPayload<AggregateRecord>` (PortType::Aggregates)
/// **Output**: `kb_content` — changed records, `done` — Empty signal
/// **Services**: `conn`, `config`, `kb_metadata`
pub struct GatherKBNode {
    name: String,
}

impl GatherKBNode {
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
impl Node for GatherKBNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "aggregates", port_type: PortType::Aggregates, required: true },
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
        let items: Vec<AggregateRecord> = match ctx.take_input("aggregates") {
            Some(PortValue::Batch(payload)) => payload
                .take::<AggregateRecord>()
                .ok_or("GatherKBNode: failed to extract Vec<AggregateRecord>")?,
            _ => return Err("GatherKBNode: missing 'aggregates' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("GatherKBNode: 'conn' service not registered")?;
        let config = ctx.service::<CatalogConfig>("config")
            .ok_or("GatherKBNode: 'config' service not registered")?;
        let kb_metadata = ctx.service::<HashMap<String, KBMetadata>>("kb_metadata")
            .ok_or("GatherKBNode: 'kb_metadata' service not registered")?;

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
}

// ─── UpdateKBNode ───────────────────────────────────────────────────────────

/// Updates `{KB}_Index` entries (SET _title, _content, _content_hash)
/// and deletes stale `{KB}_Index_Chunk` entities. Passes through
/// the input `KBContentRecord`s unchanged for downstream chunking.
///
/// **Input**: `kb_content` — `BatchPayload<KBContentRecord>` (PortType::KBContent)
/// **Output**: `kb_content` — same records (pass-through), `done` — Empty signal
/// **Services**: `conn`
pub struct UpdateKBNode {
    name: String,
}

impl UpdateKBNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for UpdateKBNode {
    fn name(&self) -> &str {
        &self.name
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
                .ok_or("UpdateKBNode: failed to extract Vec<KBContentRecord>")?,
            _ => return Err("UpdateKBNode: missing 'kb_content' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("UpdateKBNode: 'conn' service not registered")?;

        // Group by kb_name for UNWIND batching
        let mut groups: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, rec) in items.iter().enumerate() {
            groups.entry(&rec.kb_name).or_default().push(i);
        }

        let mut total_updated: usize = 0;
        let mut total_deleted: usize = 0;

        for (kb_name, indices) in &groups {
            let index_table = format!("{kb_name}_Index");
            let chunk_table = format!("{kb_name}_Index_Chunk");

            // Step 5: UNWIND UPDATE changed indexes
            {
                let items_param = CypherValue::List(
                    indices.iter().map(|&i| {
                        let rec = &items[i];
                        let mut m = BTreeMap::new();
                        m.insert("uuid".into(), CypherValue::String(rec.index_entry_uuid.clone()));
                        m.insert("title".into(), CypherValue::String(rec.title_text.clone()));
                        m.insert("content".into(), CypherValue::String(rec.content_text.clone()));
                        m.insert("hash".into(), CypherValue::String(rec.new_hash.clone()));
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
                ).await.map_err(|e| e.to_string())?;
                total_updated += indices.len();
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

        ctx.log_metric("items", items.len());
        ctx.log_metric("updated", total_updated);
        ctx.log_metric("deleted", total_deleted);

        ctx.set_output("done", PortValue::Empty);
        ctx.set_output("kb_content", PortValue::Batch(
            BatchPayload::new(PortType::KBContent, items),
        ));
        Ok(())
    }
}

// ─── ChunkKBNode ────────────────────────────────────────────────────────────

/// Generates chunk EntityRecords + RelationRecords (HAS_CHUNK, SOURCED)
/// from changed KB content. Pure CPU work — no DB queries.
///
/// **Input**: `kb_content` — `BatchPayload<KBContentRecord>` (PortType::KBContent)
/// **Output**: `entities` — chunk records, `relations` — links, `done` — Empty
/// **Services**: `chunker_cache`
pub struct ChunkKBNode {
    name: String,
}

impl ChunkKBNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for ChunkKBNode {
    fn name(&self) -> &str {
        &self.name
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
                .ok_or("ChunkKBNode: failed to extract Vec<KBContentRecord>")?,
            _ => return Err("ChunkKBNode: missing 'kb_content' input".to_string()),
        };

        let chunker_cache = ctx.service::<HashMap<ChunkerConfig, Chunker>>("chunker_cache")
            .ok_or("ChunkKBNode: 'chunker_cache' service not registered")?;
        let kb_metadata = ctx.service::<HashMap<String, KBMetadata>>("kb_metadata")
            .ok_or("ChunkKBNode: 'kb_metadata' service not registered")?;

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
}

// ─── FlushFTSNode ───────────────────────────────────────────────────────────

/// Flushes Lucivy FTS indexes for all KBs touched during ingestion.
///
/// Called after `UpdateKBNode` (which triggers Lucivy update hooks via SET).
/// Runs `CALL FLUSH_LUCIVY_INDEX('{kb}_Index')` to commit + reload the reader
/// so subsequent searches don't pay the lazy-flush cost.
///
/// **Input**: `trigger` — Empty signal (optional, from update_kb.done)
/// **Output**: `done` — Empty signal
/// **Services**: `conn` (DbConnection), `flush_kb_names` (Vec<String>)
pub struct FlushFTSNode {
    name: String,
}

impl FlushFTSNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for FlushFTSNode {
    fn name(&self) -> &str {
        &self.name
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
            .ok_or("FlushFTSNode: 'conn' service not registered")?;
        let kb_names = ctx.service::<Vec<String>>("flush_kb_names")
            .ok_or("FlushFTSNode: 'flush_kb_names' service not registered")?;

        let mut flushed: usize = 0;
        for kb_name in kb_names.iter() {
            let table = format!("{kb_name}_Index");
            if conn.execute(&format!("CALL FLUSH_LUCIVY_INDEX('{table}')")).await.is_ok() {
                flushed += 1;
            }
        }

        ctx.log_metric("kb_count", kb_names.len());
        ctx.log_metric("flushed", flushed);
        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

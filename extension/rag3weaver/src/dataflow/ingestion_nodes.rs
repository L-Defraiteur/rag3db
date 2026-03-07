//! Ingestion nodes for the dataflow graph.
//!
//! Replaces the 7 `Processor` structs in `catalog.rs` with typed dataflow nodes.
//! Each node uses `PortType::Empty` for trigger/done signaling (Option B from doc 15).
//! Data is baked into node constructors, services passed via `Arc`.
//!
//! - [`InsertBatchNode`] — batch INSERT via Cypher
//! - [`LinkBatchNode`] — batch MATCH+CREATE for relations
//! - [`EmbedBatchNode`] — batch dense embedding + UNWIND
//! - [`SparseEmbedBatchNode`] — batch sparse embedding + UNWIND
//! - [`DualEmbedBatchNode`] — dual dense+sparse in mini-batches
//! - [`ChunkBatchNode`] — parallel chunking, emits downstream nodes (DynamicNode)
//! - [`AggregateBatchNode`] — rebuild _content, re-chunk, emit downstream (DynamicNode)

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
use crate::ops::{
    CatalogOp, DualEmbedOp, EmbedOp, InsertOp, LinkOp, RefOrUuid, SparseEmbedOp,
    PRIO_POST_AGG_INSERT, PRIO_POST_AGG_LINK,
};
use crate::refs::{EntityRef, RelationRef};
use crate::schema::generate_insert_cypher;
use crate::search;
use crate::sparse_index::SparseVector;
use crate::uuid::chunk_uuid;

use super::node::{DynamicNode, GraphEmitter, Node, NodeContext};
use super::port::{PortDef, PortType, PortValue};

// ─── SplitOpsNode ───────────────────────────────────────────────────────────

/// Routes a `Vec<CatalogOp>` into typed output ports.
///
/// Input: `ops` — `BatchPayload` containing `Vec<CatalogOp>`
/// Outputs: `inserts`, `links`, `chunks`, `aggregates`, `embeds`, `sparse_embeds`, `dual_embeds`
///
/// Each output is a `BatchPayload` wrapping the corresponding `Vec<T>`.
/// Empty batches produce no output on that port.
pub struct SplitOpsNode;

#[async_trait]
impl Node for SplitOpsNode {
    fn name(&self) -> &str {
        "split_ops"
    }
    fn inputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "ops",
            port_type: PortType::Ops,
            required: true,
        }]
    }
    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "inserts",       port_type: PortType::Inserts,      required: false },
            PortDef { name: "links",          port_type: PortType::Links,        required: false },
            PortDef { name: "chunks",         port_type: PortType::Chunks,       required: false },
            PortDef { name: "aggregates",     port_type: PortType::Aggregates,   required: false },
            PortDef { name: "embeds",         port_type: PortType::Embeds,       required: false },
            PortDef { name: "sparse_embeds",  port_type: PortType::SparseEmbeds, required: false },
            PortDef { name: "dual_embeds",    port_type: PortType::DualEmbeds,   required: false },
        ]
    }
    async fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let ops: Vec<CatalogOp> = match ctx.take_input("ops") {
            Some(PortValue::Batch(payload)) => payload
                .take::<CatalogOp>()
                .ok_or("SplitOpsNode: failed to extract Vec<CatalogOp> from BatchPayload")?,
            _ => return Err("SplitOpsNode: missing or invalid 'ops' input".to_string()),
        };

        let mut inserts: Vec<InsertOp> = Vec::new();
        let mut links: Vec<LinkOp> = Vec::new();
        let mut chunks: Vec<crate::ops::ChunkOp> = Vec::new();
        let mut aggregates: Vec<crate::ops::AggregateOp> = Vec::new();
        let mut embeds: Vec<EmbedOp> = Vec::new();
        let mut sparse_embeds: Vec<SparseEmbedOp> = Vec::new();
        let mut dual_embeds: Vec<DualEmbedOp> = Vec::new();

        for op in ops {
            match op {
                CatalogOp::Insert(o) => inserts.push(o),
                CatalogOp::Link(o) => links.push(o),
                CatalogOp::Chunk(o) => chunks.push(o),
                CatalogOp::Aggregate(o) => aggregates.push(o),
                CatalogOp::Embed(o) => embeds.push(o),
                CatalogOp::SparseEmbed(o) => sparse_embeds.push(o),
                CatalogOp::DualEmbed(o) => dual_embeds.push(o),
            }
        }

        use super::port::BatchPayload;

        eprintln!(
            "[SplitOpsNode] routed: inserts={}, links={}, chunks={}, aggregates={}, embeds={}, sparse={}, dual={}",
            inserts.len(), links.len(), chunks.len(), aggregates.len(),
            embeds.len(), sparse_embeds.len(), dual_embeds.len(),
        );

        if !inserts.is_empty() {
            ctx.set_output("inserts", PortValue::Batch(
                BatchPayload::new(PortType::Inserts, inserts),
            ));
        }
        if !links.is_empty() {
            ctx.set_output("links", PortValue::Batch(
                BatchPayload::new(PortType::Links, links),
            ));
        }
        if !chunks.is_empty() {
            ctx.set_output("chunks", PortValue::Batch(
                BatchPayload::new(PortType::Chunks, chunks),
            ));
        }
        if !aggregates.is_empty() {
            ctx.set_output("aggregates", PortValue::Batch(
                BatchPayload::new(PortType::Aggregates, aggregates),
            ));
        }
        if !embeds.is_empty() {
            ctx.set_output("embeds", PortValue::Batch(
                BatchPayload::new(PortType::Embeds, embeds),
            ));
        }
        if !sparse_embeds.is_empty() {
            ctx.set_output("sparse_embeds", PortValue::Batch(
                BatchPayload::new(PortType::SparseEmbeds, sparse_embeds),
            ));
        }
        if !dual_embeds.is_empty() {
            ctx.set_output("dual_embeds", PortValue::Batch(
                BatchPayload::new(PortType::DualEmbeds, dual_embeds),
            ));
        }

        Ok(())
    }
}

// ─── InsertBatchNode ────────────────────────────────────────────────────────

/// Batch INSERT: executes Cypher CREATE for each InsertOp, resolves EntityRefs,
/// caches internal node IDs.
///
/// **Input**: `ops` — `BatchPayload<InsertOp>`
/// **Services**: `conn` (DbConnection), `node_id_cache` (RwLock<NodeIdCache>)
pub struct InsertBatchNode {
    name: String,
}

impl InsertBatchNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for InsertBatchNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "ops", port_type: PortType::Inserts, required: true },
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
        let mut items: Vec<InsertOp> = match ctx.take_input("ops") {
            Some(PortValue::Batch(payload)) => payload
                .take::<InsertOp>()
                .ok_or("InsertBatchNode: failed to extract Vec<InsertOp>")?,
            _ => return Err("InsertBatchNode: missing 'ops' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("InsertBatchNode: 'conn' service not registered")?;
        let node_id_cache = ctx.service::<RwLock<NodeIdCache>>("node_id_cache")
            .ok_or("InsertBatchNode: 'node_id_cache' service not registered")?;

        // Group by (entity_name, sorted column_set) for UNWIND batching.
        // Key = (entity_name, columns), Value = indices into `items`.
        let mut groups: HashMap<(String, Vec<String>), Vec<usize>> = HashMap::new();
        for (i, insert) in items.iter().enumerate() {
            let mut columns: Vec<String> = insert.data.keys().cloned().collect();
            columns.sort();
            groups
                .entry((insert.entity_name.clone(), columns))
                .or_default()
                .push(i);
        }

        let group_summary: Vec<String> = groups
            .iter()
            .map(|((name, _), idxs)| format!("{}×{}", name, idxs.len()))
            .collect();
        eprintln!(
            "[InsertBatchNode:{}] {} items → {} UNWIND groups: [{}]",
            self.name, items.len(), groups.len(), group_summary.join(", "),
        );

        for ((entity_name, columns), indices) in &groups {
            let col_refs: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();

            // Build UNWIND CREATE ... RETURN ID(n), item._uuid
            let set_clauses: String = col_refs
                .iter()
                .map(|c| format!("{c}: item.{c}"))
                .collect::<Vec<_>>()
                .join(", ");
            let cypher = format!(
                "UNWIND $items AS item \
                 CREATE (n:{entity_name} {{{set_clauses}}}) \
                 RETURN ID(n), item._uuid"
            );

            // Build items list param
            let items_param = CypherValue::List(
                indices
                    .iter()
                    .map(|&i| {
                        let insert = &items[i];
                        let mut map = BTreeMap::new();
                        for col in &col_refs {
                            map.insert(
                                col.to_string(),
                                insert
                                    .data
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

            // Build UUID → row index map for safe matching (don't rely on row order)
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
                let insert = &mut items[i];
                let uuid = insert
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

                if let Some(resolver) = insert.take_resolver() {
                    resolver.resolve(uuid);
                }
            }
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── LinkBatchNode ──────────────────────────────────────────────────────────

/// Batch LINK: executes MATCH+CREATE for each LinkOp, resolves from/to refs.
///
/// **Input**: `ops` — `BatchPayload<LinkOp>`
/// **Services**: `conn` (DbConnection)
pub struct LinkBatchNode {
    name: String,
}

impl LinkBatchNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for LinkBatchNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "ops", port_type: PortType::Links, required: true },
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
        let mut items: Vec<LinkOp> = match ctx.take_input("ops") {
            Some(PortValue::Batch(payload)) => payload
                .take::<LinkOp>()
                .ok_or("LinkBatchNode: failed to extract Vec<LinkOp>")?,
            _ => return Err("LinkBatchNode: missing 'ops' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("LinkBatchNode: 'conn' service not registered")?;

        // Resolve all refs first (should be instant — InsertBatchNode already completed)
        struct ResolvedLink {
            from_uuid: String,
            to_uuid: String,
            index: usize,
        }
        let mut resolved: Vec<ResolvedLink> = Vec::with_capacity(items.len());
        for (i, link) in items.iter_mut().enumerate() {
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
            resolved.push(ResolvedLink { from_uuid, to_uuid, index: i });
        }

        // Group by (rel_name, sorted property keys) for UNWIND batching.
        let mut groups: HashMap<(String, Vec<String>), Vec<usize>> = HashMap::new();
        for (ri, rl) in resolved.iter().enumerate() {
            let link = &items[rl.index];
            let mut prop_keys: Vec<String> = link.properties.keys().cloned().collect();
            prop_keys.sort();
            groups
                .entry((link.rel_name.clone(), prop_keys))
                .or_default()
                .push(ri);
        }

        let group_summary: Vec<String> = groups
            .iter()
            .map(|((name, _), idxs)| format!("{}×{}", name, idxs.len()))
            .collect();
        eprintln!(
            "[LinkBatchNode:{}] {} links → {} UNWIND groups: [{}]",
            self.name, items.len(), groups.len(), group_summary.join(", "),
        );

        for ((rel_name, prop_keys), indices) in &groups {
            // Build UNWIND MATCH+CREATE
            let mut cypher = format!(
                "UNWIND $items AS item \
                 MATCH (a {{_uuid: item.from_uuid}}), (b {{_uuid: item.to_uuid}}) \
                 CREATE (a)-[:{rel_name}"
            );
            if !prop_keys.is_empty() {
                let prop_strs: Vec<String> =
                    prop_keys.iter().map(|k| format!("{k}: item.{k}")).collect();
                cypher.push_str(&format!(" {{{}}}", prop_strs.join(", ")));
            }
            cypher.push_str("]->(b)");

            let items_param = CypherValue::List(
                indices
                    .iter()
                    .map(|&ri| {
                        let rl = &resolved[ri];
                        let link = &items[rl.index];
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
                                link.properties.get(key).cloned().unwrap_or(CypherValue::Null),
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
                let link = &mut items[rl.index];
                if let Some(resolver) = link.take_resolver() {
                    resolver.resolve(rl.from_uuid.clone(), rl.to_uuid.clone());
                }
            }
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── EmbedBatchNode ─────────────────────────────────────────────────────────

/// Batch dense embedding: resolves refs, calls Embedder::embed(), UNWIND SET.
///
/// **Input**: `ops` — `BatchPayload<EmbedOp>`
/// **Services**: `conn` (DbConnection), `embedder` (Embedder), `embedding_dim` (usize)
pub struct EmbedBatchNode {
    name: String,
}

impl EmbedBatchNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for EmbedBatchNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "ops", port_type: PortType::Embeds, required: true },
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
        struct EmbedWork {
            uuid: String,
            text: String,
            entity_name: String,
            embedding_col: String,
        }

        let mut items: Vec<EmbedOp> = match ctx.take_input("ops") {
            Some(PortValue::Batch(payload)) => payload
                .take::<EmbedOp>()
                .ok_or("EmbedBatchNode: failed to extract Vec<EmbedOp>")?,
            _ => return Err("EmbedBatchNode: missing 'ops' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("EmbedBatchNode: 'conn' service not registered")?;
        let embedder = ctx.service::<Arc<dyn Embedder>>("embedder")
            .ok_or("EmbedBatchNode: 'embedder' service not registered")?;
        let embedding_dim = *ctx.service::<usize>("embedding_dim")
            .ok_or("EmbedBatchNode: 'embedding_dim' service not registered")?;

        let mut works = Vec::new();
        for embed in items.iter_mut() {
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

        if !works.is_empty() {
            let texts: Vec<String> = works.iter().map(|w| w.text.clone()).collect();
            let vectors = embedder
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

            // Group by (entity_name, embedding_col)
            let mut groups: HashMap<(&str, &str), Vec<(&EmbedWork, &Vec<f32>)>> = HashMap::new();
            for (work, vector) in works.iter().zip(vectors.iter()) {
                if vector.len() != embedding_dim {
                    return Err(format!(
                        "embedding dimension mismatch: expected {}, got {}",
                        embedding_dim,
                        vector.len()
                    ));
                }
                groups
                    .entry((&work.entity_name, &work.embedding_col))
                    .or_default()
                    .push((work, vector));
            }

            let group_summary: Vec<String> = groups
                .iter()
                .map(|((ent, col), g)| format!("{}.{}×{}", ent, col, g.len()))
                .collect();
            eprintln!(
                "[EmbedBatchNode:{}] {} texts embedded → {} UNWIND groups: [{}]",
                self.name, works.len(), groups.len(), group_summary.join(", "),
            );

            for ((entity_name, col), group) in &groups {
                let items_param = CypherValue::List(
                    group
                        .iter()
                        .map(|(work, vec)| {
                            let mut map = BTreeMap::new();
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
            }
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── SparseEmbedBatchNode ───────────────────────────────────────────────────

/// Batch sparse embedding: resolves refs, calls SparseEmbedder, UNWIND SET.
///
/// **Input**: `ops` — `BatchPayload<SparseEmbedOp>`
/// **Services**: `conn` (DbConnection), `sparse_embedder` (SparseEmbedder)
pub struct SparseEmbedBatchNode {
    name: String,
}

impl SparseEmbedBatchNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Node for SparseEmbedBatchNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "ops", port_type: PortType::SparseEmbeds, required: true },
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
        struct SparseWork {
            uuid: String,
            text: String,
            entity_name: String,
            kb_name: String,
        }

        let mut items: Vec<SparseEmbedOp> = match ctx.take_input("ops") {
            Some(PortValue::Batch(payload)) => payload
                .take::<SparseEmbedOp>()
                .ok_or("SparseEmbedBatchNode: failed to extract Vec<SparseEmbedOp>")?,
            _ => return Err("SparseEmbedBatchNode: missing 'ops' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("SparseEmbedBatchNode: 'conn' service not registered")?;
        let sparse_embedder = ctx.service::<Arc<dyn SparseEmbedder>>("sparse_embedder")
            .ok_or("SparseEmbedBatchNode: 'sparse_embedder' service not registered")?;

        let mut works = Vec::new();
        for op in items.iter_mut() {
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

        if !works.is_empty() {
            let texts: Vec<String> = works.iter().map(|w| w.text.clone()).collect();
            let sparse_vecs = sparse_embedder
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

            let mut groups: HashMap<(&str, &str), Vec<(&SparseWork, &SparseVector)>> =
                HashMap::new();
            for (work, sv) in works.iter().zip(sparse_vecs.iter()) {
                groups
                    .entry((&work.entity_name, &work.kb_name))
                    .or_default()
                    .push((work, sv));
            }

            let group_summary: Vec<String> = groups
                .iter()
                .map(|((ent, kb), g)| format!("{}.{}×{}", ent, kb, g.len()))
                .collect();
            eprintln!(
                "[SparseEmbedBatchNode:{}] {} texts → {} UNWIND groups: [{}]",
                self.name, works.len(), groups.len(), group_summary.join(", "),
            );

            for ((entity_name, kb_name), group) in &groups {
                let indices_col = format!("{kb_name}_sparse_indices");
                let weights_col = format!("{kb_name}_sparse_weights");

                let items_param = CypherValue::List(
                    group
                        .iter()
                        .map(|(work, sv)| {
                            let mut map = BTreeMap::new();
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
            }
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── DualEmbedBatchNode ─────────────────────────────────────────────────────

/// Dual dense+sparse embedding in GPU mini-batches, then batch UNWIND.
///
/// **Input**: `ops` — `BatchPayload<DualEmbedOp>`
/// **Services**: `conn` (DbConnection), `dual_embedder` (DualEmbedder), `embedding_dim` (usize)
pub struct DualEmbedBatchNode {
    name: String,
    gpu_batch_size: usize,
}

impl DualEmbedBatchNode {
    pub fn new(name: impl Into<String>, gpu_batch_size: usize) -> Self {
        Self {
            name: name.into(),
            gpu_batch_size,
        }
    }
}

#[async_trait]
impl Node for DualEmbedBatchNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "ops", port_type: PortType::DualEmbeds, required: true },
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
        struct DualWork {
            uuid: String,
            text: String,
            entity_name: String,
            kb_name: String,
        }

        let mut items: Vec<DualEmbedOp> = match ctx.take_input("ops") {
            Some(PortValue::Batch(payload)) => payload
                .take::<DualEmbedOp>()
                .ok_or("DualEmbedBatchNode: failed to extract Vec<DualEmbedOp>")?,
            _ => return Err("DualEmbedBatchNode: missing 'ops' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("DualEmbedBatchNode: 'conn' service not registered")?;
        let dual_embedder = ctx.service::<Arc<dyn DualEmbedder>>("dual_embedder")
            .ok_or("DualEmbedBatchNode: 'dual_embedder' service not registered")?;
        let embedding_dim = *ctx.service::<usize>("embedding_dim")
            .ok_or("DualEmbedBatchNode: 'embedding_dim' service not registered")?;

        let mut works = Vec::new();
        for op in items.iter_mut() {
            let uuid = op
                .entity_ref
                .ready()
                .await
                .map_err(|e| format!("dual embed ref resolution failed: {e}"))?;

            if op.texts.is_empty() {
                continue;
            }

            works.push(DualWork {
                uuid,
                text: op.texts.join("\n"),
                entity_name: op.entity_ref.entity().to_string(),
                kb_name: op.kb_name.clone(),
            });
        }

        if works.is_empty() {
            ctx.set_output("done", PortValue::Empty);
            return Ok(());
        }

        eprintln!(
            "[DualEmbedBatchNode:{}] {} texts, gpu_batch_size={}",
            self.name, works.len(), self.gpu_batch_size,
        );

        // GPU embedding in mini-batches
        let mut dense_results: Vec<(&DualWork, Vec<f32>)> = Vec::with_capacity(works.len());
        let mut sparse_results: Vec<(&DualWork, SparseVector)> = Vec::with_capacity(works.len());

        for chunk in works.chunks(self.gpu_batch_size) {
            let texts: Vec<String> = chunk.iter().map(|w| w.text.clone()).collect();
            let (dense_vecs, sparse_vecs) = dual_embedder
                .embed_dual(&texts)
                .await
                .map_err(|e| format!("dual embed failed: {e}"))?;

            if dense_vecs.len() != chunk.len() || sparse_vecs.len() != chunk.len() {
                return Err(format!(
                    "dual embedder returned {}/{} vectors for {} texts",
                    dense_vecs.len(),
                    sparse_vecs.len(),
                    chunk.len()
                ));
            }

            let base_idx = dense_results.len();
            for (i, (dense, sparse)) in
                dense_vecs.into_iter().zip(sparse_vecs.into_iter()).enumerate()
            {
                dense_results.push((&works[base_idx + i], dense));
                sparse_results.push((&works[base_idx + i], sparse));
            }
        }

        // UNWIND dense
        {
            let mut groups: HashMap<(&str, String), Vec<(&DualWork, &Vec<f32>)>> = HashMap::new();
            for (work, vec) in &dense_results {
                if vec.len() != embedding_dim {
                    return Err(format!(
                        "embedding dimension mismatch: expected {}, got {}",
                        embedding_dim,
                        vec.len()
                    ));
                }
                let col = format!("{}_embedding", work.kb_name);
                groups
                    .entry((&work.entity_name, col))
                    .or_default()
                    .push((work, vec));
            }

            for ((entity_name, col), group) in &groups {
                let items_param = CypherValue::List(
                    group
                        .iter()
                        .map(|(work, vec)| {
                            let mut map = BTreeMap::new();
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
            }
        }

        // UNWIND sparse
        {
            let mut groups: HashMap<(&str, &str), Vec<(&DualWork, &SparseVector)>> =
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
                    group
                        .iter()
                        .map(|(work, sv)| {
                            let mut map = BTreeMap::new();
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
            }
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── ChunkBatchNode ─────────────────────────────────────────────────────────

/// DynamicNode: processes ChunkOps via rayon parallel chunking, then emits
/// InsertBatchNode + LinkBatchNode + EmbedBatchNode downstream.
///
/// **Input**: `ops` — `BatchPayload<ChunkOp>`
/// **Services**: `conn`, `node_id_cache`, `embedder`, `embedding_dim`,
///               `config` (CatalogConfig), `kb_metadata`, `chunker_cache`,
///               optionally `sparse_embedder`, `dual_embedder`
pub struct ChunkBatchNode;

#[async_trait]
impl DynamicNode for ChunkBatchNode {
    fn name(&self) -> &str {
        "chunk_batch"
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "ops", port_type: PortType::Chunks, required: true },
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
    async fn execute_dynamic(
        &mut self,
        ctx: &mut NodeContext,
        emitter: &mut GraphEmitter,
    ) -> Result<(), String> {
        use rayon::prelude::*;
        use super::port::BatchPayload;

        let items: Vec<crate::ops::ChunkOp> = match ctx.take_input("ops") {
            Some(PortValue::Batch(payload)) => payload
                .take::<crate::ops::ChunkOp>()
                .ok_or("ChunkBatchNode: failed to extract Vec<ChunkOp>")?,
            _ => return Err("ChunkBatchNode: missing 'ops' input".to_string()),
        };

        let config = ctx.service::<CatalogConfig>("config")
            .ok_or("ChunkBatchNode: 'config' service not registered")?;
        let kb_metadata = ctx.service::<HashMap<String, KBMetadata>>("kb_metadata")
            .ok_or("ChunkBatchNode: 'kb_metadata' service not registered")?;
        let chunker_cache = ctx.service::<HashMap<ChunkerConfig, Chunker>>("chunker_cache")
            .ok_or("ChunkBatchNode: 'chunker_cache' service not registered")?;
        let has_sparse = ctx.service::<bool>("has_sparse").map(|v| *v).unwrap_or(false);
        let has_dual = ctx.service::<bool>("has_dual").map(|v| *v).unwrap_or(false);

        // Parallel chunking via rayon
        let all_downstream: Vec<Vec<CatalogOp>> = items
            .par_iter()
            .map(|chunk_op| {
                crate::catalog::compute_chunk_ops(
                    &chunk_op.entity_name,
                    &chunk_op.parent_uuid,
                    &chunk_op.entity_ref,
                    &chunk_op.data,
                    &config,
                    &kb_metadata,
                    &chunker_cache,
                    has_sparse,
                    has_dual,
                )
            })
            .collect();

        // Flatten and partition downstream ops by type
        let mut inserts: Vec<InsertOp> = Vec::new();
        let mut links: Vec<LinkOp> = Vec::new();
        let mut embeds: Vec<EmbedOp> = Vec::new();
        let mut sparse_embeds: Vec<SparseEmbedOp> = Vec::new();
        let mut dual_embeds: Vec<DualEmbedOp> = Vec::new();

        for ops in all_downstream {
            for op in ops {
                match op {
                    CatalogOp::Insert(o) => inserts.push(o),
                    CatalogOp::Link(o) => links.push(o),
                    CatalogOp::Embed(o) => embeds.push(o),
                    CatalogOp::SparseEmbed(o) => sparse_embeds.push(o),
                    CatalogOp::DualEmbed(o) => dual_embeds.push(o),
                    _ => {}
                }
            }
        }

        eprintln!(
            "[ChunkBatchNode] {} chunk_ops → downstream: inserts={}, links={}, embeds={}, sparse={}, dual={}",
            items.len(), inserts.len(), links.len(), embeds.len(),
            sparse_embeds.len(), dual_embeds.len(),
        );

        // Emit downstream nodes with data via set_initial_input
        if !inserts.is_empty() {
            emitter.add_node(Box::new(InsertBatchNode::new("chunk_inserts")));
            emitter.set_initial_input("chunk_inserts", "ops",
                PortValue::Batch(BatchPayload::new(PortType::Inserts, inserts)));
        }

        if !links.is_empty() {
            emitter.add_node(Box::new(LinkBatchNode::new("chunk_links")));
            emitter.set_initial_input("chunk_links", "ops",
                PortValue::Batch(BatchPayload::new(PortType::Links, links)));
            // Links depend on inserts (need resolved UUIDs)
            if emitter.added_nodes.iter().any(|n| n.name() == "chunk_inserts") {
                emitter.connect("chunk_inserts", "done", "chunk_links", "trigger");
            }
        }

        // Embed nodes depend on inserts (need entities in DB)
        let embed_trigger = if emitter.added_nodes.iter().any(|n| n.name() == "chunk_inserts") {
            "chunk_inserts"
        } else {
            "chunk_batch"
        };

        if !dual_embeds.is_empty() {
            emitter.add_node(Box::new(DualEmbedBatchNode::new("chunk_dual_embeds", 32)));
            emitter.set_initial_input("chunk_dual_embeds", "ops",
                PortValue::Batch(BatchPayload::new(PortType::DualEmbeds, dual_embeds)));
            emitter.connect(embed_trigger, "done", "chunk_dual_embeds", "trigger");
        }

        if !embeds.is_empty() {
            emitter.add_node(Box::new(EmbedBatchNode::new("chunk_embeds")));
            emitter.set_initial_input("chunk_embeds", "ops",
                PortValue::Batch(BatchPayload::new(PortType::Embeds, embeds)));
            emitter.connect(embed_trigger, "done", "chunk_embeds", "trigger");
        }

        if !sparse_embeds.is_empty() {
            emitter.add_node(Box::new(SparseEmbedBatchNode::new("chunk_sparse_embeds")));
            emitter.set_initial_input("chunk_sparse_embeds", "ops",
                PortValue::Batch(BatchPayload::new(PortType::SparseEmbeds, sparse_embeds)));
            emitter.connect(embed_trigger, "done", "chunk_sparse_embeds", "trigger");
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── AggregateBatchNode ─────────────────────────────────────────────────────

/// Content collected from a single source field of a contributing entity.
struct SourceContent {
    entity_name: String,
    entity_uuid: String,
    field_name: String,
    text: String,
}

/// Batch-collected state for one aggregate operation.
struct AggState {
    source_uuid: String,
    index_entry_uuid: String,
    title_text: String,
    sources: Vec<SourceContent>,
    current_hash: String,
    found: bool,
}

/// DynamicNode: rebuilds `_content` on `{KB}_Index`, deletes stale chunks,
/// re-chunks per source field, and emits InsertBatchNode + LinkBatchNode +
/// EmbedBatchNode downstream.
///
/// **Input**: `ops` — `BatchPayload<AggregateOp>`
/// **Services**: `conn`, `config` (CatalogConfig), `kb_metadata`, `chunker_cache`,
///               `has_sparse` (bool), `has_dual` (bool)
pub struct AggregateBatchNode;

impl AggregateBatchNode {
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

    /// Generate downstream chunk ops for one changed aggregate.
    /// Pure CPU work — no DB queries.
    fn generate_chunk_ops(
        kb_name: &str,
        kb_signals: search::SearchSignals,
        kb_sparse: bool,
        has_dual: bool,
        index_entry_uuid: &str,
        title_text: &str,
        sources: &[SourceContent],
        chunker: &Chunker,
        chunk_table: &str,
    ) -> Vec<CatalogOp> {
        let mut downstream_ops: Vec<CatalogOp> = Vec::new();
        let mut content_offset: usize = 0;

        for source in sources {
            let chunks = chunker.chunk(&source.text);
            for chunk in &chunks {
                let source_key = format!("{}:{}", source.entity_uuid, source.field_name);
                let c_uuid = chunk_uuid(index_entry_uuid, &source_key, chunk.index);

                let embed_text = if !title_text.is_empty() {
                    format!("{title_text}\n---\n{}", chunk.text)
                } else {
                    chunk.text.clone()
                };

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
                downstream_ops.push(CatalogOp::Insert(
                    InsertOp::new(chunk_table.to_string(), chunk_data, {
                        let (_discard_ref, resolver) = EntityRef::new(chunk_table);
                        resolver
                    }, chunk_ref.clone())
                    .with_priority(PRIO_POST_AGG_INSERT),
                ));

                let has_chunk_rel = format!("{kb_name}_Index_HAS_CHUNK");
                let (_link_ref, link_resolver) = RelationRef::new(&has_chunk_rel);
                downstream_ops.push(CatalogOp::Link(
                    LinkOp::new(
                        has_chunk_rel,
                        RefOrUuid::Uuid(index_entry_uuid.to_string()),
                        RefOrUuid::Uuid(c_uuid.clone()),
                        BTreeMap::new(),
                        link_resolver,
                        _link_ref,
                    )
                    .with_priority(PRIO_POST_AGG_LINK),
                ));

                let sourced_rel = format!("{}_SOURCED_{kb_name}", source.entity_name);
                let (_src_ref, src_resolver) = RelationRef::new(&sourced_rel);
                downstream_ops.push(CatalogOp::Link(
                    LinkOp::new(
                        sourced_rel,
                        RefOrUuid::Uuid(source.entity_uuid.clone()),
                        RefOrUuid::Uuid(c_uuid.clone()),
                        BTreeMap::new(),
                        src_resolver,
                        _src_ref,
                    )
                    .with_priority(PRIO_POST_AGG_LINK),
                ));

                if has_dual && kb_signals.vector() && kb_sparse {
                    downstream_ops.push(CatalogOp::DualEmbed(DualEmbedOp {
                        entity_ref: chunk_ref,
                        kb_name: kb_name.to_string(),
                        texts: vec![embed_text],
                    }));
                } else {
                    if kb_signals.vector() {
                        downstream_ops.push(CatalogOp::Embed(EmbedOp {
                            entity_ref: chunk_ref.clone(),
                            kb_name: kb_name.to_string(),
                            texts: vec![embed_text.clone()],
                        }));
                    }
                    if kb_sparse {
                        downstream_ops.push(CatalogOp::SparseEmbed(SparseEmbedOp {
                            entity_ref: chunk_ref,
                            kb_name: kb_name.to_string(),
                            texts: vec![embed_text],
                        }));
                    }
                }
            }
            content_offset += source.text.len() + 1;
        }

        downstream_ops
    }

    /// Process a batch of aggregate ops sharing the same (title_entity, kb_name).
    /// Uses UNWIND queries instead of 1-at-a-time loops.
    ///
    /// Returns (downstream_ops, n_skipped, n_queries).
    async fn process_batch(
        conn: &dyn DbConnection,
        config: &CatalogConfig,
        kb_meta: &KBMetadata,
        kb_name: &str,
        title_entity: &str,
        chunker_cache: &HashMap<ChunkerConfig, Chunker>,
        has_sparse: bool,
        has_dual: bool,
        ops: &[&crate::ops::AggregateOp],
    ) -> Result<(Vec<CatalogOp>, usize, usize), String> {
        let kb_config = config.knowledge_bases.get(kb_name);
        let kb_signals = kb_config
            .map(|c| c.signals)
            .unwrap_or(search::SearchSignals::HYBRID);
        let kb_sparse = kb_signals.sparse() && has_sparse;

        let index_table = format!("{kb_name}_Index");
        let chunk_table = format!("{kb_name}_Index_Chunk");
        let title_field_name = &kb_meta.title.field;
        let mut n_queries: usize = 0;

        // Content fields on the title entity itself
        let title_content_fields: Vec<&String> = kb_meta
            .content
            .iter()
            .filter(|c| c.entity == title_entity)
            .map(|c| &c.field)
            .collect();

        // Initialize batch state
        let mut states: Vec<AggState> = ops.iter().map(|op| AggState {
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
            let return_clause = return_fields.join(", ");

            let cypher = format!(
                "UNWIND $items AS item \
                 MATCH (e:{title_entity} {{_uuid: item.uuid}}) \
                 RETURN {return_clause}"
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
                            states[idx].sources.push(SourceContent {
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

        // ── Step 2: UNWIND read linked content (1 query per content entity type) ──
        let other_content_entities: HashSet<&str> = kb_meta
            .content
            .iter()
            .map(|c| c.entity.as_str())
            .filter(|e| *e != title_entity)
            .collect();

        for content_entity_name in &other_content_entities {
            let relation = Self::find_relation_to_entity(config, title_entity, content_entity_name);
            if let Some((rel_name, is_forward)) = relation {
                let entity_fields: Vec<&String> = kb_meta
                    .content
                    .iter()
                    .filter(|c| c.entity == *content_entity_name)
                    .map(|c| &c.field)
                    .collect();

                if entity_fields.is_empty() {
                    continue;
                }

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
                let fields_clause = fields_return.join(", ");

                let cypher = if is_forward {
                    format!(
                        "UNWIND $items AS item \
                         MATCH (t:{title_entity} {{_uuid: item.uuid}})-[:{rel_name}]->(c:{content_entity_name}) \
                         RETURN {fields_clause}"
                    )
                } else {
                    format!(
                        "UNWIND $items AS item \
                         MATCH (t:{title_entity} {{_uuid: item.uuid}})<-[:{rel_name}]-(c:{content_entity_name}) \
                         RETURN {fields_clause}"
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
                                states[idx].sources.push(SourceContent {
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
        struct ChangedAgg {
            state_idx: usize,
            title_text: String,
            content_text: String,
            new_hash: String,
        }

        let mut changed: Vec<ChangedAgg> = Vec::new();
        let mut skipped: usize = 0;

        for (i, state) in states.iter_mut().enumerate() {
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

            changed.push(ChangedAgg {
                state_idx: i,
                title_text: state.title_text.clone(),
                content_text,
                new_hash,
            });
        }

        if changed.is_empty() {
            return Ok((vec![], skipped, n_queries));
        }

        // ── Step 5: UNWIND UPDATE changed indexes ──
        {
            let items_param = CypherValue::List(
                changed.iter().map(|c| {
                    let mut m = BTreeMap::new();
                    m.insert("uuid".into(), CypherValue::String(
                        states[c.state_idx].index_entry_uuid.clone()
                    ));
                    m.insert("title".into(), CypherValue::String(c.title_text.clone()));
                    m.insert("content".into(), CypherValue::String(c.content_text.clone()));
                    m.insert("hash".into(), CypherValue::String(c.new_hash.clone()));
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
            )
            .await
            .map_err(|e| e.to_string())?;
            n_queries += 1;
        }

        // ── Step 6: UNWIND DELETE old chunks ──
        {
            let items_param = CypherValue::List(
                changed.iter().map(|c| {
                    let mut m = BTreeMap::new();
                    m.insert("uuid".into(), CypherValue::String(
                        states[c.state_idx].index_entry_uuid.clone()
                    ));
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
            n_queries += 1;
        }

        // ── Step 7: Re-chunk changed aggregates ──
        let chunking = &kb_meta.chunking;
        let chunker_key = ChunkerConfig {
            max_size: chunking.max_size,
            overlap: chunking.overlap,
            strategy: chunking.strategy.clone(),
        };
        let chunker = chunker_cache
            .get(&chunker_key)
            .expect("chunker must be pre-warmed");

        let mut all_downstream: Vec<CatalogOp> = Vec::new();

        for c in &changed {
            let idx_uuid = &states[c.state_idx].index_entry_uuid;
            let sources = &states[c.state_idx].sources;
            let chunk_ops = Self::generate_chunk_ops(
                kb_name, kb_signals, kb_sparse, has_dual,
                idx_uuid, &c.title_text, sources, chunker, &chunk_table,
            );
            all_downstream.extend(chunk_ops);
        }

        Ok((all_downstream, skipped, n_queries))
    }
}

#[async_trait]
impl DynamicNode for AggregateBatchNode {
    fn name(&self) -> &str {
        "aggregate_batch"
    }
    fn inputs(&self) -> &[PortDef] {
        &[
            PortDef { name: "ops", port_type: PortType::Aggregates, required: true },
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
    async fn execute_dynamic(
        &mut self,
        ctx: &mut NodeContext,
        emitter: &mut GraphEmitter,
    ) -> Result<(), String> {
        use super::port::BatchPayload;

        let items: Vec<crate::ops::AggregateOp> = match ctx.take_input("ops") {
            Some(PortValue::Batch(payload)) => payload
                .take::<crate::ops::AggregateOp>()
                .ok_or("AggregateBatchNode: failed to extract Vec<AggregateOp>")?,
            _ => return Err("AggregateBatchNode: missing 'ops' input".to_string()),
        };

        let conn = ctx.service::<Arc<dyn DbConnection>>("conn")
            .ok_or("AggregateBatchNode: 'conn' service not registered")?;
        let config = ctx.service::<CatalogConfig>("config")
            .ok_or("AggregateBatchNode: 'config' service not registered")?;
        let kb_metadata = ctx.service::<HashMap<String, KBMetadata>>("kb_metadata")
            .ok_or("AggregateBatchNode: 'kb_metadata' service not registered")?;
        let chunker_cache = ctx.service::<HashMap<ChunkerConfig, Chunker>>("chunker_cache")
            .ok_or("AggregateBatchNode: 'chunker_cache' service not registered")?;
        let has_sparse = ctx.service::<bool>("has_sparse").map(|v| *v).unwrap_or(false);
        let has_dual = ctx.service::<bool>("has_dual").map(|v| *v).unwrap_or(false);

        // Deduplicate by index_entry_uuid
        let mut seen = HashSet::new();
        let mut unique_ops: Vec<&crate::ops::AggregateOp> = Vec::new();
        for agg in &items {
            if seen.insert(agg.index_entry_uuid.clone()) {
                unique_ops.push(agg);
            }
        }

        // Group by (title_entity, kb_name) — same field structure = same UNWIND queries
        let mut groups: HashMap<(&str, &str), Vec<&crate::ops::AggregateOp>> = HashMap::new();
        for agg in &unique_ops {
            groups
                .entry((&agg.title_entity, &agg.kb_name))
                .or_default()
                .push(agg);
        }

        // Process all groups via batched UNWIND queries
        let mut all_inserts: Vec<InsertOp> = Vec::new();
        let mut all_links: Vec<LinkOp> = Vec::new();
        let mut all_embeds: Vec<EmbedOp> = Vec::new();
        let mut all_sparse_embeds: Vec<SparseEmbedOp> = Vec::new();
        let mut all_dual_embeds: Vec<DualEmbedOp> = Vec::new();
        let mut total_skipped: usize = 0;
        let mut total_queries: usize = 0;

        for ((_title_entity, kb_name), group_ops) in &groups {
            let kb_meta = match kb_metadata.get(*kb_name) {
                Some(m) => m,
                None => continue,
            };

            let (downstream, skipped, n_queries) = Self::process_batch(
                &**conn,
                &config,
                kb_meta,
                kb_name,
                _title_entity,
                &chunker_cache,
                has_sparse,
                has_dual,
                group_ops,
            ).await?;

            total_skipped += skipped;
            total_queries += n_queries;

            for op in downstream {
                match op {
                    CatalogOp::Insert(o) => all_inserts.push(o),
                    CatalogOp::Link(o) => all_links.push(o),
                    CatalogOp::Embed(o) => all_embeds.push(o),
                    CatalogOp::SparseEmbed(o) => all_sparse_embeds.push(o),
                    CatalogOp::DualEmbed(o) => all_dual_embeds.push(o),
                    _ => {}
                }
            }
        }

        let group_summary: Vec<String> = groups
            .iter()
            .map(|((te, kb), ops)| format!("{te}@{kb}×{}", ops.len()))
            .collect();
        eprintln!(
            "[AggregateBatchNode] {} ops ({} unique) → {} UNWIND groups: [{}], {} queries, {} skipped (unchanged), \
             downstream: inserts={}, links={}, embeds={}, sparse={}, dual={}",
            items.len(), seen.len(), groups.len(), group_summary.join(", "),
            total_queries, total_skipped,
            all_inserts.len(), all_links.len(),
            all_embeds.len(), all_sparse_embeds.len(), all_dual_embeds.len(),
        );

        // Emit downstream nodes with data via set_initial_input
        if !all_inserts.is_empty() {
            emitter.add_node(Box::new(InsertBatchNode::new("agg_inserts")));
            emitter.set_initial_input("agg_inserts", "ops",
                PortValue::Batch(BatchPayload::new(PortType::Inserts, all_inserts)));
            emitter.connect("aggregate_batch", "done", "agg_inserts", "trigger");
        }

        if !all_links.is_empty() {
            emitter.add_node(Box::new(LinkBatchNode::new("agg_links")));
            emitter.set_initial_input("agg_links", "ops",
                PortValue::Batch(BatchPayload::new(PortType::Links, all_links)));
            if emitter.added_nodes.iter().any(|n| n.name() == "agg_inserts") {
                emitter.connect("agg_inserts", "done", "agg_links", "trigger");
            } else {
                emitter.connect("aggregate_batch", "done", "agg_links", "trigger");
            }
        }

        let embed_trigger = if emitter.added_nodes.iter().any(|n| n.name() == "agg_inserts") {
            "agg_inserts"
        } else {
            "aggregate_batch"
        };

        if !all_dual_embeds.is_empty() {
            emitter.add_node(Box::new(DualEmbedBatchNode::new("agg_dual_embeds", 32)));
            emitter.set_initial_input("agg_dual_embeds", "ops",
                PortValue::Batch(BatchPayload::new(PortType::DualEmbeds, all_dual_embeds)));
            emitter.connect(embed_trigger, "done", "agg_dual_embeds", "trigger");
        }

        if !all_embeds.is_empty() {
            emitter.add_node(Box::new(EmbedBatchNode::new("agg_embeds")));
            emitter.set_initial_input("agg_embeds", "ops",
                PortValue::Batch(BatchPayload::new(PortType::Embeds, all_embeds)));
            emitter.connect(embed_trigger, "done", "agg_embeds", "trigger");
        }

        if !all_sparse_embeds.is_empty() {
            emitter.add_node(Box::new(SparseEmbedBatchNode::new("agg_sparse_embeds")));
            emitter.set_initial_input("agg_sparse_embeds", "ops",
                PortValue::Batch(BatchPayload::new(PortType::SparseEmbeds, all_sparse_embeds)));
            emitter.connect(embed_trigger, "done", "agg_sparse_embeds", "trigger");
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::port::BatchPayload;
    use crate::connection::MockConnection;
    use crate::embedder::MockEmbedder;
    use crate::dataflow::graph::DataflowGraph;
    use crate::dataflow::runtime::DataflowRuntime;
    use crate::dataflow::services::ServiceRegistry;
    use crate::refs::EntityRef;

    /// Build a ServiceRegistry with conn + node_id_cache for insert tests.
    fn insert_services(conn: Arc<MockConnection>) -> ServiceRegistry {
        let mut s = ServiceRegistry::new();
        let conn_dyn: Arc<dyn DbConnection> = conn;
        s.register::<Arc<dyn DbConnection>>("conn", Arc::new(conn_dyn));
        s.register::<RwLock<NodeIdCache>>("node_id_cache", Arc::new(RwLock::new(NodeIdCache::new())));
        s
    }

    #[tokio::test]
    async fn insert_batch_node_resolves_refs() {
        let conn = Arc::new(MockConnection::new());

        let (entity_ref, resolver) = EntityRef::new("Document");
        let mut data = BTreeMap::new();
        data.insert("_uuid".to_string(), CypherValue::String("uuid-1".to_string()));
        data.insert("title".to_string(), CypherValue::String("Test".to_string()));

        let insert = InsertOp::new(
            "Document".to_string(),
            data,
            resolver,
            entity_ref.clone(),
        );

        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(InsertBatchNode::new("test_insert"))).unwrap();
        graph.set_initial_input("test_insert", "ops",
            PortValue::Batch(BatchPayload::new(PortType::Inserts, vec![insert])));

        let runtime = DataflowRuntime::with_services(10, insert_services(conn));
        let result = runtime.execute(&mut graph).await;
        assert!(result.is_ok());
        assert!(entity_ref.is_ready());
        assert_eq!(entity_ref.uuid().unwrap(), "uuid-1");
    }

    #[tokio::test]
    async fn link_batch_node_resolves_refs() {
        let conn = Arc::new(MockConnection::new());

        let (rel_ref, resolver) = RelationRef::new("HAS_SECTION");
        let link = LinkOp::new(
            "HAS_SECTION".to_string(),
            RefOrUuid::from("from-uuid"),
            RefOrUuid::from("to-uuid"),
            BTreeMap::new(),
            resolver,
            rel_ref.clone(),
        );

        let mut services = ServiceRegistry::new();
        let conn_dyn: Arc<dyn DbConnection> = conn;
        services.register::<Arc<dyn DbConnection>>("conn", Arc::new(conn_dyn));

        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(LinkBatchNode::new("test_link"))).unwrap();
        graph.set_initial_input("test_link", "ops",
            PortValue::Batch(BatchPayload::new(PortType::Links, vec![link])));

        let runtime = DataflowRuntime::with_services(10, services);
        let result = runtime.execute(&mut graph).await;
        assert!(result.is_ok());
        let resolved = rel_ref.resolved().unwrap();
        assert_eq!(resolved.from_uuid, "from-uuid");
        assert_eq!(resolved.to_uuid, "to-uuid");
    }

    #[tokio::test]
    async fn embed_batch_node_calls_embedder() {
        let conn = Arc::new(MockConnection::new());
        let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(384));

        let (entity_ref, resolver) = EntityRef::new("Document");
        resolver.resolve("uuid-1".to_string());

        let embed = EmbedOp {
            entity_ref,
            kb_name: "main".to_string(),
            texts: vec!["hello world".to_string()],
        };

        let mut services = ServiceRegistry::new();
        let conn_dyn: Arc<dyn DbConnection> = conn;
        services.register::<Arc<dyn DbConnection>>("conn", Arc::new(conn_dyn));
        services.register::<Arc<dyn Embedder>>("embedder", Arc::new(embedder));
        services.register::<usize>("embedding_dim", Arc::new(384usize));

        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(EmbedBatchNode::new("test_embed"))).unwrap();
        graph.set_initial_input("test_embed", "ops",
            PortValue::Batch(BatchPayload::new(PortType::Embeds, vec![embed])));

        let runtime = DataflowRuntime::with_services(10, services);
        let result = runtime.execute(&mut graph).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn insert_then_link_pipeline() {
        let conn = Arc::new(MockConnection::new());

        let (entity_ref, resolver) = EntityRef::new("Document");
        let mut data = BTreeMap::new();
        data.insert("_uuid".to_string(), CypherValue::String("doc-1".to_string()));

        let insert = InsertOp::new(
            "Document".to_string(),
            data,
            resolver,
            entity_ref.clone(),
        );

        let (rel_ref, rel_resolver) = RelationRef::new("SELF_REF");
        let link = LinkOp::new(
            "SELF_REF".to_string(),
            RefOrUuid::Ref(entity_ref.clone()),
            RefOrUuid::from("other-uuid"),
            BTreeMap::new(),
            rel_resolver,
            rel_ref.clone(),
        );

        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(InsertBatchNode::new("inserts"))).unwrap();
        graph.set_initial_input("inserts", "ops",
            PortValue::Batch(BatchPayload::new(PortType::Inserts, vec![insert])));
        graph.add_node(Box::new(LinkBatchNode::new("links"))).unwrap();
        graph.set_initial_input("links", "ops",
            PortValue::Batch(BatchPayload::new(PortType::Links, vec![link])));
        graph.connect("inserts", "done", "links", "trigger").unwrap();

        let runtime = DataflowRuntime::with_services(10, insert_services(conn));
        let result = runtime.execute(&mut graph).await;
        assert!(result.is_ok());
        assert_eq!(entity_ref.uuid().unwrap(), "doc-1");
        let resolved = rel_ref.resolved().unwrap();
        assert_eq!(resolved.from_uuid, "doc-1");
        assert_eq!(resolved.to_uuid, "other-uuid");
    }

    // ─── SplitOpsNode tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn split_ops_routes_by_type() {
        use super::super::port::BatchPayload;
        use crate::ops::AggregateOp;
        use crate::refs::RelationRef;

        let (entity_ref, resolver) = EntityRef::new("Doc");
        resolver.resolve("uuid-1".to_string());
        let (_, link_resolver) = RelationRef::new("HAS");

        let ops = vec![
            CatalogOp::Insert(InsertOp::new(
                "Doc".to_string(),
                BTreeMap::new(),
                {
                    let (_, r) = EntityRef::new("Doc");
                    r
                },
                entity_ref.clone(),
            )),
            CatalogOp::Link(LinkOp::new(
                "HAS".to_string(),
                RefOrUuid::from("a"),
                RefOrUuid::from("b"),
                BTreeMap::new(),
                link_resolver,
                {
                    let (r, _) = RelationRef::new("HAS");
                    r
                },
            )),
            CatalogOp::Embed(EmbedOp {
                entity_ref: entity_ref.clone(),
                kb_name: "main".to_string(),
                texts: vec!["hello".to_string()],
            }),
        ];

        let payload = BatchPayload::new(PortType::Ops, ops);
        let mut ctx = NodeContext::new();
        ctx.set_input("ops", PortValue::Batch(payload));

        let mut node = SplitOpsNode;
        let result = node.execute(&mut ctx).await;
        assert!(result.is_ok());

        let outputs = ctx.drain_outputs();
        // Should have inserts, links, embeds — no chunks/aggregates/sparse/dual
        assert!(outputs.contains_key("inserts"));
        assert!(outputs.contains_key("links"));
        assert!(outputs.contains_key("embeds"));
        assert!(!outputs.contains_key("chunks"));
        assert!(!outputs.contains_key("aggregates"));
        assert!(!outputs.contains_key("sparse_embeds"));
        assert!(!outputs.contains_key("dual_embeds"));

        // Verify counts via BatchPayload
        if let Some(PortValue::Batch(p)) = outputs.get("inserts") {
            assert_eq!(p.count(), 1);
        } else {
            panic!("inserts should be Batch");
        }
        if let Some(PortValue::Batch(p)) = outputs.get("links") {
            assert_eq!(p.count(), 1);
        } else {
            panic!("links should be Batch");
        }
        if let Some(PortValue::Batch(p)) = outputs.get("embeds") {
            assert_eq!(p.count(), 1);
        } else {
            panic!("embeds should be Batch");
        }
    }

    #[tokio::test]
    async fn split_ops_empty_input() {
        use super::super::port::BatchPayload;

        let ops: Vec<CatalogOp> = vec![];
        let payload = BatchPayload::new(PortType::Ops, ops);
        let mut ctx = NodeContext::new();
        ctx.set_input("ops", PortValue::Batch(payload));

        let mut node = SplitOpsNode;
        let result = node.execute(&mut ctx).await;
        assert!(result.is_ok());

        let outputs = ctx.drain_outputs();
        // All outputs should be absent (empty batches not emitted)
        assert!(outputs.is_empty());
    }

    #[tokio::test]
    async fn split_ops_missing_input_errors() {
        let mut ctx = NodeContext::new();
        let mut node = SplitOpsNode;
        let result = node.execute(&mut ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing or invalid"));
    }
}

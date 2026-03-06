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

// ─── InsertBatchNode ────────────────────────────────────────────────────────

/// Batch INSERT: executes Cypher CREATE for each InsertOp, resolves EntityRefs,
/// caches internal node IDs.
pub struct InsertBatchNode {
    name: String,
    items: Vec<InsertOp>,
    conn: Arc<dyn DbConnection>,
    node_id_cache: Arc<RwLock<NodeIdCache>>,
}

impl InsertBatchNode {
    pub fn new(
        name: String,
        items: Vec<InsertOp>,
        conn: Arc<dyn DbConnection>,
        node_id_cache: Arc<RwLock<NodeIdCache>>,
    ) -> Self {
        Self {
            name,
            items,
            conn,
            node_id_cache,
        }
    }
}

#[async_trait]
impl Node for InsertBatchNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "trigger",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "done",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    async fn execute(&self, ctx: &mut NodeContext) -> Result<(), String> {
        // Safety: we need &mut self.items but Node::execute takes &self.
        // Use interior mutability via unsafe pointer — the runtime guarantees
        // single-threaded execution per node.
        let items = unsafe {
            &mut *(std::ptr::addr_of!(self.items) as *mut Vec<InsertOp>)
        };

        for insert in items.iter_mut() {
            let mut columns: Vec<&str> = insert.data.keys().map(|k| k.as_str()).collect();
            columns.sort();

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

            let uuid = insert
                .data
                .get("_uuid")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Cache internal node ID
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

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── LinkBatchNode ──────────────────────────────────────────────────────────

/// Batch LINK: executes MATCH+CREATE for each LinkOp, resolves from/to refs.
pub struct LinkBatchNode {
    name: String,
    items: Vec<LinkOp>,
    conn: Arc<dyn DbConnection>,
}

impl LinkBatchNode {
    pub fn new(
        name: String,
        items: Vec<LinkOp>,
        conn: Arc<dyn DbConnection>,
    ) -> Self {
        Self { name, items, conn }
    }
}

#[async_trait]
impl Node for LinkBatchNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "trigger",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "done",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    async fn execute(&self, ctx: &mut NodeContext) -> Result<(), String> {
        let items = unsafe {
            &mut *(std::ptr::addr_of!(self.items) as *mut Vec<LinkOp>)
        };

        for link in items.iter_mut() {
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

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── EmbedBatchNode ─────────────────────────────────────────────────────────

/// Batch dense embedding: resolves refs, calls Embedder::embed(), UNWIND SET.
pub struct EmbedBatchNode {
    name: String,
    items: Vec<EmbedOp>,
    conn: Arc<dyn DbConnection>,
    embedder: Arc<dyn Embedder>,
    embedding_dim: usize,
}

impl EmbedBatchNode {
    pub fn new(
        name: String,
        items: Vec<EmbedOp>,
        conn: Arc<dyn DbConnection>,
        embedder: Arc<dyn Embedder>,
        embedding_dim: usize,
    ) -> Self {
        Self {
            name,
            items,
            conn,
            embedder,
            embedding_dim,
        }
    }
}

#[async_trait]
impl Node for EmbedBatchNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "trigger",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "done",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    async fn execute(&self, ctx: &mut NodeContext) -> Result<(), String> {
        struct EmbedWork {
            uuid: String,
            text: String,
            entity_name: String,
            embedding_col: String,
        }

        let items = unsafe {
            &mut *(std::ptr::addr_of!(self.items) as *mut Vec<EmbedOp>)
        };

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

            // Group by (entity_name, embedding_col)
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
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── SparseEmbedBatchNode ───────────────────────────────────────────────────

/// Batch sparse embedding: resolves refs, calls SparseEmbedder, UNWIND SET.
pub struct SparseEmbedBatchNode {
    name: String,
    items: Vec<SparseEmbedOp>,
    conn: Arc<dyn DbConnection>,
    sparse_embedder: Arc<dyn SparseEmbedder>,
}

impl SparseEmbedBatchNode {
    pub fn new(
        name: String,
        items: Vec<SparseEmbedOp>,
        conn: Arc<dyn DbConnection>,
        sparse_embedder: Arc<dyn SparseEmbedder>,
    ) -> Self {
        Self {
            name,
            items,
            conn,
            sparse_embedder,
        }
    }
}

#[async_trait]
impl Node for SparseEmbedBatchNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn inputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "trigger",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "done",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    async fn execute(&self, ctx: &mut NodeContext) -> Result<(), String> {
        struct SparseWork {
            uuid: String,
            text: String,
            entity_name: String,
            kb_name: String,
        }

        let items = unsafe {
            &mut *(std::ptr::addr_of!(self.items) as *mut Vec<SparseEmbedOp>)
        };

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

            let mut groups: HashMap<(&str, &str), Vec<(&SparseWork, &SparseVector)>> =
                HashMap::new();
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
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── DualEmbedBatchNode ─────────────────────────────────────────────────────

/// Dual dense+sparse embedding in GPU mini-batches, then batch UNWIND.
pub struct DualEmbedBatchNode {
    name: String,
    items: Vec<DualEmbedOp>,
    conn: Arc<dyn DbConnection>,
    embedder: Arc<dyn DualEmbedder>,
    embedding_dim: usize,
    gpu_batch_size: usize,
}

impl DualEmbedBatchNode {
    pub fn new(
        name: String,
        items: Vec<DualEmbedOp>,
        conn: Arc<dyn DbConnection>,
        embedder: Arc<dyn DualEmbedder>,
        embedding_dim: usize,
        gpu_batch_size: usize,
    ) -> Self {
        Self {
            name,
            items,
            conn,
            embedder,
            embedding_dim,
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
        &[PortDef {
            name: "trigger",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "done",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    async fn execute(&self, ctx: &mut NodeContext) -> Result<(), String> {
        struct DualWork {
            uuid: String,
            text: String,
            entity_name: String,
            kb_name: String,
        }

        let items = unsafe {
            &mut *(std::ptr::addr_of!(self.items) as *mut Vec<DualEmbedOp>)
        };

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

        // GPU embedding in mini-batches
        let mut dense_results: Vec<(&DualWork, Vec<f32>)> = Vec::with_capacity(works.len());
        let mut sparse_results: Vec<(&DualWork, SparseVector)> = Vec::with_capacity(works.len());

        for chunk in works.chunks(self.gpu_batch_size) {
            let texts: Vec<String> = chunk.iter().map(|w| w.text.clone()).collect();
            let (dense_vecs, sparse_vecs) = self
                .embedder
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
                if vec.len() != self.embedding_dim {
                    return Err(format!(
                        "embedding dimension mismatch: expected {}, got {}",
                        self.embedding_dim,
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
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── ChunkBatchNode ─────────────────────────────────────────────────────────

/// DynamicNode: processes ChunkOps via rayon parallel chunking, then emits
/// InsertBatchNode + LinkBatchNode + EmbedBatchNode downstream.
pub struct ChunkBatchNode {
    config: CatalogConfig,
    kb_metadata: HashMap<String, KBMetadata>,
    chunker_cache: HashMap<ChunkerConfig, Chunker>,
    has_sparse: bool,
    has_dual: bool,
    items: Vec<crate::ops::ChunkOp>,
    // Services needed by emitted nodes
    conn: Arc<dyn DbConnection>,
    node_id_cache: Arc<RwLock<NodeIdCache>>,
    embedder: Arc<dyn Embedder>,
    sparse_embedder: Option<Arc<dyn SparseEmbedder>>,
    dual_embedder: Option<Arc<dyn DualEmbedder>>,
    embedding_dim: usize,
}

impl ChunkBatchNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: CatalogConfig,
        kb_metadata: HashMap<String, KBMetadata>,
        chunker_cache: HashMap<ChunkerConfig, Chunker>,
        has_sparse: bool,
        has_dual: bool,
        items: Vec<crate::ops::ChunkOp>,
        conn: Arc<dyn DbConnection>,
        node_id_cache: Arc<RwLock<NodeIdCache>>,
        embedder: Arc<dyn Embedder>,
        sparse_embedder: Option<Arc<dyn SparseEmbedder>>,
        dual_embedder: Option<Arc<dyn DualEmbedder>>,
        embedding_dim: usize,
    ) -> Self {
        Self {
            config,
            kb_metadata,
            chunker_cache,
            has_sparse,
            has_dual,
            items,
            conn,
            node_id_cache,
            embedder,
            sparse_embedder,
            dual_embedder,
            embedding_dim,
        }
    }
}

#[async_trait]
impl DynamicNode for ChunkBatchNode {
    fn name(&self) -> &str {
        "chunk_batch"
    }
    fn inputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "trigger",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "done",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    async fn execute_dynamic(
        &self,
        ctx: &mut NodeContext,
        emitter: &mut GraphEmitter,
    ) -> Result<(), String> {
        use rayon::prelude::*;

        // Parallel chunking via rayon
        let all_downstream: Vec<Vec<CatalogOp>> = self
            .items
            .par_iter()
            .map(|chunk_op| {
                crate::catalog::compute_chunk_ops(
                    &chunk_op.entity_name,
                    &chunk_op.parent_uuid,
                    &chunk_op.entity_ref,
                    &chunk_op.data,
                    &self.config,
                    &self.kb_metadata,
                    &self.chunker_cache,
                    self.has_sparse,
                    self.has_dual,
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

        // Emit downstream nodes
        if !inserts.is_empty() {
            emitter.add_node(Box::new(InsertBatchNode::new(
                "chunk_inserts".to_string(),
                inserts,
                self.conn.clone(),
                self.node_id_cache.clone(),
            )));
            emitter.connect("chunk_batch", "done", "chunk_inserts", "trigger");
        }

        if !links.is_empty() {
            emitter.add_node(Box::new(LinkBatchNode::new(
                "chunk_links".to_string(),
                links,
                self.conn.clone(),
            )));
            // Links depend on inserts (need resolved UUIDs)
            if emitter.added_nodes.iter().any(|n| n.name() == "chunk_inserts") {
                emitter.connect("chunk_inserts", "done", "chunk_links", "trigger");
            } else {
                emitter.connect("chunk_batch", "done", "chunk_links", "trigger");
            }
        }

        // Embed nodes depend on inserts (need entities in DB)
        let embed_trigger = if emitter.added_nodes.iter().any(|n| n.name() == "chunk_inserts") {
            "chunk_inserts"
        } else {
            "chunk_batch"
        };

        if !dual_embeds.is_empty() {
            if let Some(ref dual_emb) = self.dual_embedder {
                emitter.add_node(Box::new(DualEmbedBatchNode::new(
                    "chunk_dual_embeds".to_string(),
                    dual_embeds,
                    self.conn.clone(),
                    dual_emb.clone(),
                    self.embedding_dim,
                    32,
                )));
                emitter.connect(embed_trigger, "done", "chunk_dual_embeds", "trigger");
            }
        }

        if !embeds.is_empty() {
            emitter.add_node(Box::new(EmbedBatchNode::new(
                "chunk_embeds".to_string(),
                embeds,
                self.conn.clone(),
                self.embedder.clone(),
                self.embedding_dim,
            )));
            emitter.connect(embed_trigger, "done", "chunk_embeds", "trigger");
        }

        if !sparse_embeds.is_empty() {
            if let Some(ref sparse_emb) = self.sparse_embedder {
                emitter.add_node(Box::new(SparseEmbedBatchNode::new(
                    "chunk_sparse_embeds".to_string(),
                    sparse_embeds,
                    self.conn.clone(),
                    sparse_emb.clone(),
                )));
                emitter.connect(embed_trigger, "done", "chunk_sparse_embeds", "trigger");
            }
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

/// DynamicNode: rebuilds `_content` on `{KB}_Index`, deletes stale chunks,
/// re-chunks per source field, and emits InsertBatchNode + LinkBatchNode +
/// EmbedBatchNode downstream.
pub struct AggregateBatchNode {
    items: Vec<crate::ops::AggregateOp>,
    conn: Arc<dyn DbConnection>,
    config: CatalogConfig,
    kb_metadata: HashMap<String, KBMetadata>,
    chunker_cache: HashMap<ChunkerConfig, Chunker>,
    has_sparse: bool,
    has_dual: bool,
    // Services for emitted nodes
    node_id_cache: Arc<RwLock<NodeIdCache>>,
    embedder: Arc<dyn Embedder>,
    sparse_embedder: Option<Arc<dyn SparseEmbedder>>,
    dual_embedder: Option<Arc<dyn DualEmbedder>>,
    embedding_dim: usize,
}

impl AggregateBatchNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        items: Vec<crate::ops::AggregateOp>,
        conn: Arc<dyn DbConnection>,
        config: CatalogConfig,
        kb_metadata: HashMap<String, KBMetadata>,
        chunker_cache: HashMap<ChunkerConfig, Chunker>,
        has_sparse: bool,
        has_dual: bool,
        node_id_cache: Arc<RwLock<NodeIdCache>>,
        embedder: Arc<dyn Embedder>,
        sparse_embedder: Option<Arc<dyn SparseEmbedder>>,
        dual_embedder: Option<Arc<dyn DualEmbedder>>,
        embedding_dim: usize,
    ) -> Self {
        Self {
            items,
            conn,
            config,
            kb_metadata,
            chunker_cache,
            has_sparse,
            has_dual,
            node_id_cache,
            embedder,
            sparse_embedder,
            dual_embedder,
            embedding_dim,
        }
    }

    fn find_relation_to_entity(
        &self,
        title_entity: &str,
        content_entity: &str,
    ) -> Option<(String, bool)> {
        for (rel_name, rel_def) in &self.config.relations {
            if rel_def.from == title_entity && rel_def.to == content_entity {
                return Some((rel_name.clone(), true));
            }
            if rel_def.from == content_entity && rel_def.to == title_entity {
                return Some((rel_name.clone(), false));
            }
        }
        None
    }

    async fn process_one(
        &self,
        agg: &crate::ops::AggregateOp,
    ) -> Result<Vec<CatalogOp>, String> {
        let kb_name = &agg.kb_name;
        let kb_meta = match self.kb_metadata.get(kb_name) {
            Some(m) => m,
            None => return Ok(vec![]),
        };
        let kb_config = self.config.knowledge_bases.get(kb_name);
        let kb_signals = kb_config
            .map(|c| c.signals)
            .unwrap_or(search::SearchSignals::HYBRID);
        let kb_sparse = kb_signals.sparse() && self.has_sparse;

        let index_table = format!("{kb_name}_Index");
        let chunk_table = format!("{kb_name}_Index_Chunk");
        let title_entity = &agg.title_entity;
        let source_uuid = &agg.source_uuid;

        // 1. Get title entity's field values
        let title_field_name = &kb_meta.title.field;
        let content_field_names: Vec<&String> = kb_meta
            .content
            .iter()
            .filter(|c| c.entity == *title_entity)
            .map(|c| &c.field)
            .collect();

        let mut return_fields = vec![format!("e.{title_field_name} AS _title_val")];
        for f in &content_field_names {
            return_fields.push(format!("e.{f} AS {f}"));
        }
        let return_clause = return_fields.join(", ");
        let title_query = format!(
            "MATCH (e:{title_entity} {{_uuid: $uuid}}) RETURN {return_clause}"
        );
        let title_result = self
            .conn
            .execute_with_params(
                &title_query,
                &[QueryParam::new("uuid", source_uuid.clone())],
            )
            .await
            .map_err(|e| e.to_string())?;

        if title_result.is_empty() {
            return Ok(vec![]);
        }

        let title_max_chars = kb_meta.chunking.title_max_chars;
        let raw_title = title_result.rows[0]
            .first()
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let title_text: String = if title_max_chars > 0 && raw_title.len() > title_max_chars {
            raw_title.chars().take(title_max_chars).collect()
        } else {
            raw_title.to_string()
        };

        let mut sources: Vec<SourceContent> = Vec::new();
        for (i, f) in content_field_names.iter().enumerate() {
            let text = title_result.rows[0]
                .get(i + 1)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !text.is_empty() {
                sources.push(SourceContent {
                    entity_name: title_entity.clone(),
                    entity_uuid: source_uuid.clone(),
                    field_name: f.to_string(),
                    text: text.to_string(),
                });
            }
        }

        // 2. Collect content from linked entities
        let other_content_entities: HashSet<&str> = kb_meta
            .content
            .iter()
            .map(|c| c.entity.as_str())
            .filter(|e| *e != title_entity.as_str())
            .collect();

        for content_entity_name in &other_content_entities {
            let relation = self.find_relation_to_entity(title_entity, content_entity_name);
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

                let mut fields_return = vec!["c._uuid AS _uuid".to_string()];
                for f in &entity_fields {
                    fields_return.push(format!("c.{f} AS {f}"));
                }
                let fields_clause = fields_return.join(", ");

                let query = if is_forward {
                    format!(
                        "MATCH (t:{title_entity} {{_uuid: $uuid}})-[:{rel_name}]->(c:{content_entity_name}) \
                         RETURN {fields_clause}"
                    )
                } else {
                    format!(
                        "MATCH (t:{title_entity} {{_uuid: $uuid}})<-[:{rel_name}]-(c:{content_entity_name}) \
                         RETURN {fields_clause}"
                    )
                };

                let result = self
                    .conn
                    .execute_with_params(
                        &query,
                        &[QueryParam::new("uuid", source_uuid.clone())],
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                for row in &result.rows {
                    let entity_uuid = row
                        .first()
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    for (i, f) in entity_fields.iter().enumerate() {
                        let text = row
                            .get(i + 1)
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if !text.is_empty() {
                            sources.push(SourceContent {
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

        // 3. Sort sources for deterministic output
        sources.sort_by(|a, b| {
            a.entity_name
                .cmp(&b.entity_name)
                .then(a.entity_uuid.cmp(&b.entity_uuid))
                .then(a.field_name.cmp(&b.field_name))
        });

        // 4. Rebuild _content and compute hash
        let content_text = sources
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let new_hash = content_hash(&format!("{title_text}\n{content_text}"));

        // 5. Compare with stored hash
        let idx_query = format!(
            "MATCH (idx:{index_table} {{_uuid: $uuid}}) RETURN idx._content_hash"
        );
        let idx_result = self
            .conn
            .execute_with_params(
                &idx_query,
                &[QueryParam::new("uuid", agg.index_entry_uuid.clone())],
            )
            .await
            .map_err(|e| e.to_string())?;

        let current_hash = idx_result
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if current_hash == new_hash {
            return Ok(vec![]);
        }

        // 6. UPDATE {KB}_Index
        let update_query = format!(
            "MATCH (idx:{index_table} {{_uuid: $uuid}}) \
             SET idx._title = $title, idx._content = $content, idx._content_hash = $hash"
        );
        self.conn
            .execute_with_params(
                &update_query,
                &[
                    QueryParam::new("uuid", agg.index_entry_uuid.clone()),
                    QueryParam::new("title", title_text.clone()),
                    QueryParam::new("content", content_text),
                    QueryParam::new("hash", new_hash),
                ],
            )
            .await
            .map_err(|e| e.to_string())?;

        // 7. Delete old chunks
        let del_chunks = format!(
            "MATCH (c:{chunk_table} {{_parent_uuid: $uuid}}) DETACH DELETE c"
        );
        let _ = self
            .conn
            .execute_with_params(
                &del_chunks,
                &[QueryParam::new("uuid", agg.index_entry_uuid.clone())],
            )
            .await;

        // 8. Re-chunk per source field, collect downstream ops
        let chunking = &kb_meta.chunking;
        let chunker_key = ChunkerConfig {
            max_size: chunking.max_size,
            overlap: chunking.overlap,
            strategy: chunking.strategy.clone(),
        };
        let chunker = self
            .chunker_cache
            .get(&chunker_key)
            .expect("chunker must be pre-warmed");

        let mut downstream_ops: Vec<CatalogOp> = Vec::new();
        let mut content_offset: usize = 0;

        for source in &sources {
            let chunks = chunker.chunk(&source.text);
            if chunks.is_empty() {
                continue;
            }

            for chunk in &chunks {
                let source_key = format!("{}:{}", source.entity_uuid, source.field_name);
                let c_uuid = chunk_uuid(&agg.index_entry_uuid, &source_key, chunk.index);

                let embed_text = if !title_text.is_empty() {
                    format!("{title_text}\n---\n{}", chunk.text)
                } else {
                    chunk.text.clone()
                };

                let mut chunk_data = BTreeMap::new();
                chunk_data
                    .insert("_uuid".to_string(), CypherValue::String(c_uuid.clone()));
                chunk_data.insert(
                    "_parent_uuid".to_string(),
                    CypherValue::String(agg.index_entry_uuid.clone()),
                );
                chunk_data.insert(
                    "_parent_field".to_string(),
                    CypherValue::String(source.field_name.clone()),
                );
                chunk_data.insert(
                    "_kb_name".to_string(),
                    CypherValue::String(kb_name.clone()),
                );
                chunk_data.insert(
                    "_source_field".to_string(),
                    CypherValue::String(source.field_name.clone()),
                );
                chunk_data.insert(
                    "_source_entity".to_string(),
                    CypherValue::String(source.entity_name.clone()),
                );
                chunk_data.insert(
                    "_source_uuid".to_string(),
                    CypherValue::String(source.entity_uuid.clone()),
                );
                chunk_data.insert(
                    "_text".to_string(),
                    CypherValue::String(chunk.text.clone()),
                );
                chunk_data.insert(
                    "_text_hash".to_string(),
                    CypherValue::String(content_hash(&chunk.text)),
                );
                chunk_data.insert(
                    "_index".to_string(),
                    CypherValue::Int(chunk.index as i64),
                );
                chunk_data.insert(
                    "_start_char".to_string(),
                    CypherValue::Int(chunk.start_byte as i64),
                );
                chunk_data.insert(
                    "_end_char".to_string(),
                    CypherValue::Int(chunk.end_byte as i64),
                );
                chunk_data.insert(
                    "_start_line".to_string(),
                    CypherValue::Int(chunk.start_line as i64),
                );
                chunk_data.insert(
                    "_end_line".to_string(),
                    CypherValue::Int(chunk.end_line as i64),
                );
                chunk_data.insert(
                    "_core_start_char".to_string(),
                    CypherValue::Int(chunk.core_start_byte as i64),
                );
                chunk_data.insert(
                    "_core_end_char".to_string(),
                    CypherValue::Int(chunk.core_end_byte as i64),
                );
                chunk_data.insert(
                    "_core_start_line".to_string(),
                    CypherValue::Int(chunk.core_start_line as i64),
                );
                chunk_data.insert(
                    "_core_end_line".to_string(),
                    CypherValue::Int(chunk.core_end_line as i64),
                );
                chunk_data.insert(
                    "_content_offset".to_string(),
                    CypherValue::Int(content_offset as i64),
                );

                // InsertOp for chunk (prio 2.6)
                let (chunk_ref, chunk_resolver) = EntityRef::new(&chunk_table);
                chunk_resolver.resolve(c_uuid.clone());
                downstream_ops.push(CatalogOp::Insert(
                    InsertOp::new(chunk_table.clone(), chunk_data, {
                        let (_discard_ref, resolver) = EntityRef::new(&chunk_table);
                        resolver
                    }, chunk_ref.clone())
                    .with_priority(PRIO_POST_AGG_INSERT),
                ));

                // LinkOp: {KB}_Index_HAS_CHUNK (prio 2.7)
                let has_chunk_rel = format!("{kb_name}_Index_HAS_CHUNK");
                let (_link_ref, link_resolver) = RelationRef::new(&has_chunk_rel);
                downstream_ops.push(CatalogOp::Link(
                    LinkOp::new(
                        has_chunk_rel,
                        RefOrUuid::Uuid(agg.index_entry_uuid.clone()),
                        RefOrUuid::Uuid(c_uuid.clone()),
                        BTreeMap::new(),
                        link_resolver,
                        _link_ref,
                    )
                    .with_priority(PRIO_POST_AGG_LINK),
                ));

                // LinkOp: {Entity}_SOURCED_{KB} (prio 2.7)
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

                // EmbedOp / DualEmbedOp / SparseEmbedOp
                if self.has_dual && kb_signals.vector() && kb_sparse {
                    downstream_ops.push(CatalogOp::DualEmbed(DualEmbedOp {
                        entity_ref: chunk_ref,
                        kb_name: kb_name.clone(),
                        texts: vec![embed_text],
                    }));
                } else {
                    if kb_signals.vector() {
                        downstream_ops.push(CatalogOp::Embed(EmbedOp {
                            entity_ref: chunk_ref.clone(),
                            kb_name: kb_name.clone(),
                            texts: vec![embed_text.clone()],
                        }));
                    }
                    if kb_sparse {
                        downstream_ops.push(CatalogOp::SparseEmbed(SparseEmbedOp {
                            entity_ref: chunk_ref,
                            kb_name: kb_name.clone(),
                            texts: vec![embed_text],
                        }));
                    }
                }
            }
            content_offset += source.text.len() + 1;
        }

        Ok(downstream_ops)
    }
}

#[async_trait]
impl DynamicNode for AggregateBatchNode {
    fn name(&self) -> &str {
        "aggregate_batch"
    }
    fn inputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "trigger",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "done",
            port_type: PortType::Empty,
            required: false,
        }]
    }
    async fn execute_dynamic(
        &self,
        ctx: &mut NodeContext,
        emitter: &mut GraphEmitter,
    ) -> Result<(), String> {
        // Deduplicate by index_entry_uuid
        let mut seen = HashSet::new();
        let mut unique_ops: Vec<&crate::ops::AggregateOp> = Vec::new();
        for agg in &self.items {
            if seen.insert(agg.index_entry_uuid.clone()) {
                unique_ops.push(agg);
            }
        }

        // Process all aggregate ops and collect downstream ops
        let mut all_inserts: Vec<InsertOp> = Vec::new();
        let mut all_links: Vec<LinkOp> = Vec::new();
        let mut all_embeds: Vec<EmbedOp> = Vec::new();
        let mut all_sparse_embeds: Vec<SparseEmbedOp> = Vec::new();
        let mut all_dual_embeds: Vec<DualEmbedOp> = Vec::new();

        for agg in unique_ops {
            let downstream = self.process_one(agg).await?;
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

        // Emit downstream nodes
        if !all_inserts.is_empty() {
            emitter.add_node(Box::new(InsertBatchNode::new(
                "agg_inserts".to_string(),
                all_inserts,
                self.conn.clone(),
                self.node_id_cache.clone(),
            )));
            emitter.connect("aggregate_batch", "done", "agg_inserts", "trigger");
        }

        if !all_links.is_empty() {
            emitter.add_node(Box::new(LinkBatchNode::new(
                "agg_links".to_string(),
                all_links,
                self.conn.clone(),
            )));
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
            if let Some(ref dual_emb) = self.dual_embedder {
                emitter.add_node(Box::new(DualEmbedBatchNode::new(
                    "agg_dual_embeds".to_string(),
                    all_dual_embeds,
                    self.conn.clone(),
                    dual_emb.clone(),
                    self.embedding_dim,
                    32,
                )));
                emitter.connect(embed_trigger, "done", "agg_dual_embeds", "trigger");
            }
        }

        if !all_embeds.is_empty() {
            emitter.add_node(Box::new(EmbedBatchNode::new(
                "agg_embeds".to_string(),
                all_embeds,
                self.conn.clone(),
                self.embedder.clone(),
                self.embedding_dim,
            )));
            emitter.connect(embed_trigger, "done", "agg_embeds", "trigger");
        }

        if !all_sparse_embeds.is_empty() {
            if let Some(ref sparse_emb) = self.sparse_embedder {
                emitter.add_node(Box::new(SparseEmbedBatchNode::new(
                    "agg_sparse_embeds".to_string(),
                    all_sparse_embeds,
                    self.conn.clone(),
                    sparse_emb.clone(),
                )));
                emitter.connect(embed_trigger, "done", "agg_sparse_embeds", "trigger");
            }
        }

        ctx.set_output("done", PortValue::Empty);
        Ok(())
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::MockConnection;
    use crate::embedder::MockEmbedder;
    use crate::dataflow::graph::DataflowGraph;
    use crate::dataflow::runtime::DataflowRuntime;
    use crate::refs::EntityRef;

    #[tokio::test]
    async fn insert_batch_node_resolves_refs() {
        let conn = Arc::new(MockConnection::new());
        let cache = Arc::new(RwLock::new(NodeIdCache::new()));

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

        let node = InsertBatchNode::new(
            "test_insert".to_string(),
            vec![insert],
            conn,
            cache,
        );

        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(node)).unwrap();

        let runtime = DataflowRuntime::new(10);
        let result = runtime.execute(&mut graph).await;
        // MockConnection always succeeds with empty result
        assert!(result.is_ok());
        // The entity ref should be resolved
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

        let node = LinkBatchNode::new(
            "test_link".to_string(),
            vec![link],
            conn,
        );

        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(node)).unwrap();

        let runtime = DataflowRuntime::new(10);
        let result = runtime.execute(&mut graph).await;
        assert!(result.is_ok());
        // RelationRef should be resolved
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

        let node = EmbedBatchNode::new(
            "test_embed".to_string(),
            vec![embed],
            conn,
            embedder,
            384,
        );

        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(node)).unwrap();

        let runtime = DataflowRuntime::new(10);
        let result = runtime.execute(&mut graph).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn insert_then_link_pipeline() {
        let conn = Arc::new(MockConnection::new());
        let cache = Arc::new(RwLock::new(NodeIdCache::new()));

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
        graph
            .add_node(Box::new(InsertBatchNode::new(
                "inserts".to_string(),
                vec![insert],
                conn.clone(),
                cache,
            )))
            .unwrap();
        graph
            .add_node(Box::new(LinkBatchNode::new(
                "links".to_string(),
                vec![link],
                conn,
            )))
            .unwrap();
        graph.connect("inserts", "done", "links", "trigger").unwrap();

        let runtime = DataflowRuntime::new(10);
        let result = runtime.execute(&mut graph).await;
        assert!(result.is_ok());
        // Insert resolved the ref, link used it
        assert_eq!(entity_ref.uuid().unwrap(), "doc-1");
        let resolved = rel_ref.resolved().unwrap();
        assert_eq!(resolved.from_uuid, "doc-1");
        assert_eq!(resolved.to_uuid, "other-uuid");
    }
}

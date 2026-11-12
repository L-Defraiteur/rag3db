//! Pure preview over supplied facts, never a storage-qualified execution plan.
//!
//! No connection, model, writer or publication capability enters this module.
//! An asserted inventory origin is not proof read from storage. In particular,
//! the current resolved plan does not produce a durable source instance ID.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Serialize, Serializer};

use crate::{
    chunker::{Chunk, Chunker, ChunkerConfig},
    config::{DerivedIdentity, EntityConfig},
    connection::CypherValue,
    dataflow::derive_nodes::{derived_content_uuid, render_context, render_fields, render_inputs_hash},
    embedding_storage::{EmbeddingModelEntry, StorageKind, VectorStorage},
    hash::content_hash,
    scope::Scope,
    uuid::chunk_uuid,
};
use super::plan::{ResolvedMigrationPlan, StorageDescriptor};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EntityKey {
    pub entity: String,
    pub scope: Scope,
    pub uuid: String,
}
impl EntityKey {
    fn key(&self) -> (String, String, String, String) {
        (self.entity.clone(), self.scope.org.clone(), self.scope.project.clone(), self.uuid.clone())
    }
}

/// Full occurrence values (including provenance/positions) and visibility facts
/// are retained. Other graph entities may be included as relation endpoints.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InventoryEntity {
    pub key: EntityKey,
    pub values: BTreeMap<String, CypherValue>,
    pub masks: BTreeSet<String>,
}

/// Ordered endpoints preserve direction; repeated entries preserve multiplicity.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InventoryRelation {
    pub name: String,
    pub from: EntityKey,
    pub to: EntityKey,
    pub properties: BTreeMap<String, CypherValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChunkShape {
    pub field: String,
    pub title: String,
    pub content_offset: usize,
    #[serde(serialize_with = "serialize_chunk")]
    pub chunk: Chunk,
}
fn serialize_chunk<S: Serializer>(c: &Chunk, s: S) -> Result<S::Ok, S::Error> {
    (&c.text, c.index, c.start_byte, c.end_byte, c.start_line, c.end_line,
        c.core_start_byte, c.core_end_byte, c.core_start_line, c.core_end_line).serialize(s)
}
fn serialize_bits<S: Serializer>(values: &[f32], s: S) -> Result<S::Ok, S::Error> {
    values.iter().map(|f| f.to_bits()).collect::<Vec<_>>().serialize(s)
}

#[derive(Debug, Clone, Serialize)]
pub struct KnownDenseValue {
    pub model: String,
    /// Only the default `_text` input is supported in this first preview.
    /// None means its historical provenance has not been established.
    pub input: Option<String>,
    pub marker: String,
    #[serde(serialize_with = "serialize_bits")]
    pub values: Vec<f32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct InventoryChunk {
    pub parent: EntityKey,
    pub uuid: String,
    pub shape: ChunkShape,
    pub dense: Vec<KnownDenseValue>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SparseEvidence {
    pub materialized: bool,
    pub debt: bool,
    pub inconsistent: bool,
    /// A highlighting reader alone is not sparse data to transfer.
    pub reader_declared: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferInventory {
    pub declared_source_identity: Option<String>,
    pub descriptor: StorageDescriptor,
    pub source_entity: String,
    pub source_config: EntityConfig,
    pub models: Vec<EmbeddingModelEntry>,
    pub sparse: SparseEvidence,
    pub dense_debt: bool,
    pub entities: Vec<InventoryEntity>,
    pub relations: Vec<InventoryRelation>,
    pub chunks: Vec<InventoryChunk>,
}
#[derive(Debug, Clone, Serialize)]
pub struct TransferRecipe {
    pub target_descriptor: StorageDescriptor,
    pub target_entity: String,
    pub target_config: EntityConfig,
    pub models: Vec<EmbeddingModelEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RefusalCode {
    ExpectedIdentityMissing, SourceIdentityMissing, ForeignSourceIdentity,
    GenerationUnavailable, DescriptorMismatch, UnsupportedRecipe, InvalidInventory,
    SparseUnsupported, DenseDebt, ModelMismatch, ChunkCoverage, InputProvenanceUnavailable,
    DenseCoverage, InvalidVector, StaleMarker, ConflictingVectors,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransferRefusal {
    pub code: RefusalCode,
    pub location: String,
    pub detail: String,
}
fn refusal(code: RefusalCode, location: impl Into<String>, detail: impl Into<String>) -> TransferRefusal {
    TransferRefusal { code, location: location.into(), detail: detail.into() }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PreviewVersion {
    pub key: EntityKey,
    pub fields: BTreeMap<String, String>,
    /// Occurrence-local render state, not a property of the shared version.
    pub occurrences: BTreeMap<String, String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DenseBinding {
    pub model: String,
    pub source_storage: VectorStorageNames,
    pub target_storage: VectorStorageNames,
    pub marker: String,
    pub value_bits: Vec<u32>,
    /// All covering source UUIDs, not an arbitrary first vector.
    pub sources: BTreeSet<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VectorStorageNames {
    pub column: String,
    pub marker: String,
    pub index: String,
    pub dim: usize,
}
fn storage(table: &str, entry: &EmbeddingModelEntry) -> VectorStorageNames {
    let v = VectorStorage::resolve(table, entry);
    VectorStorageNames { column: v.column, marker: v.marker, index: v.index, dim: v.dim }
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PreviewChunk {
    pub parent: EntityKey,
    pub uuid: String,
    pub shape: ChunkShape,
    pub dense: BTreeMap<String, DenseBinding>,
}

/// Report only. Private fields, no Deserialize, execute, apply or writer token.
#[derive(Debug, Clone, Serialize)]
pub struct KnownTransferPreview {
    identity: String,
    fixed_plan_identity: String,
    inventory_fingerprint: String,
    retained_entities: Vec<InventoryEntity>,
    retained_relations: Vec<InventoryRelation>,
    versions: Vec<PreviewVersion>,
    chunks: Vec<PreviewChunk>,
}
impl KnownTransferPreview {
    pub fn identity(&self) -> &str { &self.identity }
    pub fn fixed_plan_identity(&self) -> &str { &self.fixed_plan_identity }
    pub fn inventory_fingerprint(&self) -> &str { &self.inventory_fingerprint }
    pub fn retained_entities(&self) -> &[InventoryEntity] { &self.retained_entities }
    pub fn retained_relations(&self) -> &[InventoryRelation] { &self.retained_relations }
    pub fn versions(&self) -> &[PreviewVersion] { &self.versions }
    pub fn chunks(&self) -> &[PreviewChunk] { &self.chunks }
}

// Canonicalize object keys as well as collection ordering. This is a report
// fingerprint, never a storage generation or a durable execution identity.
fn encoded<T: Serialize>(value: &T) -> Result<String, TransferRefusal> {
    fn canonical(v: serde_json::Value) -> serde_json::Value {
        match v {
            serde_json::Value::Object(m) => serde_json::Value::Object(
                m.into_iter().collect::<BTreeMap<_, _>>().into_iter().map(|(k,v)| (k,canonical(v))).collect()),
            serde_json::Value::Array(a) => serde_json::Value::Array(a.into_iter().map(canonical).collect()),
            v => v,
        }
    }
    serde_json::to_value(value).map(canonical).and_then(|v| serde_json::to_string(&v))
        .map_err(|e| refusal(RefusalCode::InvalidInventory, "serialization", e.to_string()))
}
fn valid_values(values: &BTreeMap<String, CypherValue>) -> bool {
    fn valid(v: &CypherValue) -> bool {
        match v {
            CypherValue::Float(f) => f.is_finite(),
            CypherValue::List(v) => v.iter().all(valid),
            CypherValue::Map(v) => v.values().all(valid),
            CypherValue::Blob(_) | CypherValue::Typed { .. } => false,
            _ => true,
        }
    }
    values.values().all(valid)
}

fn models(entries: &[EmbeddingModelEntry], location: &str) -> Result<BTreeMap<String, EmbeddingModelEntry>, TransferRefusal> {
    let mut names = BTreeMap::new();
    let mut slugs = BTreeSet::new();
    let mut legacy = 0;
    for entry in entries {
        if entry.name.trim().is_empty() || entry.dim == 0 || !slugs.insert(entry.slug())
            || names.insert(entry.name.clone(), entry.clone()).is_some() {
            return Err(refusal(RefusalCode::ModelMismatch, location, "missing/duplicate model or slug collision"));
        }
        if entry.storage == StorageKind::Legacy { legacy += 1; }
    }
    if legacy > 1 { return Err(refusal(RefusalCode::ModelMismatch, location, "multiple models share legacy storage")); }
    Ok(names)
}

/// Compute a preview from supplied facts. `expected_source_identity` is an
/// explicit comparison input, NOT extracted from the current resolved plan.
/// Even matching declared values grant no proof of provenance or write access.
pub fn preview_known_transfer(
    fixed_plan: &ResolvedMigrationPlan,
    expected_source_identity: Option<&str>,
    inventory: &TransferInventory,
    recipe: &TransferRecipe,
) -> Result<KnownTransferPreview, Vec<TransferRefusal>> {
    preview(fixed_plan, expected_source_identity, inventory, recipe).map_err(|e| vec![e])
}
fn preview(
    plan: &ResolvedMigrationPlan, expected: Option<&str>, input: &TransferInventory, recipe: &TransferRecipe,
) -> Result<KnownTransferPreview, TransferRefusal> {
    use RefusalCode::*;
    let expected = expected.filter(|s| !s.trim().is_empty())
        .ok_or_else(|| refusal(ExpectedIdentityMissing, "origin", "no expected source instance identity supplied"))?;
    let declared = input.declared_source_identity.as_deref().filter(|s| !s.trim().is_empty())
        .ok_or_else(|| refusal(SourceIdentityMissing, "origin", "inventory declares no source instance identity"))?;
    if declared != expected { return Err(refusal(ForeignSourceIdentity, "origin", "declared identity differs from explicit expectation")); }
    plan.validate_generation_preconditions().map_err(|e| refusal(GenerationUnavailable, "plan", e))?;
    if input.descriptor != plan.inputs().source || recipe.target_descriptor != plan.inputs().target {
        return Err(refusal(DescriptorMismatch, "descriptors", "supplied descriptors differ from fixed inputs"));
    }
    let cfg = &input.source_config;
    let target = &recipe.target_config;
    if cfg.signals.sparse() || target.signals.sparse() || input.sparse.materialized || input.sparse.debt || input.sparse.inconsistent {
        return Err(refusal(SparseUnsupported, "sparse", "sparse/dual data, debt or inconsistent evidence cannot be transported"));
    }
    if input.dense_debt { return Err(refusal(DenseDebt, "dense", "source declares unavailable dense values")); }
    cfg.validate().and_then(|_| target.validate()).map_err(|e| refusal(UnsupportedRecipe, "config", e))?;
    let derived = target.derived.as_ref().ok_or_else(|| refusal(UnsupportedRecipe, "target", "content derivation required"))?;
    let DerivedIdentity::Content { format } = &derived.identity else {
        return Err(refusal(UnsupportedRecipe, "target", "content identity required"));
    };
    // Inline rendering alone: render_fields otherwise can read user templates.
    if derived.from != input.source_entity || !derived.gather.is_empty()
        || derived.render.values().any(|s| !s.contains("{{") && !s.contains("{%"))
        || input.source_entity.trim().is_empty() || recipe.target_entity.trim().is_empty()
        || input.source_entity == recipe.target_entity || cfg.derived.is_some()
        || cfg.chunked == Some(false) || target.chunked == Some(false)
        || cfg.search_index.is_some() || target.search_index.is_some()
        || cfg.signals != target.signals || !cfg.signals.bm25()
        || cfg.fields.values().filter(|f| f.is_title || f.title_for.as_deref() == Some("self")).count() > 1
        || target.fields.keys().collect::<BTreeSet<_>>() != derived.render.keys().collect::<BTreeSet<_>>()
        || format != &recipe.target_descriptor.application_format
        || cfg.content_fields() != target.content_fields() || cfg.title_field() != target.title_field()
        || encoded(&cfg.chunking)? != encoded(&target.chunking)?
        || target.fields.iter().any(|(name,f)| cfg.fields.get(name).map(|s| encoded(s).ok()) != Some(encoded(f).ok())) {
        return Err(refusal(UnsupportedRecipe, "config", "only unchanged field roles, inline renders and chunk recipe are supported"));
    }
    let source_models = models(&input.models, "source models")?;
    let target_models = models(&recipe.models, "target models")?;
    if source_models.len() != target_models.len()
        || source_models.iter().any(|(n,s)| target_models.get(n).is_none_or(|t| t.dim != s.dim))
        || (cfg.signals.vector() && source_models.is_empty()) || (!cfg.signals.vector() && !source_models.is_empty()) {
        return Err(refusal(ModelMismatch, "models", "complete identical model names/dimensions required"));
    }
    let mut inventory = input.clone();
    inventory.entities.sort_by_key(|e| e.key.key());
    inventory.chunks.sort_by_key(|c| (c.parent.key(), c.uuid.clone()));
    inventory.models.sort_by_key(|m| m.name.clone());
    for chunk in &mut inventory.chunks { chunk.dense.sort_by_key(|v| v.model.clone()); }
    // Serialized properties are validated before they are used for sorting/hashing.
    if inventory.entities.iter().any(|e| !valid_values(&e.values))
        || inventory.relations.iter().any(|e| !valid_values(&e.properties)) {
        return Err(refusal(InvalidInventory, "values", "non-finite or unsupported values cannot be fingerprinted faithfully"));
    }
    let mut relations = inventory.relations.iter().map(|r| encoded(r).map(|s| (s,r.clone())))
        .collect::<Result<Vec<_>,_>>()?;
    relations.sort_by(|a,b| a.0.cmp(&b.0));
    inventory.relations = relations.into_iter().map(|(_,r)| r).collect();
    let mut nodes = BTreeSet::new();
    for entity in &inventory.entities {
        if entity.key.entity.trim().is_empty() || entity.key.uuid.trim().is_empty()
            || entity.key.scope.validate().is_err() || !nodes.insert(entity.key.key()) {
            return Err(refusal(InvalidInventory, entity.key.uuid.clone(), "invalid or duplicate entity key"));
        }
        for (name, value) in [("_uuid", &entity.key.uuid), ("_org", &entity.key.scope.org), ("_project", &entity.key.scope.project)] {
            if entity.values.get(name).is_some_and(|v| v.as_str() != Some(value)) {
                return Err(refusal(InvalidInventory, entity.key.uuid.clone(), "system columns contradict the entity key"));
            }
        }
        if entity.key.entity == recipe.target_entity {
            return Err(refusal(InvalidInventory, entity.key.uuid.clone(), "target entity already present"));
        }
    }
    for relation in &inventory.relations {
        if relation.name.trim().is_empty() || !nodes.contains(&relation.from.key()) || !nodes.contains(&relation.to.key()) {
            return Err(refusal(InvalidInventory, &relation.name, "relation endpoint missing from retained graph"));
        }
    }
    let mut observed = BTreeMap::new();
    for chunk in &inventory.chunks {
        if !nodes.contains(&chunk.parent.key()) || chunk.parent.entity != inventory.source_entity
            || chunk.uuid != chunk_uuid(&chunk.parent.uuid, &chunk.shape.field, chunk.shape.chunk.index)
            || observed.insert((chunk.parent.key(), chunk.uuid.clone()), chunk).is_some() {
            return Err(refusal(ChunkCoverage, &chunk.uuid, "orphan, duplicate or incorrectly identified source chunk"));
        }
    }
    let chunker = Chunker::new(ChunkerConfig::from(&target.chunking));
    let mut versions: BTreeMap<_, PreviewVersion> = BTreeMap::new();
    let mut chunks: BTreeMap<_, PreviewChunk> = BTreeMap::new();
    let mut consumed = BTreeSet::new();
    let mut occurrence_count = 0;
    for occurrence in inventory.entities.iter().filter(|e| e.key.entity == inventory.source_entity) {
        occurrence_count += 1;
        let context = render_context(&occurrence.values, &BTreeMap::new());
        let rendered = render_fields(derived, &context).map_err(|e| refusal(UnsupportedRecipe, &occurrence.key.uuid, e))?;
        if rendered.iter().any(|(name,text)| occurrence.values.get(name).and_then(|v| v.as_str()) != Some(text.as_str())) {
            return Err(refusal(UnsupportedRecipe, &occurrence.key.uuid, "rendered field is not byte-identical to its source field"));
        }
        let uuid = derived_content_uuid(&recipe.target_entity, format, target, &rendered, &occurrence.key.scope)
            .map_err(|e| refusal(UnsupportedRecipe, &occurrence.key.uuid, e))?;
        let key = EntityKey { entity: recipe.target_entity.clone(), scope: occurrence.key.scope.clone(), uuid };
        let version = versions.entry(key.key()).or_insert_with(|| PreviewVersion {
            key: key.clone(), fields: rendered.clone(), occurrences: BTreeMap::new(),
        });
        if version.fields != rendered { return Err(refusal(InvalidInventory, &key.uuid, "content identity collision")); }
        version.occurrences.insert(occurrence.key.uuid.clone(), render_inputs_hash(&context));
        let title = target.title_field().and_then(|f| rendered.get(f)).cloned().unwrap_or_default();
        let mut offset = 0;
        for (i, field) in target.content_fields().into_iter().enumerate() {
            if i > 0 { offset += 2; }
            let text = rendered.get(field).map(String::as_str).unwrap_or("");
            for chunk in chunker.chunk(text) {
                let shape = ChunkShape { field: field.into(), title: title.clone(), content_offset: offset, chunk };
                let source_uuid = chunk_uuid(&occurrence.key.uuid, field, shape.chunk.index);
                let source_key = (occurrence.key.key(), source_uuid.clone());
                let source = observed.get(&source_key).ok_or_else(|| refusal(ChunkCoverage, &source_uuid, "required source chunk missing"))?;
                if source.shape != shape { return Err(refusal(ChunkCoverage, &source_uuid, "text, title or full/core coordinates differ")); }
                consumed.insert(source_key);
                let uuid = chunk_uuid(&key.uuid, field, shape.chunk.index);
                let destination = chunks.entry((key.key(), uuid.clone())).or_insert_with(|| PreviewChunk {
                    parent: key.clone(), uuid, shape: shape.clone(), dense: BTreeMap::new(),
                });
                if destination.shape != shape { return Err(refusal(ChunkCoverage, &source_uuid, "destination shape collision")); }
                let mut seen_models = BTreeSet::new();
                for known in &source.dense {
                    let location = format!("{}/{}/{}/{}/{}", occurrence.key.scope.org,
                        occurrence.key.scope.project, occurrence.key.entity, source_uuid, known.model);
                    let entry = source_models.get(&known.model).ok_or_else(|| refusal(ModelMismatch, &location, "unknown vector model"))?;
                    if !seen_models.insert(known.model.clone()) { return Err(refusal(DenseCoverage, &location, "duplicate model value")); }
                    if known.input.as_deref() != Some(shape.chunk.text.as_str()) {
                        return Err(refusal(InputProvenanceUnavailable, &location, "known `_text` model input required"));
                    }
                    if known.marker != content_hash(&shape.chunk.text) { return Err(refusal(StaleMarker, &location, "model marker does not match chunk text")); }
                    if known.values.len() != entry.dim || known.values.iter().any(|v| !v.is_finite()) {
                        return Err(refusal(InvalidVector, &location, "dimension mismatch or non-finite values"));
                    }
                    let bits: Vec<_> = known.values.iter().map(|v| v.to_bits()).collect();
                    if let Some(binding) = destination.dense.get_mut(&known.model) {
                        if binding.value_bits != bits { return Err(refusal(ConflictingVectors, &location, format!("conflicting values for {}", known.model))); }
                        binding.sources.insert(source_uuid.clone());
                    } else {
                        destination.dense.insert(known.model.clone(), DenseBinding {
                            model: known.model.clone(), marker: known.marker.clone(), value_bits: bits,
                            source_storage: storage(&format!("{}_Chunk", inventory.source_entity), entry),
                            target_storage: storage(&format!("{}_Chunk", recipe.target_entity), &target_models[&known.model]),
                            sources: BTreeSet::from([source_uuid.clone()]),
                        });
                    }
                }
                if seen_models.len() != source_models.len() { return Err(refusal(DenseCoverage, &source_uuid, "not all declared models cover this source chunk")); }
            }
            offset += text.len();
        }
    }
    if occurrence_count == 0 || chunks.is_empty() || consumed.len() != observed.len() {
        return Err(refusal(ChunkCoverage, "inventory", "empty transfer or extra/uncovered source chunks"));
    }
    let inventory_fingerprint = content_hash(&encoded(&inventory)?);
    let versions: Vec<_> = versions.into_values().collect();
    let chunks: Vec<_> = chunks.into_values().collect();
    let mut normalized_recipe = recipe.clone();
    normalized_recipe.models.sort_by_key(|m| m.name.clone());
    let identity = content_hash(&encoded(&("known-transfer-preview-v1", plan.identity(), expected,
        &inventory_fingerprint, &normalized_recipe, &versions, &chunks))?);
    Ok(KnownTransferPreview { identity, fixed_plan_identity: plan.identity().into(), inventory_fingerprint,
        retained_entities: inventory.entities, retained_relations: inventory.relations, versions, chunks })
}

#[cfg(test)]
mod tests;

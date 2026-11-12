//! Binding of complete SUPPLIED declarations to fixed migration inputs.
//!
//! This module does not observe storage, establish an origin/generation, connect
//! a model or grant execution. Missing declarations are not empty declarations.
use std::collections::{BTreeMap, BTreeSet};
use serde::Serialize;
use crate::{backend::BackendRelation, config::{EntityConfig, RelationDef},
    embedding_storage::{EmbeddingModelEntry, StorageKind, validate_slug}, hash::content_hash,
    model_source::{Capability, ModelSource}, search::SearchSignals};
use super::plan::{MigrationPlanInputs, ResolvedMigrationPlan, StorageDescriptor, StorageGeneration};

const CONFIG_DOMAIN: &str = "declared-entity-configs:v1:";
const FACTS_DOMAIN: &str = "declared-migration-facts:v1:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FactsError {
    Incomplete(String), Invalid(String), UnresolvedRecipe(String),
    UnknownContract(String), Mismatch(String),
}
impl std::fmt::Display for FactsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for FactsError {}

/// Explicit provider knowledge: absent, complete definitions, or unavailable.
/// `Absent` is an assertion about the supplied declaration, not discovery.
#[derive(Debug, Clone, Serialize)]
pub enum ProviderFacts {
    Absent,
    Complete { name: String, entities: BTreeMap<String, EntityConfig>, relations: BTreeMap<String, RelationDef> },
    Unavailable { name: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct RecipeFactsInput {
    pub id: String,
    /// Source bytes for non-inline derived templates, indexed by their spec.
    /// None is unknown; Some(empty) explicitly declares no such templates.
    pub resolved_templates: Option<BTreeMap<String, String>>,
    /// Complete declarations, already normalized from legacy aliases by caller.
    /// No environment resolution or builders are called here.
    pub model_sources: Option<BTreeMap<Capability, ModelSource>>,
    pub dense_models: Option<BTreeMap<String, EmbeddingModelEntry>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeclaredSchemaInput {
    /// Effective EntityConfig values, including schema fields already merged.
    pub entities: Option<BTreeMap<String, EntityConfig>>,
    pub relations: Option<BTreeMap<String, BackendRelation>>,
    pub provider: ProviderFacts,
    /// Supplied final workspace signal overrides, never read from environment.
    pub index_signals: BTreeMap<String, SearchSignals>,
    pub recipe: RecipeFactsInput,
}

/// Immutable normalized facts, constructed through the existing schema validator.
#[derive(Debug, Clone, Serialize)]
pub struct DeclaredSchemaFacts {
    input: DeclaredSchemaInput,
    entities: BTreeMap<String, EntityConfig>,
    relations: BTreeMap<String, RelationDef>,
    config_fingerprint: String,
}
impl DeclaredSchemaFacts {
    pub fn from_complete_input(input: DeclaredSchemaInput) -> Result<Self, FactsError> {
        if input.entities.is_none() { return Err(FactsError::Incomplete("effective entity configs".into())); }
        if input.relations.is_none() { return Err(FactsError::Incomplete("relation definitions/policies".into())); }
        match &input.provider {
            ProviderFacts::Unavailable { name } => return Err(FactsError::Incomplete(format!("provider {name}"))),
            ProviderFacts::Complete { name, entities, relations } => {
                if name.trim().is_empty() { return Err(FactsError::Invalid("empty provider name".into())); }
                validate_finite(entities, relations)?;
            },
            ProviderFacts::Absent => {},
        }
        let entities = input.entities.as_ref().expect("checked above");
        let relations = input.relations.as_ref().expect("checked above");
        let defs = relations.iter().map(|(n,r)| (n.clone(),r.def.clone())).collect();
        validate_finite(entities, &defs)?;
        let templates = input.recipe.resolved_templates.as_ref()
            .ok_or_else(|| FactsError::Incomplete("resolved recipe templates".into()))?;
        let model_sources = input.recipe.model_sources.as_ref()
            .ok_or_else(|| FactsError::Incomplete("effective model declarations".into()))?;
        let models = input.recipe.dense_models.as_ref()
            .ok_or_else(|| FactsError::Incomplete("dense model registry".into()))?;
        if input.recipe.id.trim().is_empty() { return Err(FactsError::Invalid("empty recipe identifier".into())); }
        let (entities, relations) = Self::compose_effective(&input).map_err(FactsError::Invalid)?;
        validate_finite(&entities, &relations)?;
        let mut required_templates = BTreeSet::new();
        for config in entities.values() {
            if let Some(derived) = &config.derived {
                for spec in derived.render.values() {
                    // This is the inline rule of the existing template resolver;
                    // named bytes must be provided instead of loading a file.
                    if !spec.contains("{{") && !spec.contains("{%") {
                        required_templates.insert(spec.clone());
                        if !templates.contains_key(spec) { return Err(FactsError::UnresolvedRecipe(spec.clone())); }
                    }
                }
            }
        }
        if templates.keys().any(|k| !required_templates.contains(k)) {
            return Err(FactsError::Invalid("unreferenced resolved template in recipe".into()));
        }
        let mut slugs = BTreeSet::new();
        let mut legacy = 0;
        for (name, model) in models {
            if name != &model.name || name.trim().is_empty() || model.dim == 0 || !slugs.insert(model.slug()) {
                return Err(FactsError::Invalid("invalid dense registry or colliding model slugs".into()));
            }
            validate_slug(&model.slug()).map_err(FactsError::Invalid)?;
            if model.storage == StorageKind::Legacy { legacy += 1; }
        }
        if legacy > 1 { return Err(FactsError::Invalid("multiple legacy dense columns".into())); }
        if entities.values().any(|c| c.signals.vector()) && models.is_empty() {
            return Err(FactsError::Incomplete("dense registry for vector signals".into()));
        }
        for source in model_sources.values() {
            if source.model.trim().is_empty() || source.dimensions == Some(0) || source.context_tokens == Some(0) {
                return Err(FactsError::Invalid("invalid effective model declaration".into()));
            }
        }
        if let Some(embed) = model_sources.get(&Capability::Embed) {
            let entry = models.get(&embed.model).ok_or_else(|| FactsError::Incomplete("declared embed model missing from registry".into()))?;
            if embed.dimensions.is_some_and(|d| d != entry.dim) {
                return Err(FactsError::Invalid("embed model dimension contradicts registry".into()));
            }
        }
        let config_fingerprint = digest(CONFIG_DOMAIN, &entities)?;
        Ok(Self { input, entities, relations, config_fingerprint })
    }
    pub fn entities(&self) -> &BTreeMap<String, EntityConfig> { &self.entities }
    pub fn relations(&self) -> &BTreeMap<String, RelationDef> { &self.relations }
    pub fn config_fingerprint(&self) -> &str { &self.config_fingerprint }
}

/// All facts are declared inputs. No source is represented as observed/persisted.
#[derive(Debug, Clone, Serialize)]
pub struct DeclaredMigrationFacts {
    source: DeclaredSchemaFacts,
    target: DeclaredSchemaFacts,
    schema_facts_fingerprint: String,
}
impl DeclaredMigrationFacts {
    pub fn new(source: DeclaredSchemaFacts, target: DeclaredSchemaFacts) -> Result<Self, FactsError> {
        let schema_facts_fingerprint = digest(FACTS_DOMAIN, &(&source, &target))?;
        Ok(Self { source, target, schema_facts_fingerprint })
    }
    pub fn source(&self) -> &DeclaredSchemaFacts { &self.source }
    pub fn target(&self) -> &DeclaredSchemaFacts { &self.target }
    pub fn schema_facts_fingerprint(&self) -> &str { &self.schema_facts_fingerprint }

    /// Generate both config fingerprints and the full facts fingerprint. No
    /// caller-supplied fingerprint is accepted on this constructor path.
    pub fn bind_inputs(&self, source: StorageFormatFacts, target: StorageFormatFacts,
        variables: BTreeMap<String, String>) -> Result<MigrationPlanInputs, FactsError>
    {
        let descriptor = |format: StorageFormatFacts, facts: &DeclaredSchemaFacts| {
            let d = StorageDescriptor { engine_format: format.engine_format,
                internal_schema: format.internal_schema, application_format: format.application_format,
                generation: format.generation, recipe: facts.input.recipe.id.clone(),
                config_fingerprint: facts.config_fingerprint.clone() };
            d.validate().map_err(FactsError::Invalid)?;
            Ok::<_, FactsError>(d)
        };
        Ok(MigrationPlanInputs { source: descriptor(source, &self.source)?, target: descriptor(target, &self.target)?,
            variables, schema_facts_fingerprint: self.schema_facts_fingerprint.clone() })
    }

    /// Only declaration-to-plan correspondence. Unversioned is valid here;
    /// execution generation checks and storage provenance remain separate.
    pub fn compare_with_fixed_plan(&self, plan: &ResolvedMigrationPlan) -> Result<DeclaredFactsMatch, FactsError> {
        let i = plan.inputs();
        for (label, value, domain, expected) in [
            ("source configs", i.source.config_fingerprint.as_str(), CONFIG_DOMAIN, self.source.config_fingerprint.as_str()),
            ("target configs", i.target.config_fingerprint.as_str(), CONFIG_DOMAIN, self.target.config_fingerprint.as_str()),
            ("schema facts", i.schema_facts_fingerprint.as_str(), FACTS_DOMAIN, self.schema_facts_fingerprint.as_str()),
        ] {
            check_contract(value, domain, label)?;
            if value != expected { return Err(FactsError::Mismatch(label.into())); }
        }
        if i.source.recipe != self.source.input.recipe.id || i.target.recipe != self.target.input.recipe.id {
            return Err(FactsError::Mismatch("recipe identifier".into()));
        }
        Ok(DeclaredFactsMatch { referenced_plan_identity: plan.identity().into(),
            schema_facts_fingerprint: self.schema_facts_fingerprint.clone(), provenance: FactsProvenance::SuppliedDeclarations })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StorageFormatFacts {
    pub engine_format: String,
    pub internal_schema: String,
    pub application_format: String,
    pub generation: StorageGeneration,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FactsProvenance { SuppliedDeclarations }
/// Sealed report: no Deserialize, storage observation, journal or writer token.
#[derive(Debug, Clone, Serialize)]
pub struct DeclaredFactsMatch {
    referenced_plan_identity: String,
    schema_facts_fingerprint: String,
    provenance: FactsProvenance,
}
impl DeclaredFactsMatch {
    pub fn referenced_plan_identity(&self) -> &str { &self.referenced_plan_identity }
    pub fn schema_facts_fingerprint(&self) -> &str { &self.schema_facts_fingerprint }
    pub fn provenance(&self) -> FactsProvenance { self.provenance }
}
fn check_contract(value: &str, domain: &str, label: &str) -> Result<(), FactsError> {
    match value.strip_prefix(domain) {
        Some(hash) if hash.len()==64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) => Ok(()),
        _ => Err(FactsError::UnknownContract(label.into())),
    }
}
fn digest<T: Serialize>(domain: &str, value: &T) -> Result<String, FactsError> {
    Ok(format!("{domain}{}", content_hash(&canonical_json(value).map_err(FactsError::Invalid)?)))
}

/// Encoding mechanism shared with the preview report, not its semantic domain.
/// Callers must validate non-finite/unsupported typed values before serialization.
pub(super) fn canonical_json<T: Serialize>(value: &T) -> Result<String, String> {
    fn canonical(v: serde_json::Value) -> serde_json::Value {
        match v {
            serde_json::Value::Object(m) => serde_json::Value::Object(m.into_iter().collect::<BTreeMap<_, _>>()
                .into_iter().map(|(k,v)| (k,canonical(v))).collect()),
            serde_json::Value::Array(a) => serde_json::Value::Array(a.into_iter().map(canonical).collect()),
            v => v,
        }
    }
    serde_json::to_value(value).map(canonical).and_then(|v| serde_json::to_string(&v)).map_err(|e| e.to_string())
}
pub(super) fn entity_config_is_finite(c: &EntityConfig) -> bool {
    c.fields.values().all(|f| f.boost.is_none_or(|v| v.is_finite()))
        && c.snapshot.as_ref().is_none_or(|s| s.max_missing_ratio.is_finite())
        && c.fusion.as_ref().is_none_or(|f| f.rrf_k.is_finite()
            && f.bm25.weight.is_finite() && f.vector.weight.is_finite() && f.sparse.weight.is_finite())
        && c.field_weights.iter().all(|w| w.default.is_finite() && w.weights.values().all(|v| v.is_finite()))
}
fn validate_finite(entities: &BTreeMap<String, EntityConfig>, relations: &BTreeMap<String, RelationDef>) -> Result<(), FactsError> {
    if entities.values().any(|c| !entity_config_is_finite(c))
        || relations.values().any(|r| r.properties.as_ref().is_some_and(|p| p.values().any(|f| f.boost.is_some_and(|v| !v.is_finite())))) {
        return Err(FactsError::Invalid("non-finite schema value cannot be fingerprinted faithfully".into()));
    }
    Ok(())

#[cfg(test)]
mod tests;

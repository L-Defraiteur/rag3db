//! Observations cover persisted namespaces, not all effective backend facts.
use super::*;
use crate::{config::{EntityConfig, RelationDef}, dataflow::migrations::{AppliedMigration,
    facts::{DeclaredSchemaFacts, DeclaredSchemaInput, FactsError}}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MetadataReadProvenance {
    /// A connection view without a successful read-only transaction boundary.
    ConnectionView,
    /// Values read together in the transaction that has since committed.
    /// This is neither a live lease nor a durable generation/publication.
    ReadOnlySnapshot,
}

/// Immutable values from persisted metadata namespaces and the same journal view.
/// None means the owning table is absent; Some(empty) is an observed empty set.
/// Completeness concerns these persisted namespaces only, never missing provider,
/// relation-policy, template, model or data facts. No Deserialize or public builder.
#[derive(Debug, Clone, Serialize)]
pub struct ObservedCatalogMetadata {
    pub(super) entity_configs: Option<BTreeMap<String, EntityConfig>>,
    pub(super) relations: Option<BTreeMap<String, RelationDef>>,
    pub(super) journal: Option<BTreeMap<u64, AppliedMigration>>,
    pub(super) provenance: MetadataReadProvenance,
}
impl ObservedCatalogMetadata {
    pub fn entity_configs(&self) -> Option<&BTreeMap<String, EntityConfig>> { self.entity_configs.as_ref() }
    pub fn relations(&self) -> Option<&BTreeMap<String, RelationDef>> { self.relations.as_ref() }
    pub fn journal(&self) -> Option<&BTreeMap<u64, AppliedMigration>> { self.journal.as_ref() }
    pub fn provenance(&self) -> MetadataReadProvenance { self.provenance }

    /// Validate the supplied effective schema with A's sole producer, then check
    /// every persisted entity/relation against it. Extra supplied definitions,
    /// providers, policies, templates and models remain SUPPLIED, never observed.
    /// No plan, journal status, generation or storage admission is qualified.
    pub fn compare_with_supplied_schema(&self, input: DeclaredSchemaInput) -> Result<PersistedSchemaComparison, FactsError> {
        if self.provenance != MetadataReadProvenance::ReadOnlySnapshot {
            return Err(FactsError::Incomplete("read-only metadata snapshot".into()));
        }
        let entities = self.entity_configs.as_ref().ok_or_else(|| FactsError::Incomplete("persisted entity namespace".into()))?;
        let relations = self.relations.as_ref().ok_or_else(|| FactsError::Incomplete("persisted relation namespace".into()))?;
        let supplied_schema = DeclaredSchemaFacts::from_complete_input(input)?;
        for (name, observed) in entities {
            let supplied = supplied_schema.entities().get(name).ok_or_else(|| FactsError::Mismatch(format!("persisted entity {name} missing from supplied schema")))?;
            if serde_json::to_value(observed).map_err(|e| FactsError::Invalid(e.to_string()))?
                != serde_json::to_value(supplied).map_err(|e| FactsError::Invalid(e.to_string()))? {
                return Err(FactsError::Mismatch(format!("persisted entity {name}")));
            }
        }
        for (name, observed) in relations {
            let supplied = supplied_schema.relations().get(name).ok_or_else(|| FactsError::Mismatch(format!("persisted relation {name} missing from supplied schema")))?;
            if serde_json::to_value(observed).map_err(|e| FactsError::Invalid(e.to_string()))?
                != serde_json::to_value(supplied).map_err(|e| FactsError::Invalid(e.to_string()))? {
                return Err(FactsError::Mismatch(format!("persisted relation {name}")));
            }
        }
        Ok(PersistedSchemaComparison { observed_metadata: self.clone(), supplied_schema })
    }
}

/// Keeps observed metadata separate from the supplied schema, including all its
/// declared-only facts. This report is not a storage qualification or write token.
#[derive(Debug, Serialize)]
pub struct PersistedSchemaComparison {
    observed_metadata: ObservedCatalogMetadata,
    supplied_schema: DeclaredSchemaFacts,
}
impl PersistedSchemaComparison {
    pub fn observed_metadata(&self) -> &ObservedCatalogMetadata { &self.observed_metadata }
    pub fn supplied_schema(&self) -> &DeclaredSchemaFacts { &self.supplied_schema }
}

//! Immutable migration identity, independent of the current storage generation.
//! This contract grants neither write access nor permission to publish a target.
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::hash::content_hash;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "fingerprint", rename_all = "snake_case")]
pub enum StorageGeneration {
    /// Legacy metadata has no established data generation. Never means zero.
    Unversioned,
    Established(String),
}

/// Engine, internal schema and application format have separate identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageDescriptor {
    pub engine_format: String,
    pub internal_schema: String,
    pub application_format: String,
    pub recipe: String,
    pub config_fingerprint: String,
    pub generation: StorageGeneration,
}

impl StorageDescriptor {
    fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("engine_format", &self.engine_format),
            ("internal_schema", &self.internal_schema),
            ("application_format", &self.application_format),
            ("recipe", &self.recipe),
            ("config_fingerprint", &self.config_fingerprint),
        ] {
            if value.trim().is_empty() { return Err(format!("empty {name}")); }
        }
        if let StorageGeneration::Established(value) = &self.generation {
            if value.trim().is_empty() { return Err("empty generation fingerprint".into()); }
        }
        Ok(())
    }
}

/// Inputs fixed when planning. Do not replace source with current target metadata
/// when qualifying a replay after publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationPlanInputs {
    pub source: StorageDescriptor,
    pub target: StorageDescriptor,
    pub variables: BTreeMap<String, String>,
    /// Fingerprint of the provider's resolved schema facts, not its name alone.
    pub schema_facts_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedMigrationStep {
    pub version: u64,
    pub name: String,
    pub checksum: String,
    pub graph_hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedMigrationPlan {
    inputs: MigrationPlanInputs,
    steps: Vec<ResolvedMigrationStep>,
    identity: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoragePosition { Source, PublishedTarget }

impl ResolvedMigrationPlan {
    pub(super) fn new(inputs: MigrationPlanInputs, steps: Vec<ResolvedMigrationStep>) -> Result<Self, String> {
        inputs.source.validate()?;
        inputs.target.validate()?;
        if inputs.schema_facts_fingerprint.trim().is_empty() {
            return Err("missing resolved schema facts fingerprint".into());
        }
        if steps.is_empty() { return Err("no declared migration steps".into()); }
        let encoded = serde_json::to_string(&("resolved-migration-plan-v1", &inputs, &steps))
            .map_err(|e| e.to_string())?;
        Ok(Self { inputs, steps, identity: content_hash(&encoded) })
    }

    pub fn identity(&self) -> &str { &self.identity }
    pub fn inputs(&self) -> &MigrationPlanInputs { &self.inputs }
    pub fn steps(&self) -> &[ResolvedMigrationStep] { &self.steps }

    /// An inspectable preliminary plan is allowed for legacy storage. Execution
    /// qualification requires established source AND target generations. This
    /// checks storage identity only, not node registry, ports, cycles or transport.
    pub fn validate_generation_preconditions(&self) -> Result<(), String> {
        if matches!(self.inputs.source.generation, StorageGeneration::Unversioned) {
            return Err("source generation not established".into());
        }
        if matches!(self.inputs.target.generation, StorageGeneration::Unversioned) {
            return Err("target generation not established".into());
        }
        if self.inputs.source == self.inputs.target {
            return Err("source and target descriptors are identical".into());
        }
        Ok(())
    }

    /// Qualification is pure. The caller must obtain the durable journal state
    /// and storage descriptor under actual exclusivity before any effects.
    /// `applied_plan_identity` is the identity recorded by publication, not a
    /// checkpoint's Completed flag. This method does not create that record.
    pub fn qualify_storage(&self, current: &StorageDescriptor, applied_plan_identity: Option<&str>)
        -> Result<StoragePosition, String>
    {
        self.validate_generation_preconditions()?;
        match applied_plan_identity {
            None if current == &self.inputs.source => Ok(StoragePosition::Source),
            Some(identity) if identity == self.identity && current == &self.inputs.target =>
                Ok(StoragePosition::PublishedTarget),
            Some(identity) if identity != self.identity => Err("resolved migration plan changed".into()),
            _ => Err("storage does not match the generation expected by the fixed plan".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::{migrations::MigrationRunner, node_registry::NodeRegistry};
    use std::sync::Arc;

    fn descriptor(format: &str, generation: &str) -> StorageDescriptor {
        StorageDescriptor {
            engine_format: "fixture-engine".into(), internal_schema: "8".into(),
            application_format: format.into(), recipe: "fixture-recipe".into(),
            config_fingerprint: format!("configs-{format}"),
            generation: StorageGeneration::Established(generation.into()),
        }
    }

    fn inputs() -> MigrationPlanInputs {
        MigrationPlanInputs {
            source: descriptor("legacy", "source-data"),
            target: descriptor("shared", "target-data"),
            variables: BTreeMap::from([("query".into(), "RETURN 1".into())]),
            schema_facts_fingerprint: "fixture-provider-facts".into(),
        }
    }

    fn resolve(inputs: MigrationPlanInputs) -> ResolvedMigrationPlan {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("001_check.mmd"),
            "graph TD\ncheck[\"ValidateNode(query='$query', assert='count == 1')\"]\n").unwrap();
        let mut runner = MigrationRunner::new(Arc::new(NodeRegistry::new()));
        runner.add_dir(dir.path().to_owned());
        runner.resolve_plan(inputs).unwrap()
    }

    #[test]
    fn fixed_plan_qualifies_source_then_published_target_without_reresolving() {
        let plan = resolve(inputs());
        let identity = plan.identity().to_owned();
        assert_eq!(plan.qualify_storage(&plan.inputs().source, None).unwrap(), StoragePosition::Source);
        assert_eq!(plan.qualify_storage(&plan.inputs().target, Some(&identity)).unwrap(), StoragePosition::PublishedTarget);
        assert_eq!(plan.identity(), identity);
        // Persist original inputs, then rebuild after publication. The live
        // target is used only for qualification, never as the new source.
        let persisted = serde_json::to_string(plan.inputs()).unwrap();
        let restored = resolve(serde_json::from_str(&persisted).unwrap());
        assert_eq!(restored.identity(), identity);
        assert_eq!(restored.qualify_storage(&plan.inputs().target, Some(&identity)).unwrap(), StoragePosition::PublishedTarget);
        let mut foreign = plan.inputs().target.clone();
        foreign.generation = StorageGeneration::Established("foreign-write".into());
        assert!(plan.qualify_storage(&foreign, Some(&identity)).is_err());
        assert!(plan.qualify_storage(&plan.inputs().target, None).is_err());
        assert!(plan.qualify_storage(&plan.inputs().source, Some(&identity)).is_err());
    }

    #[test]
    fn resolved_identity_covers_variables_configs_facts_and_generations() {
        let original = inputs();
        let plan = resolve(original.clone());
        let mut variants = Vec::new();
        let mut changed = original.clone(); changed.variables.insert("query".into(), "RETURN 2".into()); variants.push(changed);
        let mut changed = original.clone(); changed.source.config_fingerprint.push('x'); variants.push(changed);
        let mut changed = original.clone(); changed.schema_facts_fingerprint.push('x'); variants.push(changed);
        let mut changed = original.clone(); changed.target.recipe.push('x'); variants.push(changed);
        let mut changed = original.clone(); changed.target.generation = StorageGeneration::Established("other".into()); variants.push(changed);
        for changed in variants {
            let other = resolve(changed);
            assert_ne!(plan.identity(), other.identity());
            assert!(other.qualify_storage(&other.inputs().target, Some(plan.identity())).is_err());
        }
        let mut changed_variables = original.clone();
        changed_variables.variables.insert("query".into(), "RETURN 2".into());
        assert_ne!(plan.steps()[0].graph_hash, resolve(changed_variables).steps()[0].graph_hash);
        assert_eq!(plan.identity(), resolve(original).identity());
    }

    #[test]
    fn legacy_unversioned_is_inspectable_but_never_execution_ready() {
        let mut inputs = inputs();
        inputs.source.generation = StorageGeneration::Unversioned;
        let plan = resolve(inputs);
        assert!(!plan.steps().is_empty());
        assert_eq!(plan.validate_generation_preconditions().unwrap_err(), "source generation not established");
        assert!(plan.qualify_storage(&plan.inputs().source, None).is_err());
    }
}

//! Frozen source support for the initial, owned BM25 profile.
//! This is input to MigrationRunner, not another graph executor or status journal.
use super::plan::{MigrationPlanInputs, ResolvedMigrationPlan, ResolvedMigrationStep};
use super::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceStep {
    version: u64,
    name: String,
    content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FrozenInitialProgram {
    steps: Vec<SourceStep>,
    variables: BTreeMap<String, String>,
}
impl FrozenInitialProgram {
    fn definitions(
        &self,
    ) -> Result<Vec<super::super::checkpoint::GraphDefinition>, MigrationError> {
        let vars = self
            .variables
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut previous = None;
        self.steps
            .iter()
            .map(|step| {
                if step.name.trim().is_empty()
                    || previous.is_some_and(|version| version >= step.version)
                {
                    return Err(MigrationError::InvalidPlan(
                        "unordered/invalid frozen source steps".into(),
                    ));
                }
                previous = Some(step.version);
                let graph = parse_mermaid_template(&step.content, &vars)
                    .map_err(|e| MigrationError::InvalidPlan(e.to_string()))?;
                if graph.nodes.is_empty()
                    || graph
                        .nodes
                        .iter()
                        .any(|node| node.node_type != "IngestEntitiesNode")
                {
                    return Err(MigrationError::InvalidPlan(
                        "initial BM25 profile requires only complete IngestEntitiesNode calls"
                            .into(),
                    ));
                }
                Ok(graph)
            })
            .collect()
    }
    pub(super) fn resolve(
        &self,
        inputs: MigrationPlanInputs,
    ) -> Result<ResolvedMigrationPlan, MigrationError> {
        if self.steps.is_empty() || inputs.variables != self.variables {
            return Err(MigrationError::InvalidPlan(
                "missing/changed frozen inputs".into(),
            ));
        }
        let steps = self
            .steps
            .iter()
            .zip(self.definitions()?)
            .map(|(source, graph)| ResolvedMigrationStep {
                version: source.version,
                name: source.name.clone(),
                checksum: content_hash(&source.content),
                graph_hash: graph.hash(),
            })
            .collect();
        ResolvedMigrationPlan::new(inputs, steps).map_err(MigrationError::InvalidPlan)
    }
    pub(super) fn step_scopes(&self) -> Result<Vec<crate::scope::Scope>, MigrationError> {
        self.definitions()?
            .into_iter()
            .map(|graph| {
                let mut scopes = Vec::new();
                for node in graph.nodes {
                    let json = node.config["records_json"].as_str().ok_or_else(|| {
                        MigrationError::InvalidPlan("missing complete records".into())
                    })?;
                    let records: Vec<BTreeMap<String, crate::connection::CypherValue>> =
                        serde_json::from_str(json)
                            .map_err(|e| MigrationError::InvalidPlan(e.to_string()))?;
                    for record in records {
                        let scope = super::physical_index::record_scope(&record)
                            .map_err(|e| MigrationError::InvalidPlan(e.to_string()))?;
                        if !scopes.contains(&scope) {
                            scopes.push(scope);
                        }
                    }
                }
                if scopes.len() != 1 {
                    return Err(MigrationError::InvalidPlan(
                        "each initial migration step must contain exactly one cell".into(),
                    ));
                }
                Ok(scopes.remove(0))
            })
            .collect()
    }
    pub(super) fn variables(&self) -> &BTreeMap<String, String> {
        &self.variables
    }
    pub(super) fn record_calls(
        &self,
    ) -> Result<
        Vec<(
            String,
            Vec<BTreeMap<String, crate::connection::CypherValue>>,
        )>,
        MigrationError,
    > {
        let mut calls = Vec::new();
        for graph in self.definitions()? {
            for node in graph.nodes {
                let entity = node.config["entity"]
                    .as_str()
                    .ok_or_else(|| MigrationError::InvalidPlan("missing entity".into()))?;
                let json = node.config["records_json"].as_str().ok_or_else(|| {
                    MigrationError::InvalidPlan("missing complete records".into())
                })?;
                let records = serde_json::from_str(json)
                    .map_err(|e| MigrationError::InvalidPlan(format!("invalid records: {e}")))?;
                calls.push((entity.into(), records));
            }
        }
        Ok(calls)
    }
}
impl MigrationRunner {
    pub(super) fn freeze_initial_program(
        &self,
        variables: BTreeMap<String, String>,
    ) -> Result<FrozenInitialProgram, MigrationError> {
        let program = FrozenInitialProgram {
            steps: self
                .scan_all()?
                .into_iter()
                .map(|source| SourceStep {
                    version: source.version,
                    name: source.name,
                    content: source.content,
                })
                .collect(),
            variables,
        };
        if program.steps.is_empty() {
            return Err(MigrationError::InvalidPlan("empty initial program".into()));
        }
        for definition in program.definitions()? {
            let graph = DataflowGraph::from_definition(&definition, &self.registry)
                .map_err(MigrationError::InvalidPlan)?;
            graph
                .topological_sort()
                .map_err(MigrationError::InvalidPlan)?;
        }
        Ok(program)
    }

    /// The catalog lock is held only to assemble services, never across runtime
    /// execution: ingestion nodes acquire it to call the existing Catalog API.
    pub(super) fn execute_initial_program(
        &self,
        program: &FrozenInitialProgram,
        catalog: Arc<Mutex<Catalog>>,
        store: &dyn CheckpointStore,
        execution: &str,
        start: usize,
        mut finish_step: impl FnMut(usize, &mut Catalog) -> Result<(), MigrationError>,
    ) -> Result<(), MigrationError> {
        let scopes = program.step_scopes()?;
        if start > scopes.len() {
            return Err(MigrationError::InvalidPlan(
                "invalid initial program prefix".into(),
            ));
        }
        for (index, (step, definition)) in program
            .steps
            .iter()
            .zip(program.definitions()?)
            .enumerate()
            .skip(start)
        {
            {
                let mut cat = catalog
                    .lock()
                    .map_err(|e| MigrationError::DbError(e.to_string()))?;
                cat.set_scope(scopes[index].clone())
                    .map_err(|e| MigrationError::DbError(e.to_string()))?;
                if !cat
                    .begin_lot()
                    .map_err(|e| MigrationError::DbError(e.to_string()))?
                {
                    return Err(MigrationError::InvalidPlan(
                        "qualified initial step requires a native transaction".into(),
                    ));
                }
            }
            let mut graph = DataflowGraph::from_definition(&definition, &self.registry)
                .map_err(MigrationError::InvalidPlan)?;
            let mut services = super::super::services::ServiceRegistry::new();
            let failures = catalog
                .lock()
                .map_err(|e| MigrationError::DbError(e.to_string()))?
                .migration_ingestion_services(&mut services)
                .map_err(|e| MigrationError::DbError(e.to_string()))?;
            services.register("catalog", catalog.clone());
            let runtime = super::super::runtime::DataflowRuntime::with_services(
                graph.nodes.len() + 20,
                services,
            );
            if let Err(detail) = runtime.execute_with_checkpoint(
                &mut graph,
                store,
                &format!("{execution}-{}", step.version),
            ) {
                catalog
                    .lock()
                    .map_err(|e| MigrationError::DbError(e.to_string()))?
                    .rollback_lot();
                return Err(MigrationError::ExecutionError {
                    version: step.version,
                    name: step.name.clone(),
                    detail,
                });
            }
            if !failures
                .lock()
                .map_err(|e| MigrationError::DbError(e.to_string()))?
                .is_empty()
            {
                catalog
                    .lock()
                    .map_err(|e| MigrationError::DbError(e.to_string()))?
                    .rollback_lot();
                return Err(MigrationError::InvalidPlan(
                    "failed ingestion groups in initial program".into(),
                ));
            }
            let mut cat = catalog
                .lock()
                .map_err(|e| MigrationError::DbError(e.to_string()))?;
            finish_step(index + 1, &mut cat)?;
        }
        Ok(())
    }
}

//! Explicitly new, owned native BM25 indexes. No adoption of existing storage.
//! Publication metadata and the qualified migration journal share the data DB.
use super::{
    facts::DeclaredMigrationFacts,
    plan::{MigrationPlanInputs, ResolvedMigrationPlan, StorageDescriptor, StorageGeneration},
    program::FrozenInitialProgram,
    publication::{
        DeclarationSnapshot, NewPublicationOrigin, PublicationError, PublicationIntent,
        PublishedDeclaration, Registry,
    },
    MigrationRunner,
};
use crate::{
    catalog::metadata,
    connection::{CypherValue, DbConnection, QueryParam},
    dialect::{Rag3dbDialect, SchemaDialect},
    Catalog, CatalogConfig, Rag3dbConnection,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
mod snapshot;
use snapshot::PhysicalSnapshot;
type Result<T> = std::result::Result<T, PublicationError>;
pub(crate) const STATE_KEY: &str = "owned_physical_index:v1";
const REGISTRY_KEY: &str = "owned_physical_declarations:v1";
const FORMAT: &str = "owned-initial-bm25:v1";
const DB_FILE: &str = "index.rag3db";
const ROOT_MARKER: &str = ".owned-physical-index-v1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootMarker {
    format: String,
    origin: String,
    database: String,
}
fn read_root_marker(root: &Path) -> Result<RootMarker> {
    if !std::fs::symlink_metadata(root).map_err(storage)?.is_dir() {
        return Err(PublicationError::NotOwned);
    }
    let path = root.join(ROOT_MARKER);
    if !std::fs::symlink_metadata(&path).map_err(storage)?.is_file() {
        return Err(PublicationError::NotOwned);
    }
    let bytes = std::fs::read_to_string(path).map_err(storage)?;
    let marker: RootMarker = serde_json::from_str(&bytes).map_err(storage)?;
    if encode(&marker)? != bytes || marker.format != FORMAT || marker.database != DB_FILE {
        return Err(PublicationError::UnknownContract);
    }
    Ok(marker)
}
fn claim_root(root: &Path, origin: &str) -> Result<()> {
    use std::io::Write;
    let marker = RootMarker {
        format: FORMAT.into(),
        origin: origin.into(),
        database: DB_FILE.into(),
    };
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(root.join(ROOT_MARKER)).map_err(storage)?;
    file.write_all(encode(&marker)?.as_bytes())
        .map_err(storage)?;
    file.sync_all().map_err(storage)?;
    #[cfg(unix)]
    std::fs::File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(storage)?;
    Ok(())
}
/// Native ordinary open checks only presence, so even a partial/malformed claim
/// quarantines a failed bootstrap before constructing Database or replaying WAL.
pub(crate) fn refuse_normal_storage_path(path: &Path) -> Result<()> {
    let mut parents = Vec::new();
    if let Some(parent) = path.parent() {
        parents.push(parent.to_path_buf());
    }
    match std::fs::canonicalize(path) {
        Ok(real) => {
            if let Some(parent) = real.parent() {
                parents.push(parent.to_path_buf());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(storage(error)),
    }
    for parent in parents {
        match std::fs::symlink_metadata(parent.join(ROOT_MARKER)) {
            Ok(_)=>return Err(invalid("owned physical storage requires qualified root access, including incomplete bootstrap")),
            Err(error) if error.kind()==std::io::ErrorKind::NotFound=>{},
            Err(error)=>return Err(storage(error)),
        }
    }
    Ok(())
}
/// Opaque access to one exact claimed database, never a boolean bypass.
pub(crate) struct PhysicalRootPermit {
    root: PathBuf,
    origin: String,
}
impl PhysicalRootPermit {
    pub(crate) fn validate_path(&self, path: &Path) -> Result<()> {
        if path != self.root.join(DB_FILE) || read_root_marker(&self.root)?.origin != self.origin {
            return Err(PublicationError::NotOwned);
        }
        Ok(())
    }
}

fn storage(e: impl std::fmt::Display) -> PublicationError {
    PublicationError::Storage(e.to_string())
}
fn invalid(e: impl Into<String>) -> PublicationError {
    PublicationError::Corrupt(e.into())
}
fn encode<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_value(value)
        .map(|value| value.to_string())
        .map_err(storage)
}
fn digest<T: Serialize>(domain: &str, value: &T) -> Result<String> {
    Ok(crate::hash::content_hash(&encode(&(domain, value))?))
}
fn read_metadata(conn: &dyn DbConnection) -> Result<BTreeMap<String, String>> {
    let rows = conn
        .execute_with_params(
            &Rag3dbDialect.load_meta_by_prefix("prefix"),
            &[QueryParam::new(
                "prefix",
                CypherValue::String(String::new()),
            )],
        )
        .map_err(storage)?;
    metadata::decode_rows(rows).map_err(invalid)
}
fn read_registry(conn: &dyn DbConnection) -> Result<Registry> {
    Registry::decode(
        read_metadata(conn)?
            .get(REGISTRY_KEY)
            .ok_or(PublicationError::UnknownContract)?,
    )
}
fn write_meta<T: Serialize>(conn: &dyn DbConnection, key: &str, value: &T) -> Result<()> {
    metadata::write_value(conn, &Rag3dbDialect, key, &encode(value)?).map_err(storage)
}
fn connection(root: &Path, readonly: bool) -> Result<Rag3dbConnection> {
    let marker = read_root_marker(root)?;
    let permit = PhysicalRootPermit {
        root: root.into(),
        origin: marker.origin,
    };
    Rag3dbConnection::with_owned_physical_config(
        root.join(DB_FILE),
        rag3db::SystemConfig::default()
            .buffer_pool_size(64 * 1024 * 1024)
            .read_only(readonly),
        &permit,
    )
    .map_err(storage)
}
fn existing(root: &Path) -> Result<()> {
    if !root.is_absolute()
        || !std::fs::symlink_metadata(root).map_err(storage)?.is_dir()
        || !std::fs::symlink_metadata(root.join(DB_FILE))
            .map_err(storage)?
            .is_file()
    {
        return Err(PublicationError::NotOwned);
    }
    Ok(())
}
fn transaction<T>(conn: &dyn DbConnection, action: impl FnOnce() -> Result<T>) -> Result<T> {
    if conn.must_reopen().is_some() {
        return Err(PublicationError::NeedsReopen);
    }
    conn.begin().map_err(storage)?;
    let value = match action() {
        Ok(value) => value,
        Err(error) => {
            return match conn.rollback() {
                Ok(()) => Err(error),
                Err(rollback) => {
                    conn.poison("physical publication rollback failed");
                    conn.close_without_checkpoint();
                    Err(storage(format!("{error}; rollback: {rollback}")))
                }
            }
        }
    };
    if let Err(error) = conn.commit() {
        conn.poison("physical publication commit outcome unknown");
        conn.close_without_checkpoint();
        return Err(storage(format!(
            "commit outcome unknown; reopen/resolve: {error}; rollback: {:?}",
            conn.rollback()
        )));
    }
    Ok(value)
}

/// Complete physical values are sealed by this adapter, never supplied as hashes.
#[derive(Debug, Clone, Serialize)]
pub struct QualifiedPhysicalState {
    origin: String,
    publication: String,
    descriptor: StorageDescriptor,
    snapshot: PhysicalSnapshot,
}
impl QualifiedPhysicalState {
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn publication(&self) -> &str {
        &self.publication
    }
    pub fn descriptor(&self) -> &StorageDescriptor {
        &self.descriptor
    }
    pub fn logical_fingerprint(&self) -> Result<String> {
        self.snapshot.logical_fingerprint()
    }
    pub fn materialization_fingerprint(&self) -> Result<String> {
        self.snapshot.materialization_fingerprint()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredHead {
    format: String,
    origin: String,
    publication: String,
    descriptor: StorageDescriptor,
    snapshot: PhysicalSnapshot,
    cells: Vec<crate::scope::Scope>,
}
impl StoredHead {
    fn state(&self) -> QualifiedPhysicalState {
        QualifiedPhysicalState {
            origin: self.origin.clone(),
            publication: self.publication.clone(),
            descriptor: self.descriptor.clone(),
            snapshot: self.snapshot.clone(),
        }
    }
}
fn read_head(conn: &dyn DbConnection, origin: &str) -> Result<StoredHead> {
    let meta = read_metadata(conn)?;
    let encoded = meta
        .get(STATE_KEY)
        .ok_or(PublicationError::UnknownContract)?;
    let head: StoredHead = serde_json::from_str(encoded).map_err(storage)?;
    if encode(&head)? != *encoded || head.format != FORMAT {
        return Err(PublicationError::UnknownContract);
    }
    if head.origin != origin {
        return Err(PublicationError::OriginMismatch);
    }
    let registry = read_registry(conn)?;
    if registry.origin() != origin || registry.head() != head.publication {
        return Err(invalid("physical/declaration head mismatch"));
    }
    if head.descriptor.engine_format != rag3db::get_storage_version().to_string()
        || head.descriptor.internal_schema != crate::scope::SCHEMA_VERSION
        || head.descriptor.application_format != FORMAT
        || head.descriptor.generation
            != StorageGeneration::Established(format!("{}:{}", head.origin, head.publication))
    {
        return Err(PublicationError::UnknownContract);
    }
    if descriptor(&registry.published(registry.head())?) != head.descriptor {
        return Err(invalid("descriptor does not match complete declarations"));
    }
    if head.cells.is_empty() {
        return Err(invalid("missing owned cells"));
    }
    for (index, cell) in head.cells.iter().enumerate() {
        cell.validate().map_err(invalid)?;
        if head.cells[..index].contains(cell) {
            return Err(invalid("duplicate owned cells"));
        }
    }
    Ok(head)
}

/// A frozen actual source, declaration intent, complete source support and plan.
#[derive(Debug, Clone, Serialize)]
pub struct QualifiedIndexPlan {
    stored: StoredInitialPlan,
    resolved: ResolvedMigrationPlan,
}
impl QualifiedIndexPlan {
    pub fn resolved(&self) -> &ResolvedMigrationPlan {
        &self.resolved
    }
    pub fn identity(&self) -> &str {
        &self.stored.identity
    }
    pub fn operation(&self) -> &str {
        self.stored.intent.operation()
    }
    pub fn target_publication(&self) -> &str {
        self.stored.intent.target_publication()
    }
}
#[derive(Debug, Clone, Serialize)]
struct StoredInitialPlan {
    protocol: u32,
    origin: String,
    source: StoredHead,
    #[serde(serialize_with = "serialize_intent")]
    intent: PublicationIntent,
    program: FrozenInitialProgram,
    inputs: MigrationPlanInputs,
    identity: String,
    probes: Vec<LexicalProbe>,
}
fn serialize_intent<S: serde::Serializer>(
    intent: &PublicationIntent,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_str(&intent.encode().map_err(serde::ser::Error::custom)?)
}
// Intent and resolved plan restoration use their validating constructors, rather
// than deriving Deserialize for an execution capability.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanWire {
    protocol: u32,
    origin: String,
    source: StoredHead,
    intent: String,
    program: FrozenInitialProgram,
    inputs: MigrationPlanInputs,
    identity: String,
    probes: Vec<LexicalProbe>,
}
impl StoredInitialPlan {
    fn calculate_identity(&self) -> Result<String> {
        digest(
            "qualified-initial-plan:v1",
            &(
                self.protocol,
                &self.origin,
                &self.source,
                self.intent.encode()?,
                &self.program,
                &self.inputs,
                &self.probes,
            ),
        )
    }
    fn restore(bytes: &str) -> Result<QualifiedIndexPlan> {
        let wire: PlanWire = serde_json::from_str(bytes).map_err(storage)?;
        let intent = PublicationIntent::decode(&wire.intent)?;
        let stored = Self {
            protocol: wire.protocol,
            origin: wire.origin,
            source: wire.source,
            intent,
            program: wire.program,
            inputs: wire.inputs,
            identity: wire.identity,
            probes: wire.probes,
        };
        if stored.protocol != 1 || encode(&stored)? != bytes {
            return Err(PublicationError::UnknownContract);
        }
        let resolved = stored
            .program
            .resolve(stored.inputs.clone())
            .map_err(storage)?;
        if stored.calculate_identity()? != stored.identity {
            return Err(invalid("resolved initial plan identity mismatch"));
        }
        resolved
            .validate_generation_preconditions()
            .map_err(invalid)?;
        Ok(QualifiedIndexPlan { stored, resolved })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PhysicalPublicationReceipt {
    declaration: PublishedDeclaration,
    state: QualifiedPhysicalState,
    plan_identity: String,
}
impl PhysicalPublicationReceipt {
    pub fn declaration(&self) -> &PublishedDeclaration {
        &self.declaration
    }
    pub fn state(&self) -> &QualifiedPhysicalState {
        &self.state
    }
    pub fn plan_identity(&self) -> &str {
        &self.plan_identity
    }
}

// Exactly one qualified row is possible in this initial-only profile. The old
// columns keep their meaning; versioned qualified_input owns all recovery bytes.
fn initialize_journal(catalog: &Catalog) -> Result<()> {
    catalog.migration_initialize().map_err(storage)?;
    for column in ["qualified_input", "qualified_result"] {
        catalog
            .conn()
            .execute(&format!(
                "ALTER TABLE _DataflowMigration ADD {column} STRING DEFAULT NULL"
            ))
            .map_err(storage)?;
    }
    Ok(())
}
struct InitialJournal {
    status: String,
    plan: QualifiedIndexPlan,
    committed: Option<CommittedPrefix>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommittedPrefix {
    next_step: usize,
    snapshot: PhysicalSnapshot,
}
fn journal(conn: &dyn DbConnection) -> Result<Option<InitialJournal>> {
    let result = conn
        .execute(
            "MATCH (n:_DataflowMigration) RETURN n.status, n.qualified_input, n.qualified_result",
        )
        .map_err(storage)?;
    if result.rows.len() > 1 {
        return Err(invalid("initial journal contains multiple operations"));
    }
    let Some(row) = result.rows.first() else {
        return Ok(None);
    };
    if row.len() != 3 {
        return Err(invalid("malformed qualified journal row"));
    }
    let status = row[0]
        .as_str()
        .filter(|s| matches!(*s, "pending" | "applied"))
        .ok_or_else(|| invalid("unrecognized initial journal status"))?;
    let plan = StoredInitialPlan::restore(
        row[1]
            .as_str()
            .ok_or_else(|| invalid("legacy/unqualified migration journal"))?,
    )?;
    let committed = match &row[2] {
        CypherValue::Null => None,
        CypherValue::String(bytes) => {
            let snapshot: CommittedPrefix = serde_json::from_str(bytes).map_err(storage)?;
            if encode(&snapshot)? != *bytes {
                return Err(invalid("noncanonical qualified result"));
            }
            Some(snapshot)
        }
        _ => return Err(invalid("malformed qualified result")),
    };
    if status == "applied" && committed.is_none() {
        return Err(invalid("applied journal without committed values"));
    }
    let expected = journal_row(&plan, status, committed.as_ref())?;
    let fields = JOURNAL_FIELDS
        .iter()
        .map(|field| format!("n.{field}"))
        .collect::<Vec<_>>()
        .join(", ");
    let full = conn
        .execute(&format!("MATCH (n:_DataflowMigration) RETURN {fields}"))
        .map_err(storage)?;
    let values = JOURNAL_FIELDS
        .iter()
        .map(|field| expected[*field].clone())
        .collect::<Vec<_>>();
    if full.rows != vec![values] {
        return Err(invalid(
            "qualified journal fields do not match the fixed operation",
        ));
    }
    Ok(Some(InitialJournal {
        status: status.into(),
        plan,
        committed,
    }))
}
const JOURNAL_FIELDS: [&str; 12] = [
    "_uuid",
    "version",
    "name",
    "status",
    "direction",
    "checksum",
    "execution_id",
    "applied_at",
    "duration_ms",
    "error",
    "qualified_input",
    "qualified_result",
];
fn journal_row(
    plan: &QualifiedIndexPlan,
    status: &str,
    committed: Option<&CommittedPrefix>,
) -> Result<BTreeMap<String, CypherValue>> {
    Ok(BTreeMap::from([
        ("_uuid".into(), CypherValue::String("owned-initial".into())),
        ("version".into(), CypherValue::Int(1)),
        (
            "name".into(),
            CypherValue::String("owned-initial-bm25".into()),
        ),
        ("status".into(), CypherValue::String(status.into())),
        ("direction".into(), CypherValue::String("up".into())),
        (
            "checksum".into(),
            CypherValue::String(plan.identity().into()),
        ),
        (
            "execution_id".into(),
            CypherValue::String(format!("migration-owned-{}", plan.identity())),
        ),
        ("applied_at".into(), CypherValue::Int(0)),
        ("duration_ms".into(), CypherValue::Int(0)),
        ("error".into(), CypherValue::String(String::new())),
        (
            "qualified_input".into(),
            CypherValue::String(encode(&plan.stored)?),
        ),
        (
            "qualified_result".into(),
            match committed {
                Some(snapshot) => CypherValue::String(encode(snapshot)?),
                None => CypherValue::Null,
            },
        ),
    ]))
}
fn write_journal(
    conn: &dyn DbConnection,
    plan: &QualifiedIndexPlan,
    status: &str,
    committed: Option<&CommittedPrefix>,
) -> Result<()> {
    let row = journal_row(plan, status, committed)?;
    conn.execute_with_params(
        &Rag3dbDialect.batch_upsert("_DataflowMigration", &JOURNAL_FIELDS),
        &[QueryParam::new(
            "items",
            CypherValue::List(vec![CypherValue::Map(row)]),
        )],
    )
    .map_err(storage)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LexicalProbe {
    pub scope: crate::scope::Scope,
    pub target: String,
    pub query: String,
    pub expected_occurrences: Vec<String>,
}
fn validate_profile(snapshot: &DeclarationSnapshot) -> Result<()> {
    let input = snapshot.complete_input();
    if !matches!(input.provider, super::facts::ProviderFacts::Absent)
        || !input
            .recipe
            .model_sources
            .as_ref()
            .ok_or(PublicationError::UnknownContract)?
            .is_empty()
        || !input
            .recipe
            .dense_models
            .as_ref()
            .ok_or(PublicationError::UnknownContract)?
            .is_empty()
        || !input
            .recipe
            .resolved_templates
            .as_ref()
            .ok_or(PublicationError::UnknownContract)?
            .is_empty()
        || !input.index_signals.is_empty()
    {
        return Err(invalid(
            "initial lexical profile has no provider, model, external template or signal override",
        ));
    }
    if !snapshot.supplied_facts().relations().is_empty() {
        return Err(invalid(
            "initial profile accepts generated derivation relations only",
        ));
    }
    for (name, config) in snapshot.supplied_facts().entities() {
        crate::schema::validate_identifier(name, "owned entity").map_err(storage)?;
        if name.starts_with('_')
            || config.signals != crate::search::SearchSignals::BM25
            || !config.has_simple_pipeline()
            || config.has_kb_participation()
            || config.snapshot.is_some()
            || config.lifecycle.is_some()
        {
            return Err(invalid(
                "initial lexical profile requires simple BM25 entities without lifecycle/snapshot",
            ));
        }
    }
    Ok(())
}
fn validate_program(
    program: &FrozenInitialProgram,
    declarations: &DeclarationSnapshot,
    probes: &[LexicalProbe],
) -> Result<()> {
    let calls = program.record_calls().map_err(storage)?;
    if calls.is_empty() || probes.is_empty() {
        return Err(invalid(
            "initial program and lexical probes must be complete/nonempty",
        ));
    }
    let entities = declarations.supplied_facts().entities();
    for (entity, records) in calls {
        let config = entities
            .get(&entity)
            .ok_or_else(|| invalid("unknown initial input entity"))?;
        if config.derived.is_some() || records.is_empty() {
            return Err(invalid(
                "initial input must contain nonempty source records",
            ));
        }
        for record in records {
            for (name, value) in &record {
                if crate::scope::is_scope_column(name) {
                    continue;
                }
                let field = config
                    .fields
                    .get(name)
                    .ok_or_else(|| invalid(format!("undeclared initial field {entity}.{name}")))?;
                let json = serde_json::to_value(value).map_err(storage)?;
                rag3weaver_ir::normalize(&field.field_type, &json).map_err(invalid)?;
            }
            if config.fields.keys().any(|name| !record.contains_key(name)) {
                return Err(invalid("initial records require every declared field"));
            }
            let scope = record_scope(&record)?;
            if !probes.iter().any(|p| {
                p.scope == scope && p.target == entity && !p.expected_occurrences.is_empty()
            }) {
                return Err(invalid("missing lexical query for input cell/source"));
            }
        }
    }
    for probe in probes {
        probe.scope.validate().map_err(invalid)?;
        if !entities.contains_key(&probe.target) || probe.query.trim().is_empty() {
            return Err(invalid("invalid lexical probe"));
        }
        let mut expected = probe.expected_occurrences.clone();
        expected.sort();
        expected.dedup();
        if expected.len() != probe.expected_occurrences.len() {
            return Err(invalid("duplicate expected occurrence"));
        }
    }
    Ok(())
}
pub(super) fn record_scope(record: &BTreeMap<String, CypherValue>) -> Result<crate::scope::Scope> {
    let org = record
        .get(crate::scope::ORG_COLUMN)
        .and_then(CypherValue::as_str)
        .ok_or_else(|| invalid("initial input requires explicit org"))?;
    let project = record
        .get(crate::scope::PROJECT_COLUMN)
        .and_then(CypherValue::as_str)
        .ok_or_else(|| invalid("initial input requires explicit project"))?;
    let scope = crate::scope::Scope::new(org, project);
    scope.validate().map_err(invalid)?;
    Ok(scope)
}
fn descriptor(declaration: &PublishedDeclaration) -> StorageDescriptor {
    StorageDescriptor {
        engine_format: rag3db::get_storage_version().to_string(),
        internal_schema: crate::scope::SCHEMA_VERSION.into(),
        application_format: FORMAT.into(),
        recipe: declaration.snapshot().complete_input().recipe.id.clone(),
        config_fingerprint: declaration
            .snapshot()
            .supplied_facts()
            .config_fingerprint()
            .into(),
        generation: StorageGeneration::Established(format!(
            "{}:{}",
            declaration.origin(),
            declaration.publication()
        )),
    }
}
fn owned_catalog(
    root: &Path,
    conn: Rag3dbConnection,
    access: &PhysicalAccess,
) -> Result<Arc<Mutex<Catalog>>> {
    let mut catalog = Catalog::without_embedding_model(
        Box::new(conn),
        CatalogConfig {
            checkpoint_dir: Some(root.join("checkpoints")),
            ..Default::default()
        },
    );
    catalog.set_moteur_texte(crate::search_backend::MoteurTexte::Lucivy);
    catalog.set_fts_storage(crate::fts_handle::FtsStorage::BlobBacked { lazy: false });
    catalog.initialize_owned_physical(access).map_err(storage)?;
    Ok(Arc::new(Mutex::new(catalog)))
}

/// No normal Catalog/host/CLI open may silently serve or modify this closed
/// profile. The owned adapter mints its private access only after RO validation.
pub(crate) fn refuse_normal_catalog_open(conn: &dyn DbConnection) -> Result<()> {
    if let Some(path) = conn.database_path() {
        refuse_normal_storage_path(&path)?;
    }
    let tables = conn
        .execute("CALL show_tables() RETURN *")
        .map_err(storage)?;
    if tables
        .rows
        .iter()
        .any(|r| r.get(1).and_then(CypherValue::as_str) == Some("_catalog_meta"))
        && read_metadata(conn)?.contains_key(STATE_KEY)
    {
        return Err(invalid(
            "owned physical index requires qualified adapter; normal Catalog open refused",
        ));
    }
    Ok(())
}
#[derive(Clone, Copy)]
enum PhysicalAccessKind {
    EmptyBootstrap,
    ExistingProfile,
}
pub(crate) struct PhysicalAccess {
    origin: String,
    readonly: bool,
    kind: PhysicalAccessKind,
}
impl PhysicalAccess {
    pub(crate) fn validate(&self, conn: &dyn DbConnection) -> Result<()> {
        let path = conn.database_path().ok_or(PublicationError::NotOwned)?;
        let root = path.parent().ok_or(PublicationError::NotOwned)?;
        if read_root_marker(root)?.origin != self.origin {
            return Err(PublicationError::OriginMismatch);
        }
        match self.kind {
            PhysicalAccessKind::ExistingProfile => read_head(conn, &self.origin).map(|_| ()),
            PhysicalAccessKind::EmptyBootstrap => {
                if self.readonly
                    || !conn
                        .execute("CALL show_tables() RETURN *")
                        .map_err(storage)?
                        .rows
                        .is_empty()
                {
                    return Err(invalid(
                        "bootstrap capability requires the actual empty newly claimed database",
                    ));
                }
                Ok(())
            }
        }
    }
    pub(crate) fn readonly(&self) -> bool {
        self.readonly
    }
}

pub struct OwnedPhysicalIndex {
    root: PathBuf,
    origin: String,
    catalog: Option<Arc<Mutex<Catalog>>>,
    poisoned: bool,
}
impl OwnedPhysicalIndex {
    pub fn create_new(
        root: impl AsRef<Path>,
        origin: NewPublicationOrigin,
        schema: DeclarationSnapshot,
        cells: Vec<crate::scope::Scope>,
    ) -> Result<Self> {
        validate_profile(&schema)?;
        if cells.is_empty() {
            return Err(invalid("new lexical profile requires explicit cells"));
        }
        for (i, cell) in cells.iter().enumerate() {
            cell.validate().map_err(invalid)?;
            if cells[..i].contains(cell) {
                return Err(invalid("duplicate owned cell"));
            }
        }
        let root = root.as_ref();
        if !root.is_absolute() {
            return Err(invalid("owned root must be absolute"));
        }
        let registry = Registry::genesis(origin, "create-owned-index", schema.clone())?;
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(root).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                PublicationError::ExistingPath
            } else {
                storage(e)
            }
        })?;
        claim_root(root, registry.origin())?;
        physical_boundary("after-root-claim", None);
        let access = PhysicalAccess {
            origin: registry.origin().into(),
            readonly: false,
            kind: PhysicalAccessKind::EmptyBootstrap,
        };
        let catalog = owned_catalog(root, connection(root, false)?, &access)?;
        physical_boundary("after-bootstrap-initialize", None);
        let mut cat = catalog.lock().map_err(storage)?;
        let facts = schema.supplied_facts();
        let mut pending = facts.entities().clone();
        while !pending.is_empty() {
            let ready = pending
                .iter()
                .filter(|(_, config)| {
                    config
                        .derived
                        .as_ref()
                        .is_none_or(|d| cat.entity_configs().contains_key(&d.from))
                })
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>();
            if ready.is_empty() {
                return Err(invalid("cyclic/missing derivation source"));
            }
            for name in ready {
                let config = pending.remove(&name).expect("selected pending");
                cat.register_entity(&name, config).map_err(storage)?;
            }
        }
        initialize_journal(&cat)?;
        for cell in &cells {
            cat.set_scope(cell.clone()).map_err(storage)?;
            cat.prepare_schema_for_ingest().map_err(storage)?;
        }
        let declaration = registry.published(registry.head())?;
        let head = StoredHead {
            format: FORMAT.into(),
            origin: registry.origin().into(),
            publication: registry.head().into(),
            descriptor: descriptor(&declaration),
            snapshot: PhysicalSnapshot::read(cat.conn())?,
            cells,
        };
        for (name, _) in facts.entities() {
            if head
                .snapshot
                .logical_rows
                .get(name)
                .is_none_or(|rows| !rows.is_empty())
            {
                return Err(invalid("new index contains application rows"));
            }
        }
        transaction(cat.conn(), || {
            write_meta(cat.conn(), REGISTRY_KEY, &registry)?;
            write_meta(cat.conn(), STATE_KEY, &head)
        })?;
        let origin = head.origin;
        drop(cat);
        Ok(Self {
            root: root.into(),
            origin,
            catalog: Some(catalog),
            poisoned: false,
        })
    }
    fn catalog(&self) -> Result<Arc<Mutex<Catalog>>> {
        if self.poisoned {
            return Err(PublicationError::NeedsReopen);
        }
        self.catalog.clone().ok_or(PublicationError::NeedsReopen)
    }
    pub fn observe_published(&self) -> Result<QualifiedPhysicalState> {
        let catalog = self.catalog()?;
        let cat = catalog.lock().map_err(storage)?;
        let head = read_head(cat.conn(), &self.origin)?;
        validate_published(cat.conn(), &head)?;
        if PhysicalSnapshot::read(cat.conn())? != head.snapshot {
            return Err(invalid("published physical state changed"));
        }
        Ok(head.state())
    }
    pub fn prepare_initial_batch(
        &self,
        runner: &MigrationRunner,
        operation: &str,
        variables: BTreeMap<String, String>,
        probes: Vec<LexicalProbe>,
    ) -> Result<QualifiedIndexPlan> {
        let source = self.observe_published()?;
        let catalog = self.catalog()?;
        let cat = catalog.lock().map_err(storage)?;
        if journal(cat.conn())?.is_some() {
            return Err(invalid(
                "initial batch already planned/published; resolve existing operation",
            ));
        }
        let registry = read_registry(cat.conn())?;
        let declaration = registry.published(registry.head())?;
        let intent = registry.prepare(operation, declaration.snapshot().clone())?;
        let program = runner.freeze_initial_program(variables).map_err(storage)?;
        validate_program(&program, declaration.snapshot(), &probes)?;
        let source_head = read_head(cat.conn(), &self.origin)?;
        if program
            .step_scopes()
            .map_err(storage)?
            .iter()
            .any(|cell| !source_head.cells.contains(cell))
        {
            return Err(invalid("input references an unowned cell"));
        }
        let mut target = source.descriptor.clone();
        target.generation = StorageGeneration::Established(format!(
            "{}:{}",
            source.origin,
            intent.target_publication()
        ));
        let facts = DeclaredMigrationFacts::new(
            declaration.snapshot().supplied_facts().clone(),
            declaration.snapshot().supplied_facts().clone(),
        )
        .map_err(storage)?;
        let inputs = MigrationPlanInputs {
            source: source.descriptor,
            target,
            variables: program.variables().clone(),
            schema_facts_fingerprint: facts.schema_facts_fingerprint().into(),
        };
        let resolved = program.resolve(inputs.clone()).map_err(storage)?;
        resolved
            .validate_generation_preconditions()
            .map_err(invalid)?;
        let mut stored = StoredInitialPlan {
            protocol: 1,
            origin: self.origin.clone(),
            source: read_head(cat.conn(), &self.origin)?,
            intent,
            program,
            inputs,
            identity: String::new(),
            probes,
        };
        stored.identity = stored.calculate_identity()?;
        Ok(QualifiedIndexPlan { stored, resolved })
    }
}

fn physical_boundary(_stage: &str, _step: Option<usize>) {
    #[cfg(test)]
    {
        let wanted = std::env::var("RAG3WEAVER_PHYSICAL_CRASH").ok();
        if wanted.as_deref() == Some(_stage)
            || _step
                .is_some_and(|step| wanted.as_deref() == Some(format!("{_stage}:{step}").as_str()))
        {
            std::process::exit(91);
        }
    }
}
fn same_committed_values(expected: &PhysicalSnapshot, actual: &PhysicalSnapshot) -> Result<()> {
    if expected.schema != actual.schema
        || expected.logical_rows != actual.logical_rows
        || expected.metadata != actual.metadata
        || expected.execution_rows != actual.execution_rows
    {
        return Err(invalid(
            "committed application/schema/checkpoint values changed before publication",
        ));
    }
    Ok(())
}
fn verify_probes(catalog: &Arc<Mutex<Catalog>>, probes: &[LexicalProbe]) -> Result<()> {
    use crate::search::{Consistency, ResultMode, SearchOptions, SearchSignals};
    for probe in probes {
        let response = Catalog::rechercher(
            catalog,
            &probe.target,
            &probe.query,
            SearchOptions {
                signals: Some(SearchSignals::BM25),
                strict_signals: true,
                consistency: Consistency::Immediate,
                scope: Some(probe.scope.clone()),
                result_mode: ResultMode::SourceResolved,
                limit: 10000,
                fuzzy_distance: 0,
                ..Default::default()
            },
        )
        .map_err(storage)?;
        let mut got = response
            .results
            .into_iter()
            .map(|result| result.uuid)
            .collect::<Vec<_>>();
        got.sort();
        let mut expected = probe.expected_occurrences.clone();
        expected.sort();
        if response.meta.partial || got != expected {
            return Err(invalid(format!(
                "lexical closure failed for {}/{} {}: expected {expected:?}, got {got:?}",
                probe.scope.org, probe.scope.project, probe.query
            )));
        }
    }
    Ok(())
}
fn validate_pending(
    conn: &dyn DbConnection,
    head: &StoredHead,
    entry: &InitialJournal,
) -> Result<()> {
    if entry.plan.stored.origin != head.origin
        || entry.plan.stored.source.publication != head.publication
        || entry.plan.stored.inputs.source != head.descriptor
        || encode(&entry.plan.stored.source)? != encode(head)?
    {
        return Err(invalid("candidate plan no longer matches its fixed source"));
    }
    let registry = read_registry(conn)?;
    let declared = registry.published(registry.head())?;
    validate_profile(declared.snapshot())?;
    validate_program(
        &entry.plan.stored.program,
        declared.snapshot(),
        &entry.plan.stored.probes,
    )?;
    if entry
        .plan
        .stored
        .program
        .step_scopes()
        .map_err(storage)?
        .iter()
        .any(|cell| !head.cells.contains(cell))
    {
        return Err(invalid("pending program references an unowned cell"));
    }
    let mut next = registry;
    next.apply(&entry.plan.stored.intent)?;
    let actual = PhysicalSnapshot::read(conn)?;
    if let Some(expected) = &entry.committed {
        let steps = entry.plan.stored.program.step_scopes().map_err(storage)?;
        if expected.next_step == 0 || expected.next_step > steps.len() {
            return Err(invalid("invalid committed program prefix"));
        }
        same_committed_values(&expected.snapshot, &actual)
    } else if actual != head.snapshot {
        Err(invalid(
            "uncommitted initial plan has changed source values",
        ))
    } else {
        Ok(())
    }
}
fn validate_published(
    conn: &dyn DbConnection,
    head: &StoredHead,
) -> Result<Option<InitialJournal>> {
    let registry = read_registry(conn)?;
    let declaration = registry.published(registry.head())?;
    let Some(entry) = journal(conn)? else {
        if !registry.is_unique_genesis_head() || declaration.operation() != "create-owned-index" {
            return Err(invalid(
                "published target lost its qualified migration journal",
            ));
        }
        return Ok(None);
    };
    if entry.status != "applied" {
        return Err(invalid(
            "candidate execution requires explicit same-plan recovery",
        ));
    }
    let plan = &entry.plan;
    if plan.stored.origin != head.origin
        || plan.stored.source.origin != head.origin
        || plan.target_publication() != head.publication
        || plan.stored.inputs.target != head.descriptor
        || plan.stored.inputs.source != plan.stored.source.descriptor
        || plan.stored.source.cells != head.cells
        || declaration.intent_identity() != plan.stored.intent.identity()
        || declaration.operation() != plan.operation()
    {
        return Err(invalid(
            "applied initial plan does not match the publication head",
        ));
    }
    let source = registry.published(&plan.stored.source.publication)?;
    if !registry.is_genesis_publication(source.publication())
        || source.operation() != "create-owned-index"
        || descriptor(&source) != plan.stored.source.descriptor
    {
        return Err(invalid(
            "applied initial plan lost its qualified creation source",
        ));
    }
    let committed = entry
        .committed
        .as_ref()
        .ok_or_else(|| invalid("missing applied physical result"))?;
    let count = plan.stored.program.step_scopes().map_err(storage)?.len();
    if committed.next_step != count {
        return Err(invalid("applied initial program has an incomplete prefix"));
    }
    same_committed_values(&committed.snapshot, &head.snapshot)?;
    Ok(Some(entry))
}

impl OwnedPhysicalIndex {
    fn close_catalog(&mut self) -> Result<()> {
        if let Some(catalog) = self.catalog.take() {
            let mutex = Arc::try_unwrap(catalog)
                .map_err(|_| invalid("owned catalog still shared at physical closure"))?;
            let mut cat = mutex.into_inner().map_err(storage)?;
            cat.shutdown().map_err(storage)?;
            drop(cat);
        }
        Ok(())
    }
    pub fn open_owned(root: impl AsRef<Path>, expected_origin: &str) -> Result<Self> {
        let root = root.as_ref();
        existing(root)?;
        {
            let reader = connection(root, true)?;
            let head = read_head(&reader, expected_origin)?;
            validate_published(&reader, &head)?;
            if PhysicalSnapshot::read(&reader)? != head.snapshot {
                return Err(invalid("published state/schema/materialization changed"));
            }
        }
        // Exclusive native ownership prevents another writer while this handle
        // lives. Recheck on the actual connection before constructing services.
        let conn = connection(root, true)?;
        let head = read_head(&conn, expected_origin)?;
        let entry = validate_published(&conn, &head)?;
        if PhysicalSnapshot::read(&conn)? != head.snapshot {
            return Err(invalid("state changed while opening owned reader"));
        }
        let access = PhysicalAccess {
            origin: expected_origin.into(),
            readonly: true,
            kind: PhysicalAccessKind::ExistingProfile,
        };
        let catalog = owned_catalog(root, conn, &access)?;
        if let Some(entry) = entry {
            verify_probes(&catalog, &entry.plan.stored.probes)?;
        }
        Ok(Self {
            root: root.into(),
            origin: expected_origin.into(),
            catalog: Some(catalog),
            poisoned: false,
        })
    }
    pub fn execute_initial_batch(
        &mut self,
        runner: &MigrationRunner,
        plan: &QualifiedIndexPlan,
    ) -> Result<PhysicalPublicationReceipt> {
        let result = self.execute_initial_batch_inner(runner, plan);
        if result.is_err() {
            self.poisoned = true;
            if let Some(catalog) = &self.catalog {
                if let Ok(mut cat) = catalog.lock() {
                    cat.invalidate_publication(
                        "physical operation failed; explicit reopen/recovery required",
                    );
                }
            }
        }
        result
    }
    fn execute_initial_batch_inner(
        &mut self,
        runner: &MigrationRunner,
        plan: &QualifiedIndexPlan,
    ) -> Result<PhysicalPublicationReceipt> {
        let catalog = self.catalog()?;
        let conn = catalog.lock().map_err(storage)?.conn_arc();
        let head = read_head(conn.as_ref(), &self.origin)?;
        if plan.stored.origin != self.origin {
            return Err(PublicationError::OriginMismatch);
        }
        if let Some(entry) = journal(conn.as_ref())? {
            if encode(&entry.plan.stored)? != encode(&plan.stored)? {
                return Err(PublicationError::OperationConflict);
            }
            if entry.status == "applied" {
                return self.resolve_operation(plan.operation());
            }
            validate_pending(conn.as_ref(), &head, &entry)?;
        } else {
            if PhysicalSnapshot::read(conn.as_ref())? != plan.stored.source.snapshot
                || encode(&head)? != encode(&plan.stored.source)?
            {
                return Err(PublicationError::PublicationMoved);
            }
            let registry = read_registry(conn.as_ref())?;
            let declared = registry.published(registry.head())?;
            validate_profile(declared.snapshot())?;
            validate_program(
                &plan.stored.program,
                declared.snapshot(),
                &plan.stored.probes,
            )?;
            let mut next = registry;
            next.apply(&plan.stored.intent)?;
            transaction(conn.as_ref(), || {
                write_journal(conn.as_ref(), plan, "pending", None)
            })?;
        }
        let entry =
            journal(conn.as_ref())?.ok_or_else(|| invalid("missing durable initial intent"))?;
        if let Some(prefix) = &entry.committed {
            // Validate and rebuild the already committed prefix. No source write,
            // derivation, chunking or model is replayed for these steps.
            let scopes = plan.stored.program.step_scopes().map_err(storage)?;
            let mut cat = catalog.lock().map_err(storage)?;
            for scope in &scopes[..prefix.next_step] {
                cat.rebuild_lexical_cell_from_rows(scope).map_err(storage)?;
            }
        }
        let checkpoint = crate::dataflow::checkpoint_store::CypherCheckpointStore::with_directory(
            conn.clone(),
            self.root.join("checkpoints"),
        );
        let start = entry
            .committed
            .as_ref()
            .map_or(0, |prefix| prefix.next_step);
        runner
            .execute_initial_program(
                &plan.stored.program,
                catalog.clone(),
                &checkpoint,
                &format!("migration-owned-{}", plan.identity()),
                start,
                |next_step, cat| {
                    let committed = CommittedPrefix {
                        next_step,
                        snapshot: PhysicalSnapshot::read(cat.conn())
                            .map_err(|e| super::MigrationError::DbError(e.to_string()))?,
                    };
                    write_journal(cat.conn(), plan, "pending", Some(&committed))
                        .map_err(|e| super::MigrationError::DbError(e.to_string()))?;
                    physical_boundary("before-logical-commit", Some(next_step));
                    cat.commit_lot_after_native(|| {
                        physical_boundary("after-logical-commit", Some(next_step));
                        Ok(())
                    })
                    .map_err(|e| super::MigrationError::DbError(e.to_string()))
                },
            )
            .map_err(storage)?;
        drop(checkpoint);
        physical_boundary("after-lexical-durability", None);
        drop(conn);
        drop(catalog);
        self.close_catalog()?;
        // Actual close/reopen boundary, including blob-backed lexical handles.
        let conn = connection(&self.root, true)?;
        let head = read_head(&conn, &self.origin)?;
        let entry = journal(&conn)?.ok_or_else(|| invalid("missing committed initial plan"))?;
        validate_pending(&conn, &head, &entry)?;
        let required_steps = plan.stored.program.step_scopes().map_err(storage)?.len();
        if entry
            .committed
            .as_ref()
            .is_none_or(|prefix| prefix.next_step != required_steps)
        {
            return Err(invalid("initial program is not fully committed"));
        }
        let access = PhysicalAccess {
            origin: self.origin.clone(),
            readonly: true,
            kind: PhysicalAccessKind::ExistingProfile,
        };
        let reader = owned_catalog(&self.root, conn, &access)?;
        verify_probes(&reader, &plan.stored.probes)?;
        let actual = PhysicalSnapshot::read(reader.lock().map_err(storage)?.conn())?;
        // Close the verification reader before reacquiring the exclusive writer.
        self.catalog = Some(reader);
        self.close_catalog()?;
        let conn = connection(&self.root, false)?;
        let source = read_head(&conn, &self.origin)?;
        let entry = journal(&conn)?
            .ok_or_else(|| invalid("missing initial journal before final commit"))?;
        validate_pending(&conn, &source, &entry)?;
        if PhysicalSnapshot::read(&conn)? != actual {
            return Err(invalid(
                "verified materialization changed before publication",
            ));
        }
        let mut registry = read_registry(&conn)?;
        let declaration = registry.apply(&plan.stored.intent)?.receipt().clone();
        let target = StoredHead {
            format: FORMAT.into(),
            origin: self.origin.clone(),
            publication: declaration.publication().into(),
            descriptor: descriptor(&declaration),
            snapshot: actual,
            cells: source.cells.clone(),
        };
        if target.descriptor != plan.stored.inputs.target {
            return Err(invalid("final target descriptor differs from frozen plan"));
        }
        transaction(&conn, || {
            write_meta(&conn, REGISTRY_KEY, &registry)?;
            write_meta(&conn, STATE_KEY, &target)?;
            write_journal(&conn, plan, "applied", entry.committed.as_ref())
        })?;
        physical_boundary("after-publication", None);
        drop(conn);
        let reopened = Self::open_owned(&self.root, &self.origin)?;
        self.catalog = reopened.catalog;
        Ok(PhysicalPublicationReceipt {
            declaration,
            state: target.state(),
            plan_identity: plan.identity().into(),
        })
    }
    pub fn recover_initial_batch(
        root: impl AsRef<Path>,
        origin: &str,
        runner: &MigrationRunner,
        operation: &str,
    ) -> Result<(Self, PhysicalPublicationReceipt)> {
        let root = root.as_ref();
        existing(root)?;
        let plan = {
            let reader = connection(root, true)?;
            let head = read_head(&reader, origin)?;
            let entry = journal(&reader)?.ok_or_else(|| invalid("no durable initial operation"))?;
            if entry.plan.operation() != operation {
                return Err(PublicationError::OperationConflict);
            }
            if entry.status == "applied" {
                drop(reader);
                let owned = Self::open_owned(root, origin)?;
                let receipt = owned.resolve_operation(operation)?;
                return Ok((owned, receipt));
            }
            validate_pending(&reader, &head, &entry)?;
            entry.plan
        };
        let conn = connection(root, false)?;
        let head = read_head(&conn, origin)?;
        let entry = journal(&conn)?.ok_or_else(|| invalid("lost initial operation"))?;
        validate_pending(&conn, &head, &entry)?;
        let access = PhysicalAccess {
            origin: origin.into(),
            readonly: false,
            kind: PhysicalAccessKind::ExistingProfile,
        };
        let catalog = owned_catalog(root, conn, &access)?;
        let mut owned = Self {
            root: root.into(),
            origin: origin.into(),
            catalog: Some(catalog),
            poisoned: false,
        };
        let receipt = owned.execute_initial_batch(runner, &plan)?;
        Ok((owned, receipt))
    }
    pub fn resolve_operation(&self, operation: &str) -> Result<PhysicalPublicationReceipt> {
        let state = self.observe_published()?;
        let catalog = self.catalog()?;
        let cat = catalog.lock().map_err(storage)?;
        let entry = journal(cat.conn())?.ok_or_else(|| invalid("unknown physical operation"))?;
        if entry.status != "applied" || entry.plan.operation() != operation {
            return Err(PublicationError::OperationConflict);
        }
        let registry = read_registry(cat.conn())?;
        let declaration = registry.published(entry.plan.target_publication())?;
        if declaration.publication() != state.publication {
            return Err(invalid("initial receipt not at published head"));
        }
        Ok(PhysicalPublicationReceipt {
            declaration,
            state,
            plan_identity: entry.plan.identity().into(),
        })
    }
}

#[cfg(test)]
mod tests;

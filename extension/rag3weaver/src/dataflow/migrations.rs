//! Migration runner for schema migrations via dataflow graphs.
//!
//! Migrations are `.mmd` (Mermaid) files parsed into dataflow graphs and executed
//! via [`DataflowRuntime`]. Each migration benefits from checkpoint (crash recovery),
//! observability, and undo support.
//!
//! ## File convention
//!
//! ```text
//! migrations/
//!   001_add_version_field.mmd
//!   002_rename_entity_type.mmd
//! migrations/internal/
//!   001_create_dataflow_tables.mmd
//! ```
//!
//! Format: `{version}_{name}.mmd` where version is a zero-padded integer.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::checkpoint::{CheckpointStore, timestamp_ms};
use super::checkpoint_store::CypherCheckpointStore;
use super::graph::DataflowGraph;
use super::mermaid::parse_mermaid_template;
use super::node_registry::NodeRegistry;
use super::runtime::DataflowRuntime;
use super::services::ServiceRegistry;
use crate::connection::{CypherValue, DbConnection, QueryParam};
use crate::hash::content_hash;

// ─── MigrationFile ──────────────────────────────────────────────────────────

/// A migration file discovered on disk.
#[derive(Debug, Clone)]
pub struct MigrationFile {
    pub version: u64,
    pub name: String,
    pub path: PathBuf,
    pub content: String,
    pub checksum: String,
}

/// Parse a migration filename like `001_add_version.mmd` into (version, name).
fn parse_migration_filename(filename: &str) -> Option<(u64, String)> {
    let stem = filename.strip_suffix(".mmd")?;
    let underscore = stem.find('_')?;
    let version_str = &stem[..underscore];
    let name = &stem[underscore + 1..];
    if name.is_empty() {
        return None;
    }
    let version: u64 = version_str.parse().ok()?;
    Some((version, name.to_string()))
}

/// Scan a directory for `.mmd` migration files, sorted by version.
fn scan_migration_dir(dir: &Path) -> Result<Vec<MigrationFile>, MigrationError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(dir)
        .map_err(|e| MigrationError::IoError(format!("cannot read {}: {e}", dir.display())))?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry
            .map_err(|e| MigrationError::IoError(format!("readdir entry: {e}")))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if !filename.ends_with(".mmd") {
            continue;
        }
        let (version, name) = match parse_migration_filename(&filename) {
            Some(v) => v,
            None => {
                return Err(MigrationError::InvalidFilename {
                    path: path.clone(),
                    detail: format!("expected format {{version}}_{{name}}.mmd, got '{filename}'"),
                });
            }
        };
        let content = std::fs::read_to_string(&path)
            .map_err(|e| MigrationError::IoError(format!("cannot read {}: {e}", path.display())))?;
        let checksum = content_hash(&content);
        files.push(MigrationFile {
            version,
            name,
            path,
            content,
            checksum,
        });
    }

    files.sort_by_key(|f| f.version);

    // Check for duplicate versions
    for pair in files.windows(2) {
        if pair[0].version == pair[1].version {
            return Err(MigrationError::DuplicateVersion {
                version: pair[0].version,
                file_a: pair[0].path.clone(),
                file_b: pair[1].path.clone(),
            });
        }
    }

    Ok(files)
}

// ─── MigrationStatus ────────────────────────────────────────────────────────

/// Status of a migration (file + DB state combined).
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub version: u64,
    pub name: String,
    pub state: MigrationState,
    pub checksum: String,
    /// Checksum stored in DB (if applied). None if pending.
    pub db_checksum: Option<String>,
    /// Execution ID for the migration (if applied).
    pub execution_id: Option<String>,
    pub applied_at: Option<u64>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationState {
    Pending,
    Applied,
    Failed,
    RolledBack,
}

impl fmt::Display for MigrationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Applied => write!(f, "applied"),
            Self::Failed => write!(f, "failed"),
            Self::RolledBack => write!(f, "rolled_back"),
        }
    }
}

// ─── MigrationResult ────────────────────────────────────────────────────────

/// Result of applying or rolling back a single migration.
#[derive(Debug, Clone)]
pub struct MigrationResult {
    pub version: u64,
    pub name: String,
    pub direction: MigrationDirection,
    pub state: MigrationState,
    pub duration_ms: u64,
    pub error: Option<String>,
    /// If dry_run, the parsed graph definition (for inspection).
    pub dry_run_plan: Option<DryRunPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationDirection {
    Up,
    Down,
}

impl fmt::Display for MigrationDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Up => write!(f, "up"),
            Self::Down => write!(f, "down"),
        }
    }
}

/// Dry-run plan: parsed graph info without execution.
#[derive(Debug, Clone)]
pub struct DryRunPlan {
    pub node_count: usize,
    pub edge_count: usize,
    pub nodes: Vec<DryRunNode>,
    pub all_reversible: bool,
}

#[derive(Debug, Clone)]
pub struct DryRunNode {
    pub name: String,
    pub node_type: String,
    pub config: serde_json::Value,
    pub can_undo: bool,
}

// ─── MigrationError ─────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum MigrationError {
    IoError(String),
    InvalidFilename { path: PathBuf, detail: String },
    DuplicateVersion { version: u64, file_a: PathBuf, file_b: PathBuf },
    ParseError { version: u64, name: String, detail: String },
    GraphError { version: u64, name: String, detail: String },
    ExecutionError { version: u64, name: String, detail: String },
    Locked { by: String, since: u64 },
    NotApplied { version: u64 },
    NotReversible { version: u64, name: String, nodes: Vec<String> },
    DbError(String),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "IO error: {e}"),
            Self::InvalidFilename { path, detail } => {
                write!(f, "invalid migration filename '{}': {detail}", path.display())
            }
            Self::DuplicateVersion { version, file_a, file_b } => {
                write!(
                    f,
                    "duplicate version {version}: '{}' and '{}'",
                    file_a.display(),
                    file_b.display()
                )
            }
            Self::ParseError { version, name, detail } => {
                write!(f, "migration {version}_{name}: parse error: {detail}")
            }
            Self::GraphError { version, name, detail } => {
                write!(f, "migration {version}_{name}: graph error: {detail}")
            }
            Self::ExecutionError { version, name, detail } => {
                write!(f, "migration {version}_{name}: execution error: {detail}")
            }
            Self::Locked { by, since } => {
                write!(f, "migrations locked by '{by}' since {since}")
            }
            Self::NotApplied { version } => {
                write!(f, "migration {version} is not applied")
            }
            Self::NotReversible { version, name, nodes } => {
                write!(
                    f,
                    "migration {version}_{name}: not reversible — nodes without undo: {}",
                    nodes.join(", ")
                )
            }
            Self::DbError(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for MigrationError {}

// ─── MigrationRunner ────────────────────────────────────────────────────────

const LOCK_TTL_MS: u64 = 10 * 60 * 1000; // 10 minutes
const LOCK_UUID: &str = "_migration_lock";

/// Runs schema migrations defined as Mermaid dataflow graphs.
pub struct MigrationRunner {
    conn: Arc<dyn DbConnection>,
    registry: Arc<NodeRegistry>,
    migration_dirs: Vec<PathBuf>,
    lock_id: String,
}

impl MigrationRunner {
    pub fn new(conn: Arc<dyn DbConnection>, registry: Arc<NodeRegistry>) -> Self {
        let lock_id = format!("runner-{}", timestamp_ms());
        Self {
            conn,
            registry,
            migration_dirs: Vec::new(),
            lock_id,
        }
    }

    /// Add a directory to scan for migration files.
    pub fn add_dir(&mut self, dir: PathBuf) {
        self.migration_dirs.push(dir);
    }

    // ─── Schema initialization ──────────────────────────────────────────

    /// Ensure migration tables exist.
    pub async fn initialize(&self) -> Result<(), MigrationError> {
        self.conn
            .execute(
                "CREATE NODE TABLE IF NOT EXISTS _DataflowMigration(\
                     _uuid STRING, \
                     version INT64, \
                     name STRING, \
                     status STRING, \
                     direction STRING, \
                     checksum STRING, \
                     execution_id STRING, \
                     applied_at INT64, \
                     duration_ms INT64, \
                     error STRING, \
                     PRIMARY KEY(_uuid))",
            )
            .await
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        self.conn
            .execute(
                "CREATE NODE TABLE IF NOT EXISTS _DataflowMigrationLock(\
                     _uuid STRING, \
                     locked_by STRING, \
                     locked_at INT64, \
                     expires_at INT64, \
                     PRIMARY KEY(_uuid))",
            )
            .await
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        Ok(())
    }

    // ─── Scanning ───────────────────────────────────────────────────────

    /// Scan all registered directories for migration files.
    fn scan_all(&self) -> Result<Vec<MigrationFile>, MigrationError> {
        let mut all_files = Vec::new();
        for dir in &self.migration_dirs {
            let mut files = scan_migration_dir(dir)?;
            all_files.append(&mut files);
        }
        all_files.sort_by_key(|f| f.version);

        // Check for cross-directory duplicates
        for pair in all_files.windows(2) {
            if pair[0].version == pair[1].version {
                return Err(MigrationError::DuplicateVersion {
                    version: pair[0].version,
                    file_a: pair[0].path.clone(),
                    file_b: pair[1].path.clone(),
                });
            }
        }

        Ok(all_files)
    }

    /// Load applied migrations from the database.
    async fn load_applied(&self) -> Result<HashMap<u64, AppliedMigration>, MigrationError> {
        let result = self
            .conn
            .execute(
                "MATCH (m:_DataflowMigration) \
                 RETURN m.version, m.name, m.status, m.checksum, \
                        m.execution_id, m.applied_at, m.duration_ms, m.error \
                 ORDER BY m.version",
            )
            .await
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        let mut applied = HashMap::new();
        for row in &result.rows {
            let version = row[0].as_i64().unwrap_or(0) as u64;
            let name = row[1].as_str().unwrap_or("").to_string();
            let status = row[2].as_str().unwrap_or("applied").to_string();
            let checksum = row[3].as_str().unwrap_or("").to_string();
            let execution_id = row[4].as_str().unwrap_or("").to_string();
            let applied_at = row[5].as_i64().unwrap_or(0) as u64;
            let duration_ms = row[6].as_i64().unwrap_or(0) as u64;
            let error = row[7].as_str().unwrap_or("").to_string();

            applied.insert(version, AppliedMigration {
                name,
                status,
                checksum,
                execution_id,
                applied_at,
                duration_ms,
                error,
            });
        }

        Ok(applied)
    }

    // ─── Status ─────────────────────────────────────────────────────────

    /// List all migrations with their current status.
    pub async fn status(&self) -> Result<Vec<MigrationStatus>, MigrationError> {
        let files = self.scan_all()?;
        let applied = self.load_applied().await?;

        let mut statuses = Vec::new();
        for file in &files {
            let (state, db_checksum, execution_id, applied_at, duration_ms) =
                if let Some(am) = applied.get(&file.version) {
                    let state = match am.status.as_str() {
                        "applied" => MigrationState::Applied,
                        "failed" => MigrationState::Failed,
                        "rolled_back" => MigrationState::RolledBack,
                        _ => MigrationState::Pending,
                    };
                    (
                        state,
                        Some(am.checksum.clone()),
                        Some(am.execution_id.clone()),
                        Some(am.applied_at),
                        Some(am.duration_ms),
                    )
                } else {
                    (MigrationState::Pending, None, None, None, None)
                };

            statuses.push(MigrationStatus {
                version: file.version,
                name: file.name.clone(),
                state,
                checksum: file.checksum.clone(),
                db_checksum,
                execution_id,
                applied_at,
                duration_ms,
            });
        }

        Ok(statuses)
    }

    /// List pending migrations (not yet applied), sorted by version.
    pub async fn pending(&self) -> Result<Vec<MigrationFile>, MigrationError> {
        let files = self.scan_all()?;
        let applied = self.load_applied().await?;

        Ok(files
            .into_iter()
            .filter(|f| {
                applied
                    .get(&f.version)
                    .map_or(true, |am| am.status != "applied")
            })
            .collect())
    }

    // ─── Apply ──────────────────────────────────────────────────────────

    /// Apply pending migrations, optionally up to a target version.
    ///
    /// If `dry_run` is true, parse and validate each migration but do not execute.
    pub async fn apply(
        &self,
        target_version: Option<u64>,
        dry_run: bool,
        vars: &HashMap<String, String>,
    ) -> Result<Vec<MigrationResult>, MigrationError> {
        let mut pending = self.pending().await?;
        if let Some(target) = target_version {
            pending.retain(|f| f.version <= target);
        }

        if pending.is_empty() {
            return Ok(Vec::new());
        }

        if !dry_run {
            self.acquire_lock().await?;
        }

        let mut results = Vec::new();
        for file in &pending {
            let result = if dry_run {
                self.dry_run_migration(file, vars)?
            } else {
                self.apply_migration(file, vars).await
                    .unwrap_or_else(|e| MigrationResult {
                        version: file.version,
                        name: file.name.clone(),
                        direction: MigrationDirection::Up,
                        state: MigrationState::Failed,
                        duration_ms: 0,
                        error: Some(e.to_string()),
                        dry_run_plan: None,
                    })
            };

            let failed = result.state == MigrationState::Failed;
            results.push(result);
            if failed {
                break; // Stop on first failure
            }
        }

        if !dry_run {
            self.release_lock().await.ok(); // Best-effort release
        }

        Ok(results)
    }

    /// Dry-run: parse + validate + check reversibility.
    fn dry_run_migration(
        &self,
        file: &MigrationFile,
        vars: &HashMap<String, String>,
    ) -> Result<MigrationResult, MigrationError> {
        let def = parse_mermaid_template(&file.content, vars)
            .map_err(|e| MigrationError::ParseError {
                version: file.version,
                name: file.name.clone(),
                detail: e.to_string(),
            })?;

        let graph = DataflowGraph::from_definition(&def, &self.registry)
            .map_err(|e| MigrationError::GraphError {
                version: file.version,
                name: file.name.clone(),
                detail: e,
            })?;

        let nodes: Vec<DryRunNode> = graph
            .nodes
            .iter()
            .map(|n| DryRunNode {
                name: n.name().to_string(),
                node_type: n.node_type().to_string(),
                config: n.node_config(),
                can_undo: n.can_undo(),
            })
            .collect();

        let all_reversible = nodes.iter().all(|n| n.can_undo);

        Ok(MigrationResult {
            version: file.version,
            name: file.name.clone(),
            direction: MigrationDirection::Up,
            state: MigrationState::Pending, // dry-run — not applied
            duration_ms: 0,
            error: None,
            dry_run_plan: Some(DryRunPlan {
                node_count: nodes.len(),
                edge_count: graph.edges.len(),
                nodes,
                all_reversible,
            }),
        })
    }

    /// Apply a single migration.
    async fn apply_migration(
        &self,
        file: &MigrationFile,
        vars: &HashMap<String, String>,
    ) -> Result<MigrationResult, MigrationError> {
        let start = std::time::Instant::now();

        // Parse
        let def = parse_mermaid_template(&file.content, vars)
            .map_err(|e| MigrationError::ParseError {
                version: file.version,
                name: file.name.clone(),
                detail: e.to_string(),
            })?;

        // Build graph
        let mut graph = DataflowGraph::from_definition(&def, &self.registry)
            .map_err(|e| MigrationError::GraphError {
                version: file.version,
                name: file.name.clone(),
                detail: e,
            })?;

        // Set up services (conn for CypherNode/ValidateNode)
        let mut services = ServiceRegistry::new();
        services.register::<Arc<dyn DbConnection>>("conn", Arc::new(self.conn.clone()));

        // Set up checkpoint store
        let checkpoint_store = CypherCheckpointStore::new(self.conn.clone());
        checkpoint_store.initialize().await
            .map_err(|e| MigrationError::DbError(e))?;

        // Execution ID for this migration
        let execution_id = format!("migration-{:03}_{}", file.version, file.name);

        // Execute via DataflowRuntime
        let runtime = DataflowRuntime::with_services(100, services);
        let exec_result = runtime
            .execute_with_checkpoint(&mut graph, &checkpoint_store, &execution_id)
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match exec_result {
            Ok(_) => {
                // Record success
                self.record_migration(
                    file, "applied", "up", &execution_id, duration_ms, "",
                )
                .await?;

                Ok(MigrationResult {
                    version: file.version,
                    name: file.name.clone(),
                    direction: MigrationDirection::Up,
                    state: MigrationState::Applied,
                    duration_ms,
                    error: None,
                    dry_run_plan: None,
                })
            }
            Err(e) => {
                // Record failure
                self.record_migration(
                    file, "failed", "up", &execution_id, duration_ms, &e,
                )
                .await
                .ok(); // Best-effort

                Err(MigrationError::ExecutionError {
                    version: file.version,
                    name: file.name.clone(),
                    detail: e,
                })
            }
        }
    }

    // ─── Rollback ───────────────────────────────────────────────────────

    /// Rollback a previously applied migration.
    ///
    /// Loads the undo contexts from the checkpoint, reconstructs the nodes,
    /// and calls undo() in reverse topological order.
    pub async fn rollback(
        &self,
        version: u64,
    ) -> Result<MigrationResult, MigrationError> {
        let start = std::time::Instant::now();

        // Check it was applied
        let applied = self.load_applied().await?;
        let am = applied.get(&version).ok_or(MigrationError::NotApplied { version })?;
        if am.status != "applied" {
            return Err(MigrationError::NotApplied { version });
        }

        self.acquire_lock().await?;

        // Load the checkpoint for this execution
        let checkpoint_store = CypherCheckpointStore::new(self.conn.clone());
        let checkpoint = checkpoint_store
            .load_execution(&am.execution_id)
            .await
            .map_err(|e| MigrationError::DbError(e))?
            .ok_or_else(|| MigrationError::DbError(
                format!("checkpoint not found for execution '{}'", am.execution_id)
            ))?;

        // Rebuild the graph to get the nodes + topological order
        let mut graph = DataflowGraph::from_definition(&checkpoint.graph_def, &self.registry)
            .map_err(|e| MigrationError::GraphError {
                version,
                name: am.name.clone(),
                detail: e,
            })?;

        // Check all nodes are reversible
        let non_reversible: Vec<String> = graph
            .nodes
            .iter()
            .filter(|n| !n.can_undo())
            .map(|n| n.name().to_string())
            .collect();
        if !non_reversible.is_empty() {
            self.release_lock().await.ok();
            return Err(MigrationError::NotReversible {
                version,
                name: am.name.clone(),
                nodes: non_reversible,
            });
        }

        // Get topological order and reverse it
        let order = graph
            .topological_sort()
            .map_err(|e| MigrationError::GraphError {
                version,
                name: am.name.clone(),
                detail: e,
            })?;
        let reversed: Vec<String> = order.into_iter().rev().collect();

        // Set up services
        let mut services = ServiceRegistry::new();
        services.register::<Arc<dyn DbConnection>>("conn", Arc::new(self.conn.clone()));
        let services = Arc::new(services);

        // Execute undo in reverse order
        for node_name in &reversed {
            let node_idx = graph.nodes.iter().position(|n| n.name() == node_name)
                .ok_or_else(|| MigrationError::GraphError {
                    version,
                    name: am.name.clone(),
                    detail: format!("node '{}' not found in graph", node_name),
                })?;
            let node = &mut graph.nodes[node_idx];

            // Load undo context from checkpoint
            let undo_ctx = checkpoint
                .nodes
                .get(node_name.as_str())
                .and_then(|nc| nc.undo_context.clone());

            if let Some(undo_ctx) = undo_ctx {
                let mut ctx = super::node::NodeContext::with_services(
                    services.clone(),
                );
                node.undo(&mut ctx, undo_ctx)
                    .await
                    .map_err(|e| MigrationError::ExecutionError {
                        version,
                        name: am.name.clone(),
                        detail: format!("undo of node '{}' failed: {e}", node_name),
                    })?;
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        // Find the file to get checksum
        let files = self.scan_all()?;
        let file = files.iter().find(|f| f.version == version);

        // Record the rollback
        if let Some(file) = file {
            self.record_migration(
                file, "rolled_back", "down", &am.execution_id, duration_ms, "",
            )
            .await?;
        } else {
            // File may have been deleted — update the existing record
            self.update_migration_status(version, "rolled_back").await?;
        }

        self.release_lock().await.ok();

        Ok(MigrationResult {
            version,
            name: am.name.clone(),
            direction: MigrationDirection::Down,
            state: MigrationState::RolledBack,
            duration_ms,
            error: None,
            dry_run_plan: None,
        })
    }

    // ─── Check reversibility ────────────────────────────────────────────

    /// Check which pending migrations are fully reversible.
    pub async fn check_reversible(
        &self,
        vars: &HashMap<String, String>,
    ) -> Result<Vec<(MigrationFile, bool)>, MigrationError> {
        let pending = self.pending().await?;
        let mut results = Vec::new();

        for file in pending {
            let def = parse_mermaid_template(&file.content, vars)
                .map_err(|e| MigrationError::ParseError {
                    version: file.version,
                    name: file.name.clone(),
                    detail: e.to_string(),
                })?;

            let graph = DataflowGraph::from_definition(&def, &self.registry)
                .map_err(|e| MigrationError::GraphError {
                    version: file.version,
                    name: file.name.clone(),
                    detail: e,
                })?;

            let reversible = graph.nodes.iter().all(|n| n.can_undo());
            results.push((file, reversible));
        }

        Ok(results)
    }

    // ─── Locking ────────────────────────────────────────────────────────

    async fn acquire_lock(&self) -> Result<(), MigrationError> {
        let now = timestamp_ms();

        // Check for existing lock
        let result = self
            .conn
            .execute_with_params(
                "MATCH (l:_DataflowMigrationLock {_uuid: $uuid}) \
                 RETURN l.locked_by, l.locked_at, l.expires_at",
                &[QueryParam::new("uuid", CypherValue::String(LOCK_UUID.to_string()))],
            )
            .await
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        if let Some(row) = result.rows.first() {
            let locked_by = row[0].as_str().unwrap_or("unknown").to_string();
            let locked_at = row[1].as_i64().unwrap_or(0) as u64;
            let expires_at = row[2].as_i64().unwrap_or(0) as u64;

            if now < expires_at {
                // Lock is still valid
                return Err(MigrationError::Locked {
                    by: locked_by,
                    since: locked_at,
                });
            }
            // Lock expired — delete it
            self.conn
                .execute_with_params(
                    "MATCH (l:_DataflowMigrationLock {_uuid: $uuid}) DELETE l",
                    &[QueryParam::new("uuid", CypherValue::String(LOCK_UUID.to_string()))],
                )
                .await
                .map_err(|e| MigrationError::DbError(e.to_string()))?;
        }

        // Create lock
        self.conn
            .execute_with_params(
                "CREATE (l:_DataflowMigrationLock {\
                     _uuid: $uuid, \
                     locked_by: $locked_by, \
                     locked_at: $locked_at, \
                     expires_at: $expires_at})",
                &[
                    QueryParam::new("uuid", CypherValue::String(LOCK_UUID.to_string())),
                    QueryParam::new("locked_by", CypherValue::String(self.lock_id.clone())),
                    QueryParam::new("locked_at", CypherValue::Int(now as i64)),
                    QueryParam::new("expires_at", CypherValue::Int((now + LOCK_TTL_MS) as i64)),
                ],
            )
            .await
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        Ok(())
    }

    async fn release_lock(&self) -> Result<(), MigrationError> {
        self.conn
            .execute_with_params(
                "MATCH (l:_DataflowMigrationLock {_uuid: $uuid}) DELETE l",
                &[QueryParam::new("uuid", CypherValue::String(LOCK_UUID.to_string()))],
            )
            .await
            .map_err(|e| MigrationError::DbError(e.to_string()))?;
        Ok(())
    }

    // ─── DB helpers ─────────────────────────────────────────────────────

    async fn record_migration(
        &self,
        file: &MigrationFile,
        status: &str,
        direction: &str,
        execution_id: &str,
        duration_ms: u64,
        error: &str,
    ) -> Result<(), MigrationError> {
        let uuid = format!("migration-{:03}", file.version);
        let now = timestamp_ms();

        self.conn
            .execute_with_params(
                "MERGE (m:_DataflowMigration {_uuid: $uuid}) \
                 SET m.version = $version, \
                     m.name = $name, \
                     m.status = $status, \
                     m.direction = $direction, \
                     m.checksum = $checksum, \
                     m.execution_id = $execution_id, \
                     m.applied_at = $applied_at, \
                     m.duration_ms = $duration_ms, \
                     m.error = $error",
                &[
                    QueryParam::new("uuid", CypherValue::String(uuid)),
                    QueryParam::new("version", CypherValue::Int(file.version as i64)),
                    QueryParam::new("name", CypherValue::String(file.name.clone())),
                    QueryParam::new("status", CypherValue::String(status.to_string())),
                    QueryParam::new("direction", CypherValue::String(direction.to_string())),
                    QueryParam::new("checksum", CypherValue::String(file.checksum.clone())),
                    QueryParam::new("execution_id", CypherValue::String(execution_id.to_string())),
                    QueryParam::new("applied_at", CypherValue::Int(now as i64)),
                    QueryParam::new("duration_ms", CypherValue::Int(duration_ms as i64)),
                    QueryParam::new("error", CypherValue::String(error.to_string())),
                ],
            )
            .await
            .map_err(|e| MigrationError::DbError(e.to_string()))?;

        Ok(())
    }

    async fn update_migration_status(
        &self,
        version: u64,
        status: &str,
    ) -> Result<(), MigrationError> {
        let uuid = format!("migration-{:03}", version);
        self.conn
            .execute_with_params(
                "MATCH (m:_DataflowMigration {_uuid: $uuid}) SET m.status = $status",
                &[
                    QueryParam::new("uuid", CypherValue::String(uuid)),
                    QueryParam::new("status", CypherValue::String(status.to_string())),
                ],
            )
            .await
            .map_err(|e| MigrationError::DbError(e.to_string()))?;
        Ok(())
    }
}

// ─── Internal types ─────────────────────────────────────────────────────────

struct AppliedMigration {
    name: String,
    status: String,
    checksum: String,
    execution_id: String,
    applied_at: u64,
    duration_ms: u64,
    #[allow(dead_code)]
    error: String,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    // ── Filename parsing ────────────────────────────────────────────────

    #[test]
    fn parse_valid_filename() {
        let (v, name) = parse_migration_filename("001_add_version.mmd").unwrap();
        assert_eq!(v, 1);
        assert_eq!(name, "add_version");
    }

    #[test]
    fn parse_zero_padded_version() {
        let (v, name) = parse_migration_filename("042_rename_field.mmd").unwrap();
        assert_eq!(v, 42);
        assert_eq!(name, "rename_field");
    }

    #[test]
    fn parse_large_version() {
        let (v, name) = parse_migration_filename("1000_big_change.mmd").unwrap();
        assert_eq!(v, 1000);
        assert_eq!(name, "big_change");
    }

    #[test]
    fn parse_invalid_no_underscore() {
        assert!(parse_migration_filename("001addversion.mmd").is_none());
    }

    #[test]
    fn parse_invalid_no_extension() {
        assert!(parse_migration_filename("001_add_version.txt").is_none());
    }

    #[test]
    fn parse_invalid_non_numeric_version() {
        assert!(parse_migration_filename("abc_add_version.mmd").is_none());
    }

    #[test]
    fn parse_invalid_empty_name() {
        assert!(parse_migration_filename("001_.mmd").is_none());
    }

    #[test]
    fn parse_name_with_underscores() {
        let (v, name) = parse_migration_filename("003_add_embedding_index.mmd").unwrap();
        assert_eq!(v, 3);
        assert_eq!(name, "add_embedding_index");
    }

    // ── Directory scanning ──────────────────────────────────────────────

    #[test]
    fn scan_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let files = scan_migration_dir(dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn scan_nonexistent_dir() {
        let files = scan_migration_dir(Path::new("/nonexistent/path")).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn scan_dir_with_migrations() {
        let dir = tempfile::tempdir().unwrap();

        // Create migration files (out of order on disk)
        let content_2 = "graph LR\n    step[\"CypherNode(query='RETURN 2')\"]\n";
        let content_1 = "graph LR\n    step[\"CypherNode(query='RETURN 1')\"]\n";

        std::fs::write(dir.path().join("002_second.mmd"), content_2).unwrap();
        std::fs::write(dir.path().join("001_first.mmd"), content_1).unwrap();

        let files = scan_migration_dir(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].version, 1);
        assert_eq!(files[0].name, "first");
        assert_eq!(files[1].version, 2);
        assert_eq!(files[1].name, "second");
    }

    #[test]
    fn scan_ignores_non_mmd_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("001_first.mmd"), "graph LR\n").unwrap();
        std::fs::write(dir.path().join("README.md"), "# readme").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "notes").unwrap();

        let files = scan_migration_dir(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn scan_detects_duplicate_versions() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("001_first.mmd"), "graph LR\n").unwrap();
        std::fs::write(dir.path().join("001_duplicate.mmd"), "graph LR\n").unwrap();

        let err = scan_migration_dir(dir.path()).unwrap_err();
        assert!(err.to_string().contains("duplicate version 1"));
    }

    #[test]
    fn scan_checksum_is_content_hash() {
        let dir = tempfile::tempdir().unwrap();
        let content = "graph LR\n    step[\"CypherNode(query='RETURN 1')\"]\n";
        std::fs::write(dir.path().join("001_test.mmd"), content).unwrap();

        let files = scan_migration_dir(dir.path()).unwrap();
        assert_eq!(files[0].checksum, content_hash(content));
    }

    // ── MigrationState display ──────────────────────────────────────────

    #[test]
    fn migration_state_display() {
        assert_eq!(MigrationState::Pending.to_string(), "pending");
        assert_eq!(MigrationState::Applied.to_string(), "applied");
        assert_eq!(MigrationState::Failed.to_string(), "failed");
        assert_eq!(MigrationState::RolledBack.to_string(), "rolled_back");
    }

    // ── MigrationDirection display ──────────────────────────────────────

    #[test]
    fn migration_direction_display() {
        assert_eq!(MigrationDirection::Up.to_string(), "up");
        assert_eq!(MigrationDirection::Down.to_string(), "down");
    }

    // ── MigrationError display ──────────────────────────────────────────

    #[test]
    fn error_display_locked() {
        let err = MigrationError::Locked {
            by: "runner-abc".into(),
            since: 1000,
        };
        assert!(err.to_string().contains("locked by 'runner-abc'"));
    }

    #[test]
    fn error_display_not_reversible() {
        let err = MigrationError::NotReversible {
            version: 1,
            name: "test".into(),
            nodes: vec!["step1".into(), "step2".into()],
        };
        let s = err.to_string();
        assert!(s.contains("not reversible"));
        assert!(s.contains("step1, step2"));
    }

    #[test]
    fn error_display_duplicate_version() {
        let err = MigrationError::DuplicateVersion {
            version: 1,
            file_a: PathBuf::from("a.mmd"),
            file_b: PathBuf::from("b.mmd"),
        };
        assert!(err.to_string().contains("duplicate version 1"));
    }

    // ── Internal migration template tests ────────────────────────────────

    fn builtin_registry() -> super::NodeRegistry {
        let mut registry = super::NodeRegistry::new();
        super::super::node_factories::register_builtins(&mut registry);
        registry
    }

    #[test]
    fn internal_001_parses_and_builds() {
        let mmd = include_str!("../../migrations/internal/001_create_dataflow_tables.mmd");
        let def = super::parse_mermaid_template(mmd, &HashMap::new()).unwrap();

        assert_eq!(def.nodes.len(), 4, "expected 4 CypherNode declarations");
        assert_eq!(def.edges.len(), 3, "expected 3 sequential edges");

        // All nodes are CypherNode
        for node in &def.nodes {
            assert_eq!(node.node_type, "CypherNode", "node '{}' should be CypherNode", node.name);
            assert!(
                node.config["query"].as_str().unwrap().contains("CREATE NODE TABLE IF NOT EXISTS"),
                "node '{}' query should be CREATE NODE TABLE",
                node.name,
            );
        }

        // Build the graph via registry
        let registry = builtin_registry();
        let graph = super::DataflowGraph::from_definition(&def, &registry).unwrap();
        assert_eq!(graph.node_names().len(), 4);

        // Topological order: sequential chain
        let order = graph.topological_sort().unwrap();
        assert_eq!(order[0], "create_execution");
        assert_eq!(order[1], "create_node_state");
        assert_eq!(order[2], "create_migration");
        assert_eq!(order[3], "create_lock");
    }

    #[test]
    fn internal_001_not_reversible() {
        let mmd = include_str!("../../migrations/internal/001_create_dataflow_tables.mmd");
        let def = super::parse_mermaid_template(mmd, &HashMap::new()).unwrap();
        let registry = builtin_registry();
        let graph = super::DataflowGraph::from_definition(&def, &registry).unwrap();

        // DDL migrations have no capture_query → not reversible
        for node in &graph.nodes {
            assert!(
                !node.can_undo(),
                "DDL node '{}' should not be reversible",
                node.name(),
            );
        }
    }

    #[test]
    fn internal_001_tables_match_checkpoint_store() {
        // Verify the migration creates the same tables as CypherCheckpointStore::initialize()
        let mmd = include_str!("../../migrations/internal/001_create_dataflow_tables.mmd");
        let def = super::parse_mermaid_template(mmd, &HashMap::new()).unwrap();

        let table_names: Vec<&str> = def
            .nodes
            .iter()
            .filter_map(|n| {
                let query = n.config["query"].as_str()?;
                let start = query.find("_Dataflow")?;
                let end = query[start..].find('(')? + start;
                Some(&query[start..end])
            })
            .collect();

        assert!(table_names.contains(&"_DataflowExecution"));
        assert!(table_names.contains(&"_DataflowNodeState"));
        assert!(table_names.contains(&"_DataflowMigration"));
        assert!(table_names.contains(&"_DataflowMigrationLock"));
    }

    #[test]
    fn internal_001_scan_from_disk() {
        // Verify the migration file is properly scannable
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("migrations")
            .join("internal");
        let files = scan_migration_dir(&dir).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].version, 1);
        assert_eq!(files[0].name, "create_dataflow_tables");
        assert!(!files[0].checksum.is_empty());
    }
}

//! Database connection trait and value types.
//!
//! The [`DbConnection`] trait abstracts over the rag3db database connection,
//! allowing the pipeline to execute Cypher queries without depending on
//! the concrete database implementation.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during database operations.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("query error: {0}")]
    QueryError(String),

    #[error("connection error: {0}")]
    ConnectionError(String),

    #[error("type error: {0}")]
    TypeError(String),

    /// **La base doit être fermée puis rouverte** : un point de reprise y a
    /// échoué, et le moteur refuse tout jusqu'à la réouverture
    /// ([`REOPEN_AFTER_FAILED_CHECKPOINT`]), ou son tampon a été plein
    /// ([`BUFFER_POOL_FULL`]). Rien n'est sûr d'ici là — ni retenter, ni
    /// rejouer.
    #[error("must reopen: {0}")]
    MustReopen(String),
}

/// **Le nom stable du refus du moteur** après un point de reprise échoué —
/// le début de `TransactionManager::REOPEN_AFTER_FAILED_CHECKPOINT`
/// (`src/include/transaction/transaction_manager.h`). C'est sur lui, et sur
/// lui seul, qu'une erreur du moteur est reconnue.
pub const REOPEN_AFTER_FAILED_CHECKPOINT: &str =
    "A checkpoint of this database failed, so it must be closed and reopened";

/// **Le tampon du moteur plein** (`MemoryManager`, « Unable to allocate
/// memory! The buffer pool is full and no memory could be freed! »). Un
/// COPY refusé ainsi après avoir réservé ses lignes laisse la table fausse
/// en mémoire jusqu'à la réouverture (défaut du moteur, cœur C++, 5 octobre
/// 2026) : le repli MERGE y aurait cherché ses clés et écrit des doublons.
/// Reconnu comme une raison de rouvrir, comme un point de reprise échoué.
/// **Un filet à garder** même après le correctif du moteur, tant qu'une
/// preuve ne dit pas qu'il est inutile.
pub const BUFFER_POOL_FULL: &str = "The buffer pool is full";

/// **Le code de sortie d'un hôte dont la base doit être rouverte** (75,
/// `EX_TEMPFAIL`) : `rag3weaver-backend` et `rag3daemon` répondent l'erreur,
/// puis s'arrêtent avec ce code pour que leur lanceur les relance — un
/// processus neuf est la seule façon sûre de lâcher la base et tout ce que
/// la mémoire en savait, comme PostgreSQL après un PANIC.
pub const EXIT_MUST_REOPEN: i32 = 75;

/// A Cypher-compatible value. Mirrors the types that rag3db (Kuzu) supports.
///
/// Variant order matters for `#[serde(untagged)]`: `Int` before `Float`
/// ensures that `42` deserializes as `Int(42)`, not `Float(42.0)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CypherValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<CypherValue>),
    Map(BTreeMap<String, CypherValue>),
    #[serde(skip)]
    Blob(Vec<u8>),
    /// Parameter-only type annotation; stored records/checkpoints retain plain values.
    #[serde(skip)]
    Typed { value: Box<CypherValue>, field_type: crate::config::FieldType },
}

impl CypherValue {
    /// Validate explicit parameter types before crossing the native FFI boundary.
    pub fn validate_parameter_types(&self) -> Result<(), String> {
        match self {
            Self::Typed { value, field_type } => {
                crate::config::validate_payload_type(field_type, 0)?;
                let json = serde_json::to_value(value).map_err(|e| e.to_string())?;
                crate::json_schema::normalize(field_type, &json).map(|_| ())
            }
            Self::List(values) => values.iter().try_for_each(Self::validate_parameter_types),
            Self::Map(values) => values.values().try_for_each(Self::validate_parameter_types),
            _ => Ok(()),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_blob(&self) -> Option<&[u8]> {
        match self {
            Self::Blob(b) => Some(b.as_slice()),
            _ => None,
        }
    }
}

impl From<String> for CypherValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for CypherValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_owned())
    }
}

impl From<i64> for CypherValue {
    fn from(n: i64) -> Self {
        Self::Int(n)
    }
}

impl From<f64> for CypherValue {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<bool> for CypherValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<Vec<u8>> for CypherValue {
    fn from(v: Vec<u8>) -> Self {
        Self::Blob(v)
    }
}

impl<T: Into<CypherValue>> From<Vec<T>> for CypherValue {
    fn from(v: Vec<T>) -> Self {
        Self::List(v.into_iter().map(Into::into).collect())
    }
}

/// A named query parameter.
#[derive(Debug, Clone)]
pub struct QueryParam {
    pub name: String,
    pub value: CypherValue,
}

impl QueryParam {
    pub fn new(name: impl Into<String>, value: impl Into<CypherValue>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// The result of a database query.
#[derive(Debug, Clone, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<CypherValue>>,
}

impl QueryResult {
    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Trait for database access via Cypher/SQL queries.
///
/// Synchronous — all backends (rag3db, PostgreSQL) are sync under the hood.
/// Implementations must be `Send + Sync`.
pub trait DbConnection: Send + Sync {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError>;

    fn execute_with_params(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError>;

    /// **Cette base doit-elle être rouverte ?** `Some(message du moteur)`
    /// une fois qu'un point de reprise y a échoué : la connexion refuse
    /// alors tout, et son propriétaire doit la lâcher et en rouvrir une.
    fn must_reopen(&self) -> Option<String> {
        None
    }

    /// **Le tampon du moteur** retenu à l'ouverture, et sa source ; `None`
    /// pour un moteur qui n'en a pas, ou une base ouverte par une
    /// configuration fournie. [`describe_buffer_pool`] le dit en clair.
    fn buffer_pool(&self) -> Option<BufferPoolChoice> {
        None
    }

    /// **Rendre la base inutilisable** pour toutes ses connexions, comme
    /// après un point de reprise échoué : [`must_reopen`](Self::must_reopen)
    /// rend ensuite `reason`. Sans effet pour un moteur qui ne le sait pas.
    fn poison(&self, _reason: &str) {}
}

/// D'où vient la taille du tampon du moteur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferPoolSource {
    /// `RAG3DB_BUFFER_POOL_SIZE`, posée par qui lance.
    Environment,
    /// La clé `buffer_pool` du manifeste.
    Manifest,
    /// La règle du produit : la moitié de la mémoire, au plus 8 Gio.
    Rule,
    /// La mémoire du poste ne se lit pas : le moteur prend 80 % de la
    /// mémoire vive.
    EngineDefault,
}

/// La taille retenue pour le tampon du moteur, et sa source. `bytes` vaut
/// `None` quand le moteur choisit lui-même (80 % de la mémoire vive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferPoolChoice {
    pub bytes: Option<u64>,
    pub source: BufferPoolSource,
}

/// Ce qu'on dit du tampon : sa taille et sa source, pour un journal ou un
/// rapport.
pub fn describe_buffer_pool(choice: BufferPoolChoice) -> String {
    let source = match choice.source {
        BufferPoolSource::Environment => "RAG3DB_BUFFER_POOL_SIZE",
        BufferPoolSource::Manifest => "manifeste (buffer_pool)",
        BufferPoolSource::Rule => "règle du produit (la moitié de la mémoire, au plus 8 Gio)",
        BufferPoolSource::EngineDefault => "défaut du moteur",
    };
    match choice.bytes {
        // Sous le Gio, en Mio : un petit tampon ne s'écrit pas « 0.0 Gio ».
        Some(b) if b < 1u64 << 30 => format!("{} Mio, {source}", b >> 20),
        Some(b) => format!("{:.1} Gio, {source}", b as f64 / (1u64 << 30) as f64),
        None => format!("80 % de la mémoire vive, {source}"),
    }
}

/// Alias for backward compat — DbConnection is now sync natively.
pub trait SyncDbConnection: DbConnection {}
impl<T: DbConnection> SyncDbConnection for T {}

// ─── CallbackConnection ──────────────────────────────────────────────────────

/// Type alias for the sync database execute callback.
///
/// `Fn(&str, &[QueryParam])` → `Result<QueryResult, DbError>`.
/// Called for both `execute` (with empty params) and `execute_with_params`.
pub type DbExecuteFn = Box<
    dyn Fn(&str, &[QueryParam]) -> Result<QueryResult, DbError>
        + Send
        + Sync,
>;

/// Database connection backed by a user-provided closure.
///
/// Useful for WASM (bridging to JS via rag3db_wasm.js) or any environment
/// where the database access is provided externally.
///
/// ```ignore
/// use rag3weaver::connection::{CallbackConnection, QueryResult, DbError};
///
/// let conn = CallbackConnection::new(|cypher, params| {
///     // forward to rag3db_wasm.js, HTTP API, etc.
///     Ok(QueryResult::default())
/// });
/// ```
pub struct CallbackConnection {
    execute_fn: DbExecuteFn,
}

impl CallbackConnection {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&str, &[QueryParam]) -> Result<QueryResult, DbError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            execute_fn: Box::new(f),
        }
    }
}

impl DbConnection for CallbackConnection {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        (self.execute_fn)(cypher, &[])
    }

    fn execute_with_params(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        (self.execute_fn)(cypher, params)
    }
}

/// Mock database connection for testing. Returns empty result sets.
#[derive(Debug, Default)]
pub struct MockConnection {
    /// Toute requête contenant ce motif échoue. C'est ce qui permet
    /// d'éprouver un groupe qui rate au milieu des autres.
    echoue_sur: Option<String>,
}

impl MockConnection {
    pub fn new() -> Self {
        Self { echoue_sur: None }
    }

    /// Un mock dont toute requête contenant `motif` rend une erreur.
    pub fn qui_echoue_sur(motif: &str) -> Self {
        Self { echoue_sur: Some(motif.to_string()) }
    }

    fn verifier(&self, cypher: &str) -> Result<(), DbError> {
        match &self.echoue_sur {
            Some(m) if cypher.contains(m.as_str()) => Err(DbError::QueryError(format!(
                "échec simulé : la requête contient « {m} »"
            ))),
            _ => Ok(()),
        }
    }
}

impl DbConnection for MockConnection {
    fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        self.verifier(cypher)?;
        Ok(QueryResult::default())
    }

    fn execute_with_params(
        &self,
        cypher: &str,
        _params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        self.verifier(cypher)?;
        Ok(QueryResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cypher_value_null() {
        let v = CypherValue::Null;
        assert!(v.is_null());
        assert_eq!(v.as_str(), None);
        assert_eq!(v.as_i64(), None);
    }

    #[test]
    fn cypher_value_string() {
        let v = CypherValue::from("hello");
        assert_eq!(v.as_str(), Some("hello"));
        assert!(!v.is_null());
    }

    #[test]
    fn cypher_value_int() {
        let v = CypherValue::from(42_i64);
        assert_eq!(v.as_i64(), Some(42));
        assert_eq!(v.as_f64(), Some(42.0));
    }

    #[test]
    fn cypher_value_float() {
        let v = CypherValue::from(3.14_f64);
        assert_eq!(v.as_f64(), Some(3.14));
        assert_eq!(v.as_i64(), None);
    }

    #[test]
    fn cypher_value_bool() {
        let v = CypherValue::from(true);
        assert_eq!(v.as_bool(), Some(true));
    }

    #[test]
    fn cypher_value_list() {
        let v = CypherValue::from(vec![CypherValue::from(1_i64), CypherValue::from(2_i64)]);
        match &v {
            CypherValue::List(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].as_i64(), Some(1));
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn cypher_value_serde_roundtrip() {
        let values = vec![
            CypherValue::Null,
            CypherValue::from(true),
            CypherValue::from(42_i64),
            CypherValue::from(3.14_f64),
            CypherValue::from("hello"),
            CypherValue::from(vec![CypherValue::from(1_i64)]),
        ];

        for val in &values {
            let json = serde_json::to_string(val).unwrap();
            let parsed: CypherValue = serde_json::from_str(&json).unwrap();
            assert_eq!(*val, parsed, "roundtrip failed for {json}");
        }
    }

    #[test]
    fn cypher_value_map() {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), CypherValue::from("Alice"));
        map.insert("age".to_string(), CypherValue::from(30_i64));
        let v = CypherValue::Map(map);

        let json = serde_json::to_string(&v).unwrap();
        let parsed: CypherValue = serde_json::from_str(&json).unwrap();
        assert_eq!(v, parsed);
    }

    #[test]
    fn query_param_new() {
        let p = QueryParam::new("name", "Alice");
        assert_eq!(p.name, "name");
        assert_eq!(p.value.as_str(), Some("Alice"));
    }

    #[test]
    fn query_result_empty() {
        let qr = QueryResult::default();
        assert!(qr.is_empty());
        assert_eq!(qr.num_rows(), 0);
    }

    #[test]
    fn mock_connection_execute() {
        let conn = MockConnection::new();
        let result = conn.execute("MATCH (n) RETURN n").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn mock_connection_with_params() {
        let conn = MockConnection::new();
        let params = vec![QueryParam::new("id", 42_i64)];
        let result = conn
            .execute_with_params("MATCH (n) WHERE n.id = $id RETURN n", &params)
            .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn connection_as_trait_object() {
        let conn: Box<dyn DbConnection> = Box::new(MockConnection::new());
        let result = conn.execute("RETURN 1").unwrap();
        assert!(result.is_empty());
    }

    // ── CallbackConnection ────────────────────────────────────────────────

    #[test]
    fn callback_connection_execute() {
        let conn = CallbackConnection::new(|cypher, _params| {
            Ok(QueryResult {
                columns: vec!["query".to_string()],
                rows: vec![vec![CypherValue::String(cypher.to_string())]],
            })
        });

        let result = conn.execute("RETURN 1").unwrap();
        assert_eq!(result.num_rows(), 1);
        assert_eq!(result.rows[0][0].as_str(), Some("RETURN 1"));
    }

    #[test]
    fn callback_connection_with_params() {
        let conn = CallbackConnection::new(|cypher, params| {
            Ok(QueryResult {
                columns: vec!["cypher".to_string(), "param_count".to_string()],
                rows: vec![vec![
                    CypherValue::String(cypher.to_string()),
                    CypherValue::Int(params.len() as i64),
                ]],
            })
        });

        let params = vec![
            QueryParam::new("name", "Alice"),
            QueryParam::new("age", 30_i64),
        ];
        let result = conn
            .execute_with_params("MATCH (n) WHERE n.name = $name", &params)
            .unwrap();
        assert_eq!(result.rows[0][1].as_i64(), Some(2));
    }

    #[test]
    fn callback_connection_error() {
        let conn = CallbackConnection::new(|_cypher, _params| {
            Err(DbError::QueryError("simulated error".into()))
        });

        let err = conn.execute("BAD QUERY").unwrap_err();
        assert!(matches!(err, DbError::QueryError(_)));
    }

    #[test]
    fn callback_connection_as_trait_object() {
        let conn: Box<dyn DbConnection> = Box::new(CallbackConnection::new(
            |_cypher, _params| Ok(QueryResult::default()),
        ));
        let result = conn.execute("RETURN 1").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn db_error_display() {
        assert_eq!(
            DbError::QueryError("syntax error".into()).to_string(),
            "query error: syntax error"
        );
        assert_eq!(
            DbError::ConnectionError("timeout".into()).to_string(),
            "connection error: timeout"
        );
        assert_eq!(
            DbError::TypeError("expected INT64".into()).to_string(),
            "type error: expected INT64"
        );
    }
}

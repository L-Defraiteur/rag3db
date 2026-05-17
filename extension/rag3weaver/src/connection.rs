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
}

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
}

impl CypherValue {
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
pub struct MockConnection;

impl MockConnection {
    pub fn new() -> Self {
        Self
    }
}

impl DbConnection for MockConnection {
    fn execute(&self, _cypher: &str) -> Result<QueryResult, DbError> {
        Ok(QueryResult::default())
    }

    fn execute_with_params(
        &self,
        _cypher: &str,
        _params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
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

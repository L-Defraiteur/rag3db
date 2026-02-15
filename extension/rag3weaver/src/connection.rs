//! Database connection trait and value types.
//!
//! The [`DbConnection`] trait abstracts over the rag3db database connection,
//! allowing the pipeline to execute Cypher queries without depending on
//! the concrete database implementation.

use std::collections::HashMap;

use async_trait::async_trait;
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
    Map(HashMap<String, CypherValue>),
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

/// Trait for database access via Cypher queries.
///
/// Implementations must be `Send + Sync` for use across async tasks.
#[async_trait]
pub trait DbConnection: Send + Sync {
    async fn execute(&self, cypher: &str) -> Result<QueryResult, DbError>;

    async fn execute_with_params(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError>;
}

/// Mock database connection for testing. Returns empty result sets.
#[derive(Debug, Default)]
pub struct MockConnection;

impl MockConnection {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DbConnection for MockConnection {
    async fn execute(&self, _cypher: &str) -> Result<QueryResult, DbError> {
        Ok(QueryResult::default())
    }

    async fn execute_with_params(
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
        let mut map = HashMap::new();
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

    #[tokio::test]
    async fn mock_connection_execute() {
        let conn = MockConnection::new();
        let result = conn.execute("MATCH (n) RETURN n").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn mock_connection_with_params() {
        let conn = MockConnection::new();
        let params = vec![QueryParam::new("id", 42_i64)];
        let result = conn
            .execute_with_params("MATCH (n) WHERE n.id = $id RETURN n", &params)
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn connection_as_trait_object() {
        let conn: Box<dyn DbConnection> = Box::new(MockConnection::new());
        let result = conn.execute("RETURN 1").await.unwrap();
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

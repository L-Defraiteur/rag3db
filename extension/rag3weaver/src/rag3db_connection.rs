//! Native rag3db connection (feature: `rag3db-native`).
//!
//! Provides [`Rag3dbConnection`] that implements [`DbConnection`] by embedding
//! the rag3db C++ engine in-process via the official Rust crate.

use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;

use crate::connection::{CypherValue, DbConnection, DbError, QueryParam, QueryResult};

/// In-process rag3db connection.
///
/// Owns both the [`Database`](rag3db::Database) and [`Connection`](rag3db::Connection),
/// embedding the full rag3db engine in the current process.
///
/// # Safety
///
/// This struct is self-referential: `conn` borrows from `db`.
/// Fields are declared so that `conn` is dropped before `db` (Rust drops
/// fields in declaration order). The `Database` lives on the heap (`Box`)
/// so its address is stable.
pub struct Rag3dbConnection {
    // SAFETY: conn borrows from db. Declared first so it drops first.
    conn: rag3db::Connection<'static>,
    #[allow(dead_code)]
    db: Box<rag3db::Database>,
}

// rag3db::Connection is already Send+Sync (unsafe impl in the crate).
// Our wrapper inherits these guarantees.
unsafe impl Send for Rag3dbConnection {}
unsafe impl Sync for Rag3dbConnection {}

impl Rag3dbConnection {
    /// Open (or create) a database at the given path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, DbError> {
        Self::with_config(path, rag3db::SystemConfig::default())
    }

    /// Create an in-memory database.
    pub fn in_memory() -> Result<Self, DbError> {
        let db = Box::new(
            rag3db::Database::in_memory(rag3db::SystemConfig::default())
                .map_err(|e| DbError::ConnectionError(e.to_string()))?,
        );
        Self::connect(db)
    }

    /// Open a database with a custom [`SystemConfig`](rag3db::SystemConfig).
    pub fn with_config(path: impl AsRef<Path>, config: rag3db::SystemConfig) -> Result<Self, DbError> {
        let db = Box::new(
            rag3db::Database::new(path, config)
                .map_err(|e| DbError::ConnectionError(e.to_string()))?,
        );
        Self::connect(db)
    }

    fn connect(db: Box<rag3db::Database>) -> Result<Self, DbError> {
        // SAFETY: db is heap-allocated (Box), address is stable.
        // conn is declared before db in the struct, so it drops first.
        // We never expose the inner Database or Connection separately.
        let db_ptr = &*db as *const rag3db::Database;
        let conn = unsafe {
            let db_ref = &*db_ptr;
            let conn = rag3db::Connection::new(db_ref)
                .map_err(|e| DbError::ConnectionError(e.to_string()))?;
            std::mem::transmute::<rag3db::Connection<'_>, rag3db::Connection<'static>>(conn)
        };
        Ok(Self { conn, db })
    }

    /// Execute a raw Cypher query (sync, used internally).
    fn query_sync(&self, cypher: &str) -> Result<QueryResult, DbError> {
        let mut result = self
            .conn
            .query(cypher)
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let columns = result.get_column_names();
        let mut rows = Vec::new();
        for row in &mut result {
            rows.push(row.into_iter().map(rag3db_value_to_cypher).collect());
        }

        Ok(QueryResult { columns, rows })
    }

    /// Execute a parameterized Cypher query (sync, used internally).
    fn query_with_params_sync(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        let mut stmt = self
            .conn
            .prepare(cypher)
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let rag3db_params: Vec<(&str, rag3db::Value)> = params
            .iter()
            .map(|p| (p.name.as_str(), cypher_to_rag3db_value(&p.value)))
            .collect();

        let mut result = self
            .conn
            .execute(&mut stmt, rag3db_params)
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        let columns = result.get_column_names();
        let mut rows = Vec::new();
        for row in &mut result {
            rows.push(row.into_iter().map(rag3db_value_to_cypher).collect());
        }

        Ok(QueryResult { columns, rows })
    }
}

#[async_trait]
impl DbConnection for Rag3dbConnection {
    async fn execute(&self, cypher: &str) -> Result<QueryResult, DbError> {
        self.query_sync(cypher)
    }

    async fn execute_with_params(
        &self,
        cypher: &str,
        params: &[QueryParam],
    ) -> Result<QueryResult, DbError> {
        self.query_with_params_sync(cypher, params)
    }
}

// ── Value conversions ──────────────────────────────────────────────────

/// Convert a rag3db `Value` to our `CypherValue`.
fn rag3db_value_to_cypher(value: rag3db::Value) -> CypherValue {
    match value {
        rag3db::Value::Null(_) => CypherValue::Null,
        rag3db::Value::Bool(b) => CypherValue::Bool(b),
        rag3db::Value::Int64(i) => CypherValue::Int(i),
        rag3db::Value::Int32(i) => CypherValue::Int(i as i64),
        rag3db::Value::Int16(i) => CypherValue::Int(i as i64),
        rag3db::Value::Int8(i) => CypherValue::Int(i as i64),
        rag3db::Value::UInt64(u) => CypherValue::Int(u as i64),
        rag3db::Value::UInt32(u) => CypherValue::Int(u as i64),
        rag3db::Value::UInt16(u) => CypherValue::Int(u as i64),
        rag3db::Value::UInt8(u) => CypherValue::Int(u as i64),
        rag3db::Value::Int128(i) => CypherValue::Int(i as i64),
        rag3db::Value::Double(f) => CypherValue::Float(f),
        rag3db::Value::Float(f) => CypherValue::Float(f as f64),
        rag3db::Value::String(s) => CypherValue::String(s),
        rag3db::Value::List(_, vs) | rag3db::Value::Array(_, vs) => {
            CypherValue::List(vs.into_iter().map(rag3db_value_to_cypher).collect())
        }
        rag3db::Value::Node(n) => {
            let mut map = HashMap::new();
            map.insert(
                "_label".to_string(),
                CypherValue::String(n.get_label_name().clone()),
            );
            let id = n.get_node_id();
            map.insert(
                "_id".to_string(),
                CypherValue::String(format!("{}:{}", id.table_id, id.offset)),
            );
            for (key, val) in n.get_properties().iter() {
                map.insert(key.clone(), rag3db_value_to_cypher(val.clone()));
            }
            CypherValue::Map(map)
        }
        rag3db::Value::Rel(r) => {
            let mut map = HashMap::new();
            map.insert(
                "_label".to_string(),
                CypherValue::String(r.get_label_name().clone()),
            );
            let src = r.get_src_node();
            let dst = r.get_dst_node();
            map.insert(
                "_src".to_string(),
                CypherValue::String(format!("{}:{}", src.table_id, src.offset)),
            );
            map.insert(
                "_dst".to_string(),
                CypherValue::String(format!("{}:{}", dst.table_id, dst.offset)),
            );
            for (key, val) in r.get_properties().iter() {
                map.insert(key.clone(), rag3db_value_to_cypher(val.clone()));
            }
            CypherValue::Map(map)
        }
        rag3db::Value::Struct(fields) => {
            let mut map = HashMap::new();
            for (key, val) in fields {
                map.insert(key, rag3db_value_to_cypher(val));
            }
            CypherValue::Map(map)
        }
        rag3db::Value::Map(_, pairs) => {
            let mut map = HashMap::new();
            for (k, v) in pairs {
                let key = match k {
                    rag3db::Value::String(s) => s,
                    other => format!("{other}"),
                };
                map.insert(key, rag3db_value_to_cypher(v));
            }
            CypherValue::Map(map)
        }
        // Fallback: Date, Timestamp, Interval, Blob, UUID, Decimal, etc.
        other => CypherValue::String(format!("{other}")),
    }
}

/// Convert our `CypherValue` to a rag3db `Value` (for prepared statement params).
fn cypher_to_rag3db_value(value: &CypherValue) -> rag3db::Value {
    match value {
        CypherValue::Null => rag3db::Value::Null(rag3db::LogicalType::String),
        CypherValue::Bool(b) => rag3db::Value::Bool(*b),
        CypherValue::Int(i) => rag3db::Value::Int64(*i),
        CypherValue::Float(f) => rag3db::Value::Double(*f),
        CypherValue::String(s) => rag3db::Value::String(s.clone()),
        CypherValue::List(vs) => {
            let converted: Vec<rag3db::Value> = vs.iter().map(cypher_to_rag3db_value).collect();
            let elem_type = converted
                .first()
                .map(rag3db::LogicalType::from)
                .unwrap_or(rag3db::LogicalType::String);
            rag3db::Value::List(elem_type, converted)
        }
        CypherValue::Map(m) => {
            let fields: Vec<(String, rag3db::Value)> = m
                .iter()
                .map(|(k, v)| (k.clone(), cypher_to_rag3db_value(v)))
                .collect();
            rag3db::Value::Struct(fields)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Unit tests (no DB needed) ──────────────────────────────────────

    #[test]
    fn value_mapping_int_types() {
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Int64(42)),
            CypherValue::Int(42)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Int32(7)),
            CypherValue::Int(7)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Int16(-1)),
            CypherValue::Int(-1)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Int8(3)),
            CypherValue::Int(3)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::UInt64(100)),
            CypherValue::Int(100)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::UInt8(255)),
            CypherValue::Int(255)
        );
    }

    #[test]
    fn value_mapping_float_types() {
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Double(3.14)),
            CypherValue::Float(3.14)
        );
        // Float(1.5) → Float(1.5 as f64)
        let result = rag3db_value_to_cypher(rag3db::Value::Float(1.5));
        assert_eq!(result, CypherValue::Float(1.5));
    }

    #[test]
    fn value_mapping_string_bool_null() {
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::String("hello".into())),
            CypherValue::String("hello".into())
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Bool(true)),
            CypherValue::Bool(true)
        );
        assert_eq!(
            rag3db_value_to_cypher(rag3db::Value::Null(rag3db::LogicalType::String)),
            CypherValue::Null
        );
    }

    #[test]
    fn value_mapping_list() {
        let list = rag3db::Value::List(
            rag3db::LogicalType::Int64,
            vec![rag3db::Value::Int64(1), rag3db::Value::Int64(2)],
        );
        assert_eq!(
            rag3db_value_to_cypher(list),
            CypherValue::List(vec![CypherValue::Int(1), CypherValue::Int(2)])
        );
    }

    #[test]
    fn value_mapping_node() {
        let mut node = rag3db::NodeVal::new((0u64, 0u64), "Person");
        node.add_property("name", rag3db::Value::String("Alice".into()));
        let result = rag3db_value_to_cypher(rag3db::Value::Node(node));

        if let CypherValue::Map(map) = &result {
            assert_eq!(map["_label"], CypherValue::String("Person".into()));
            assert_eq!(map["_id"], CypherValue::String("0:0".into()));
            assert_eq!(map["name"], CypherValue::String("Alice".into()));
        } else {
            panic!("expected Map, got {result:?}");
        }
    }

    #[test]
    fn value_mapping_struct() {
        let s = rag3db::Value::Struct(vec![
            ("key".into(), rag3db::Value::String("val".into())),
            ("num".into(), rag3db::Value::Int64(42)),
        ]);
        let result = rag3db_value_to_cypher(s);
        if let CypherValue::Map(map) = &result {
            assert_eq!(map["key"], CypherValue::String("val".into()));
            assert_eq!(map["num"], CypherValue::Int(42));
        } else {
            panic!("expected Map, got {result:?}");
        }
    }

    #[test]
    fn cypher_to_rag3db_roundtrip() {
        let cypher_val = CypherValue::Int(42);
        let rag3db_val = cypher_to_rag3db_value(&cypher_val);
        assert_eq!(rag3db_val, rag3db::Value::Int64(42));

        let cypher_val = CypherValue::String("test".into());
        let rag3db_val = cypher_to_rag3db_value(&cypher_val);
        assert_eq!(rag3db_val, rag3db::Value::String("test".into()));

        let cypher_val = CypherValue::Float(2.71);
        let rag3db_val = cypher_to_rag3db_value(&cypher_val);
        assert_eq!(rag3db_val, rag3db::Value::Double(2.71));

        let cypher_val = CypherValue::Bool(false);
        let rag3db_val = cypher_to_rag3db_value(&cypher_val);
        assert_eq!(rag3db_val, rag3db::Value::Bool(false));

        let cypher_val = CypherValue::Null;
        let rag3db_val = cypher_to_rag3db_value(&cypher_val);
        assert_eq!(rag3db_val, rag3db::Value::Null(rag3db::LogicalType::String));
    }

    // ── Integration tests (require rag3db build) ───────────────────────

    #[tokio::test]
    #[ignore]
    async fn in_memory_create_and_query() {
        let conn = Rag3dbConnection::in_memory().unwrap();

        conn.execute("CREATE NODE TABLE Person(name STRING, age INT64, PRIMARY KEY(name));")
            .await
            .unwrap();
        conn.execute("CREATE (:Person {name: 'Alice', age: 25});")
            .await
            .unwrap();
        conn.execute("CREATE (:Person {name: 'Bob', age: 30});")
            .await
            .unwrap();

        let result = conn
            .execute("MATCH (p:Person) RETURN p.name AS name, p.age AS age ORDER BY p.name;")
            .await
            .unwrap();

        assert_eq!(result.columns, vec!["name", "age"]);
        assert_eq!(result.num_rows(), 2);
        assert_eq!(result.rows[0][0], CypherValue::String("Alice".into()));
        assert_eq!(result.rows[0][1], CypherValue::Int(25));
        assert_eq!(result.rows[1][0], CypherValue::String("Bob".into()));
        assert_eq!(result.rows[1][1], CypherValue::Int(30));
    }

    #[tokio::test]
    #[ignore]
    async fn execute_with_params() {
        let conn = Rag3dbConnection::in_memory().unwrap();

        conn.execute("CREATE NODE TABLE Item(id INT64, label STRING, PRIMARY KEY(id));")
            .await
            .unwrap();

        let params = vec![
            QueryParam::new("id", 1_i64),
            QueryParam::new("label", "first"),
        ];
        conn.execute_with_params(
            "CREATE (:Item {id: $id, label: $label});",
            &params,
        )
        .await
        .unwrap();

        let params = vec![
            QueryParam::new("id", 2_i64),
            QueryParam::new("label", "second"),
        ];
        conn.execute_with_params(
            "CREATE (:Item {id: $id, label: $label});",
            &params,
        )
        .await
        .unwrap();

        let result = conn
            .execute("MATCH (i:Item) RETURN i.id, i.label ORDER BY i.id;")
            .await
            .unwrap();

        assert_eq!(result.num_rows(), 2);
        assert_eq!(result.rows[0][0], CypherValue::Int(1));
        assert_eq!(result.rows[0][1], CypherValue::String("first".into()));
        assert_eq!(result.rows[1][0], CypherValue::Int(2));
        assert_eq!(result.rows[1][1], CypherValue::String("second".into()));
    }

    #[tokio::test]
    #[ignore]
    async fn query_returns_node_as_map() {
        let conn = Rag3dbConnection::in_memory().unwrap();

        conn.execute("CREATE NODE TABLE Person(name STRING, age INT64, PRIMARY KEY(name));")
            .await
            .unwrap();
        conn.execute("CREATE (:Person {name: 'Alice', age: 25});")
            .await
            .unwrap();

        let result = conn
            .execute("MATCH (p:Person) RETURN p;")
            .await
            .unwrap();

        assert_eq!(result.num_rows(), 1);
        if let CypherValue::Map(map) = &result.rows[0][0] {
            assert_eq!(map["_label"], CypherValue::String("Person".into()));
            assert_eq!(map["name"], CypherValue::String("Alice".into()));
            assert_eq!(map["age"], CypherValue::Int(25));
            assert!(map.contains_key("_id"));
        } else {
            panic!("expected Map for node, got {:?}", result.rows[0][0]);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn query_returns_rel_as_map() {
        let conn = Rag3dbConnection::in_memory().unwrap();

        conn.execute("CREATE NODE TABLE Person(name STRING, PRIMARY KEY(name));")
            .await
            .unwrap();
        conn.execute("CREATE REL TABLE knows(FROM Person TO Person, since INT64);")
            .await
            .unwrap();
        conn.execute("CREATE (:Person {name: 'Alice'});")
            .await
            .unwrap();
        conn.execute("CREATE (:Person {name: 'Bob'});")
            .await
            .unwrap();
        conn.execute(
            "MATCH (a:Person), (b:Person) WHERE a.name='Alice' AND b.name='Bob' CREATE (a)-[:knows {since: 2020}]->(b);",
        )
        .await
        .unwrap();

        let result = conn
            .execute("MATCH (a)-[r:knows]->(b) RETURN r;")
            .await
            .unwrap();

        assert_eq!(result.num_rows(), 1);
        if let CypherValue::Map(map) = &result.rows[0][0] {
            assert_eq!(map["_label"], CypherValue::String("knows".into()));
            assert_eq!(map["since"], CypherValue::Int(2020));
            assert!(map.contains_key("_src"));
            assert!(map.contains_key("_dst"));
        } else {
            panic!("expected Map for rel, got {:?}", result.rows[0][0]);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn as_trait_object() {
        let conn: Box<dyn DbConnection> = Box::new(Rag3dbConnection::in_memory().unwrap());
        conn.execute("CREATE NODE TABLE T(id INT64, PRIMARY KEY(id));")
            .await
            .unwrap();
        conn.execute("CREATE (:T {id: 1});").await.unwrap();

        let result = conn
            .execute("MATCH (t:T) RETURN t.id;")
            .await
            .unwrap();
        assert_eq!(result.num_rows(), 1);
        assert_eq!(result.rows[0][0], CypherValue::Int(1));
    }

    #[tokio::test]
    #[ignore]
    async fn error_on_invalid_query() {
        let conn = Rag3dbConnection::in_memory().unwrap();
        let result = conn.execute("INVALID CYPHER SYNTAX!!!").await;
        assert!(result.is_err());
    }
}

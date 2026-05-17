//! Migration nodes: CypherNode + ValidateNode.
//!
//! Building blocks for schema migrations, also usable in general-purpose dataflow graphs.
//!
//! - [`CypherNode`] — executes a Cypher query, optionally captures undo context
//! - [`ValidateNode`] — asserts a condition on query results, fails the graph if violated

use std::sync::Arc;


use crate::connection::{CypherValue, DbConnection};

use super::node::{Node, NodeContext};
use super::node_registry::{ConfigParam, ConfigParamType, NodeFactory, NodeSchema};
use super::port::{PortDef, PortType, PortValue};

// ─── CypherNode ─────────────────────────────────────────────────────────────

/// Executes a Cypher query, optionally capturing undo context for rollback.
///
/// If `capture_query` is set, it is executed BEFORE the mutation query.
/// The result is stored as the undo context (serialized JSON rows).
pub struct CypherNode {
    name: String,
    query: String,
    capture_query: Option<String>,
    undo_data: Option<serde_json::Value>,
}

impl CypherNode {
    pub fn new(name: &str, query: String, capture_query: Option<String>) -> Self {
        Self {
            name: name.to_string(),
            query,
            capture_query,
            undo_data: None,
        }
    }
}

impl std::fmt::Debug for CypherNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CypherNode")
            .field("name", &self.name)
            .field("query", &self.query)
            .finish()
    }
}


impl Node for CypherNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn inputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "trigger",
            port_type: PortType::Empty,
            required: false,
        }]
    }

    fn outputs(&self) -> &[PortDef] {
        &[
            PortDef {
                name: "result",
                port_type: PortType::Map,
                required: false,
            },
            PortDef {
                name: "done",
                port_type: PortType::Empty,
                required: false,
            },
        ]
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let conn = ctx
            .service::<Arc<dyn DbConnection>>("conn")
            .ok_or("CypherNode: 'conn' service not registered")?;

        // Capture undo data before mutation
        if let Some(ref capture) = self.capture_query {
            let capture_result = conn
                .execute(capture)
                .map_err(|e| format!("CypherNode capture query failed: {e}"))?;

            let rows: Vec<serde_json::Value> = capture_result
                .rows
                .iter()
                .map(|row| {
                    let obj: serde_json::Map<String, serde_json::Value> = capture_result
                        .columns
                        .iter()
                        .zip(row.iter())
                        .map(|(col, val)| (col.clone(), cypher_value_to_json(val)))
                        .collect();
                    serde_json::Value::Object(obj)
                })
                .collect();

            self.undo_data = Some(serde_json::json!({
                "query": self.query,
                "captured_rows": rows,
            }));
        }

        // Execute the mutation
        let result = conn
            .execute(&self.query)
            .map_err(|e| format!("CypherNode query failed: {e}"))?;

        // Emit results as Map
        let rows: Vec<serde_json::Value> = result
            .rows
            .iter()
            .map(|row| {
                let obj: serde_json::Map<String, serde_json::Value> = result
                    .columns
                    .iter()
                    .zip(row.iter())
                    .map(|(col, val)| (col.clone(), cypher_value_to_json(val)))
                    .collect();
                serde_json::Value::Object(obj)
            })
            .collect();

        ctx.set_output("result", PortValue::Map(serde_json::json!(rows)));
        ctx.set_output("done", PortValue::Empty);
        ctx.log_metric("rows_affected", rows.len());

        Ok(())
    }

    fn node_type(&self) -> &'static str {
        "CypherNode"
    }

    fn node_config(&self) -> serde_json::Value {
        let mut config = serde_json::json!({ "query": self.query });
        if let Some(ref capture) = self.capture_query {
            config["capture"] = serde_json::Value::String(capture.clone());
        }
        config
    }

    fn can_undo(&self) -> bool {
        self.capture_query.is_some()
    }

    fn undo_context(&self) -> Option<serde_json::Value> {
        self.undo_data.clone()
    }

    fn undo(
        &mut self,
        ctx: &mut NodeContext,
        undo_ctx: serde_json::Value,
    ) -> Result<(), String> {
        let conn = ctx
            .service::<Arc<dyn DbConnection>>("conn")
            .ok_or("CypherNode undo: 'conn' service not registered")?;

        let rows = undo_ctx["captured_rows"]
            .as_array()
            .ok_or("CypherNode undo: missing captured_rows")?;

        // For each captured row, restore the old values
        for row in rows {
            let uuid = row["_uuid"]
                .as_str()
                .ok_or("CypherNode undo: captured row missing _uuid")?;

            // Build SET clauses from captured columns (skip _uuid)
            let obj = row
                .as_object()
                .ok_or("CypherNode undo: row is not an object")?;
            let mut set_parts = Vec::new();
            for (key, val) in obj {
                if key == "_uuid" {
                    continue;
                }
                let val_str = match val {
                    serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "\\'")),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "NULL".to_string(),
                    other => format!("'{}'", other),
                };
                set_parts.push(format!("n.{key} = {val_str}"));
            }

            if !set_parts.is_empty() {
                let undo_query = format!(
                    "MATCH (n {{_uuid: '{}'}}) SET {}",
                    uuid,
                    set_parts.join(", ")
                );
                conn.execute(&undo_query)
                    .map_err(|e| format!("CypherNode undo failed: {e}"))?;
            }
        }

        Ok(())
    }
}

// ─── CypherNodeFactory ──────────────────────────────────────────────────────

pub struct CypherNodeFactory;

impl NodeFactory for CypherNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn Node>, String> {
        let query = config["query"]
            .as_str()
            .ok_or("CypherNode: missing 'query' config")?
            .to_string();
        let capture = config
            .get("capture")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Box::new(CypherNode::new(name, query, capture)))
    }

    fn node_type(&self) -> &'static str {
        "CypherNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "CypherNode",
            description: "Execute a Cypher query with optional undo capture",
            inputs: vec![PortDef {
                name: "trigger",
                port_type: PortType::Empty,
                required: false,
            }],
            outputs: vec![
                PortDef {
                    name: "result",
                    port_type: PortType::Map,
                    required: false,
                },
                PortDef {
                    name: "done",
                    port_type: PortType::Empty,
                    required: false,
                },
            ],
            config_params: vec![
                ConfigParam {
                    name: "query",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Cypher query to execute",
                },
                ConfigParam {
                    name: "capture",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: None,
                    description: "Cypher query to capture undo data before mutation",
                },
            ],
        }
    }
}

// ─── ValidateNode ───────────────────────────────────────────────────────────

/// Validates a condition on query results. Fails the graph if the assertion is violated.
pub struct ValidateNode {
    name: String,
    query: String,
    assertion: Assertion,
    message: String,
}

/// Assertion to check on query results.
#[derive(Debug, Clone)]
pub enum Assertion {
    /// Result row count == N
    CountEquals(i64),
    /// Result row count > N
    CountGt(i64),
    /// Result row count < N
    CountLt(i64),
    /// No rows returned
    IsEmpty,
    /// At least one row returned
    IsNotEmpty,
    /// Expression on first row column value: "column_name > N" or "column_name == N"
    Expression {
        column: String,
        op: AssertOp,
        value: i64,
    },
}

#[derive(Debug, Clone)]
pub enum AssertOp {
    Eq,
    Gt,
    Lt,
    Gte,
    Lte,
}

impl Assertion {
    /// Parse an assertion from a config string.
    ///
    /// Formats:
    /// - `"empty"` → IsEmpty
    /// - `"not_empty"` → IsNotEmpty
    /// - `"count == 5"` → CountEquals(5)
    /// - `"count > 0"` → CountGt(0)
    /// - `"count < 100"` → CountLt(100)
    /// - `"cnt > 0"` → Expression { column: "cnt", op: Gt, value: 0 }
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("empty") {
            return Ok(Self::IsEmpty);
        }
        if s.eq_ignore_ascii_case("not_empty") {
            return Ok(Self::IsNotEmpty);
        }

        // Parse "column op value" format
        let parts: Vec<&str> = s.splitn(3, ' ').collect();
        if parts.len() != 3 {
            return Err(format!("invalid assertion format: '{s}' — expected 'column op value'"));
        }

        let column = parts[0];
        let op_str = parts[1];
        let value: i64 = parts[2]
            .parse()
            .map_err(|_| format!("invalid assertion value: '{}' — expected integer", parts[2]))?;

        let op = match op_str {
            "==" => AssertOp::Eq,
            ">" => AssertOp::Gt,
            "<" => AssertOp::Lt,
            ">=" => AssertOp::Gte,
            "<=" => AssertOp::Lte,
            _ => return Err(format!("invalid assertion op: '{op_str}' — expected ==, >, <, >=, <=")),
        };

        if column == "count" {
            return match op {
                AssertOp::Eq => Ok(Self::CountEquals(value)),
                AssertOp::Gt => Ok(Self::CountGt(value)),
                AssertOp::Lt => Ok(Self::CountLt(value)),
                _ => Ok(Self::Expression {
                    column: column.to_string(),
                    op,
                    value,
                }),
            };
        }

        Ok(Self::Expression {
            column: column.to_string(),
            op,
            value,
        })
    }

    /// Check the assertion against a row count and optional first-row column value.
    fn check(&self, row_count: usize, first_row_value: Option<i64>) -> Result<(), String> {
        match self {
            Self::IsEmpty => {
                if row_count != 0 {
                    Err(format!("expected empty result, got {row_count} rows"))
                } else {
                    Ok(())
                }
            }
            Self::IsNotEmpty => {
                if row_count == 0 {
                    Err("expected non-empty result, got 0 rows".into())
                } else {
                    Ok(())
                }
            }
            Self::CountEquals(n) => {
                if row_count as i64 != *n {
                    Err(format!("expected count == {n}, got {row_count}"))
                } else {
                    Ok(())
                }
            }
            Self::CountGt(n) => {
                if (row_count as i64) <= *n {
                    Err(format!("expected count > {n}, got {row_count}"))
                } else {
                    Ok(())
                }
            }
            Self::CountLt(n) => {
                if (row_count as i64) >= *n {
                    Err(format!("expected count < {n}, got {row_count}"))
                } else {
                    Ok(())
                }
            }
            Self::Expression { column, op, value } => {
                let actual = first_row_value.ok_or_else(|| {
                    format!("assertion on '{column}': no value in first row")
                })?;
                let pass = match op {
                    AssertOp::Eq => actual == *value,
                    AssertOp::Gt => actual > *value,
                    AssertOp::Lt => actual < *value,
                    AssertOp::Gte => actual >= *value,
                    AssertOp::Lte => actual <= *value,
                };
                if !pass {
                    Err(format!(
                        "assertion failed: {column} ({actual}) {op} {value}",
                        op = match op {
                            AssertOp::Eq => "==",
                            AssertOp::Gt => ">",
                            AssertOp::Lt => "<",
                            AssertOp::Gte => ">=",
                            AssertOp::Lte => "<=",
                        }
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl ValidateNode {
    pub fn new(name: &str, query: String, assertion: Assertion, message: String) -> Self {
        Self {
            name: name.to_string(),
            query,
            assertion,
            message,
        }
    }
}

impl std::fmt::Debug for ValidateNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidateNode")
            .field("name", &self.name)
            .field("query", &self.query)
            .finish()
    }
}


impl Node for ValidateNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn inputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "trigger",
            port_type: PortType::Empty,
            required: false,
        }]
    }

    fn outputs(&self) -> &[PortDef] {
        &[PortDef {
            name: "done",
            port_type: PortType::Empty,
            required: false,
        }]
    }

    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        let conn = ctx
            .service::<Arc<dyn DbConnection>>("conn")
            .ok_or("ValidateNode: 'conn' service not registered")?;

        let result = conn
            .execute(&self.query)
            .map_err(|e| format!("ValidateNode query failed: {e}"))?;

        // Extract first-row column value for expression assertions
        let first_row_value = if let Some(row) = result.rows.first() {
            // Find the column referenced in the assertion
            if let Assertion::Expression { ref column, .. } = self.assertion {
                let col_idx = result
                    .columns
                    .iter()
                    .position(|c| c == column);
                col_idx.and_then(|i| row.get(i)).and_then(|v| v.as_i64())
            } else {
                None
            }
        } else {
            None
        };

        self.assertion
            .check(result.rows.len(), first_row_value)
            .map_err(|e| format!("{}: {e}", self.message))?;

        ctx.set_output("done", PortValue::Empty);
        ctx.log_metric("validated", true);

        Ok(())
    }

    fn node_type(&self) -> &'static str {
        "ValidateNode"
    }

    fn node_config(&self) -> serde_json::Value {
        serde_json::json!({
            "query": self.query,
            "assert": format!("{:?}", self.assertion),
            "message": self.message,
        })
    }

    // ValidateNode is read-only — no undo needed
}

// ─── ValidateNodeFactory ────────────────────────────────────────────────────

pub struct ValidateNodeFactory;

impl NodeFactory for ValidateNodeFactory {
    fn create(
        &self,
        name: &str,
        config: &serde_json::Value,
    ) -> Result<Box<dyn Node>, String> {
        let query = config["query"]
            .as_str()
            .ok_or("ValidateNode: missing 'query' config")?
            .to_string();
        let assert_str = config["assert"]
            .as_str()
            .ok_or("ValidateNode: missing 'assert' config")?;
        let assertion = Assertion::parse(assert_str)?;
        let message = config
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("validation failed")
            .to_string();

        Ok(Box::new(ValidateNode::new(name, query, assertion, message)))
    }

    fn node_type(&self) -> &'static str {
        "ValidateNode"
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            node_type: "ValidateNode",
            description: "Assert a condition on Cypher query results",
            inputs: vec![PortDef {
                name: "trigger",
                port_type: PortType::Empty,
                required: false,
            }],
            outputs: vec![PortDef {
                name: "done",
                port_type: PortType::Empty,
                required: false,
            }],
            config_params: vec![
                ConfigParam {
                    name: "query",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Cypher query to execute for validation",
                },
                ConfigParam {
                    name: "assert",
                    param_type: ConfigParamType::String,
                    required: true,
                    default: None,
                    description: "Assertion: 'empty', 'not_empty', 'count == N', 'count > N', 'column > N'",
                },
                ConfigParam {
                    name: "message",
                    param_type: ConfigParamType::String,
                    required: false,
                    default: Some(serde_json::json!("validation failed")),
                    description: "Error message when assertion fails",
                },
            ],
        }
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn cypher_value_to_json(val: &CypherValue) -> serde_json::Value {
    match val {
        CypherValue::String(s) => serde_json::Value::String(s.clone()),
        CypherValue::Int(n) => serde_json::json!(n),
        CypherValue::Float(f) => serde_json::json!(f),
        CypherValue::Bool(b) => serde_json::json!(b),
        CypherValue::Null => serde_json::Value::Null,
        CypherValue::List(items) => {
            serde_json::Value::Array(items.iter().map(cypher_value_to_json).collect())
        }
        CypherValue::Map(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), cypher_value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        CypherValue::Blob(_) => serde_json::Value::String("<blob>".to_string()),
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Assertion parse tests ────────────────────────────────────────────

    #[test]
    fn parse_empty() {
        let a = Assertion::parse("empty").unwrap();
        assert!(matches!(a, Assertion::IsEmpty));
    }

    #[test]
    fn parse_not_empty() {
        let a = Assertion::parse("not_empty").unwrap();
        assert!(matches!(a, Assertion::IsNotEmpty));
    }

    #[test]
    fn parse_count_equals() {
        let a = Assertion::parse("count == 5").unwrap();
        assert!(matches!(a, Assertion::CountEquals(5)));
    }

    #[test]
    fn parse_count_gt() {
        let a = Assertion::parse("count > 0").unwrap();
        assert!(matches!(a, Assertion::CountGt(0)));
    }

    #[test]
    fn parse_count_lt() {
        let a = Assertion::parse("count < 100").unwrap();
        assert!(matches!(a, Assertion::CountLt(100)));
    }

    #[test]
    fn parse_expression() {
        let a = Assertion::parse("cnt > 0").unwrap();
        match a {
            Assertion::Expression { column, op, value } => {
                assert_eq!(column, "cnt");
                assert!(matches!(op, AssertOp::Gt));
                assert_eq!(value, 0);
            }
            _ => panic!("expected Expression"),
        }
    }

    #[test]
    fn parse_expression_gte() {
        let a = Assertion::parse("total >= 10").unwrap();
        match a {
            Assertion::Expression { column, op, value } => {
                assert_eq!(column, "total");
                assert!(matches!(op, AssertOp::Gte));
                assert_eq!(value, 10);
            }
            _ => panic!("expected Expression"),
        }
    }

    #[test]
    fn parse_invalid_format() {
        assert!(Assertion::parse("garbage").is_err());
    }

    #[test]
    fn parse_invalid_value() {
        assert!(Assertion::parse("count > abc").is_err());
    }

    // ── Assertion check tests ────────────────────────────────────────────

    #[test]
    fn check_is_empty_pass() {
        assert!(Assertion::IsEmpty.check(0, None).is_ok());
    }

    #[test]
    fn check_is_empty_fail() {
        assert!(Assertion::IsEmpty.check(3, None).is_err());
    }

    #[test]
    fn check_is_not_empty_pass() {
        assert!(Assertion::IsNotEmpty.check(1, None).is_ok());
    }

    #[test]
    fn check_is_not_empty_fail() {
        assert!(Assertion::IsNotEmpty.check(0, None).is_err());
    }

    #[test]
    fn check_count_equals_pass() {
        assert!(Assertion::CountEquals(5).check(5, None).is_ok());
    }

    #[test]
    fn check_count_equals_fail() {
        assert!(Assertion::CountEquals(5).check(3, None).is_err());
    }

    #[test]
    fn check_count_gt_pass() {
        assert!(Assertion::CountGt(0).check(1, None).is_ok());
    }

    #[test]
    fn check_count_gt_fail() {
        assert!(Assertion::CountGt(0).check(0, None).is_err());
    }

    #[test]
    fn check_expression_pass() {
        let a = Assertion::Expression {
            column: "cnt".into(),
            op: AssertOp::Gt,
            value: 0,
        };
        assert!(a.check(1, Some(5)).is_ok());
    }

    #[test]
    fn check_expression_fail() {
        let a = Assertion::Expression {
            column: "cnt".into(),
            op: AssertOp::Gt,
            value: 10,
        };
        assert!(a.check(1, Some(5)).is_err());
    }

    #[test]
    fn check_expression_no_value() {
        let a = Assertion::Expression {
            column: "cnt".into(),
            op: AssertOp::Gt,
            value: 0,
        };
        assert!(a.check(0, None).is_err());
    }

    // ── Node trait tests ─────────────────────────────────────────────────

    #[test]
    fn cypher_node_type_and_config() {
        let node = CypherNode::new(
            "migrate",
            "MATCH (n) SET n.v = 2".into(),
            Some("MATCH (n) RETURN n._uuid, n.v".into()),
        );
        assert_eq!(node.node_type(), "CypherNode");
        assert!(node.can_undo());
        let config = node.node_config();
        assert_eq!(config["query"].as_str(), Some("MATCH (n) SET n.v = 2"));
        assert_eq!(
            config["capture"].as_str(),
            Some("MATCH (n) RETURN n._uuid, n.v")
        );
    }

    #[test]
    fn cypher_node_no_capture_no_undo() {
        let node = CypherNode::new("step", "MATCH (n) RETURN n".into(), None);
        assert!(!node.can_undo());
        assert!(node.undo_context().is_none());
    }

    #[test]
    fn validate_node_type_and_config() {
        let node = ValidateNode::new(
            "check",
            "MATCH (n) RETURN count(n) AS cnt".into(),
            Assertion::CountGt(0),
            "no nodes found".into(),
        );
        assert_eq!(node.node_type(), "ValidateNode");
        assert!(!node.can_undo());
    }

    // ── Factory tests ────────────────────────────────────────────────────

    #[test]
    fn cypher_factory_creates_node() {
        let factory = CypherNodeFactory;
        let config = serde_json::json!({
            "query": "MATCH (n) SET n.v = 2",
            "capture": "MATCH (n) RETURN n._uuid, n.v",
        });
        let node = factory.create("migrate", &config).unwrap();
        assert_eq!(node.name(), "migrate");
        assert_eq!(node.node_type(), "CypherNode");
        assert!(node.can_undo());
    }

    #[test]
    fn cypher_factory_no_capture() {
        let factory = CypherNodeFactory;
        let config = serde_json::json!({ "query": "RETURN 1" });
        let node = factory.create("step", &config).unwrap();
        assert!(!node.can_undo());
    }

    #[test]
    fn cypher_factory_missing_query() {
        let factory = CypherNodeFactory;
        let config = serde_json::json!({});
        assert!(factory.create("step", &config).is_err());
    }

    #[test]
    fn validate_factory_creates_node() {
        let factory = ValidateNodeFactory;
        let config = serde_json::json!({
            "query": "MATCH (n) RETURN count(n) AS cnt",
            "assert": "cnt > 0",
            "message": "no nodes found",
        });
        let node = factory.create("check", &config).unwrap();
        assert_eq!(node.name(), "check");
        assert_eq!(node.node_type(), "ValidateNode");
    }

    #[test]
    fn validate_factory_default_message() {
        let factory = ValidateNodeFactory;
        let config = serde_json::json!({
            "query": "MATCH (n) RETURN count(n) AS cnt",
            "assert": "empty",
        });
        let node = factory.create("check", &config).unwrap();
        assert_eq!(node.name(), "check");
    }

    #[test]
    fn validate_factory_missing_assert() {
        let factory = ValidateNodeFactory;
        let config = serde_json::json!({ "query": "RETURN 1" });
        assert!(factory.create("check", &config).is_err());
    }

    // ── Helper tests ─────────────────────────────────────────────────────

    #[test]
    fn cypher_value_to_json_conversion() {
        assert_eq!(
            cypher_value_to_json(&CypherValue::String("hello".into())),
            serde_json::json!("hello")
        );
        assert_eq!(
            cypher_value_to_json(&CypherValue::Int(42)),
            serde_json::json!(42)
        );
        assert_eq!(
            cypher_value_to_json(&CypherValue::Bool(true)),
            serde_json::json!(true)
        );
        assert_eq!(
            cypher_value_to_json(&CypherValue::Null),
            serde_json::Value::Null
        );
    }
}

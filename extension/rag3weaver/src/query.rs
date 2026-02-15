//! Parameterized Cypher query builder.
//!
//! Builds `MATCH (n:Table) WHERE ... RETURN ... ORDER BY ... LIMIT` queries
//! with named parameters (`$p0`, `$p1`, ...) for safe prepared statement execution.
//! Pure logic — no database access.

use crate::connection::{CypherValue, QueryParam};

// ─── Types ──────────────────────────────────────────────────────────────────

/// A ready-to-execute query with its parameters.
#[derive(Debug, Clone)]
pub struct PreparedQuery {
    pub cypher: String,
    pub params: Vec<QueryParam>,
}

/// Sort direction for ORDER BY.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

// ─── Internal types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum WhereClause {
    Condition {
        field: String,
        op: WhereOp,
        value: CypherValue,
    },
    IsNull {
        field: String,
    },
    Raw(String),
}

#[derive(Debug, Clone, Copy)]
enum WhereOp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    In,
    NotIn,
}

impl WhereOp {
    fn sql(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Neq => "<>",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::In | Self::NotIn => unreachable!("IN handled separately"),
        }
    }
}

// ─── QueryBuilder ───────────────────────────────────────────────────────────

/// Fluent builder for parameterized MATCH queries.
///
/// # Example
/// ```ignore
/// let q = QueryBuilder::new("Document")
///     .where_eq("title", "hello")
///     .where_gt("page_count", 5_i64)
///     .order_by("title", SortDir::Asc)
///     .limit(10)
///     .build();
/// // q.cypher = "MATCH (n:Document)\nWHERE n.title = $p0 AND n.page_count > $p1\n..."
/// ```
pub struct QueryBuilder {
    table: String,
    alias: String,
    select: Vec<String>,
    conditions: Vec<WhereClause>,
    order_by: Vec<(String, SortDir)>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl QueryBuilder {
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            alias: "n".to_string(),
            select: vec![],
            conditions: vec![],
            order_by: vec![],
            limit: None,
            offset: None,
        }
    }

    pub fn alias(mut self, alias: &str) -> Self {
        self.alias = alias.to_string();
        self
    }

    /// Select specific fields. If not called, defaults to `RETURN {alias}`.
    pub fn select(mut self, fields: &[&str]) -> Self {
        self.select = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    // ── WHERE conditions ─────────────────────────────────────────────────

    pub fn where_eq(mut self, field: &str, value: impl Into<CypherValue>) -> Self {
        self.conditions.push(WhereClause::Condition {
            field: field.to_string(),
            op: WhereOp::Eq,
            value: value.into(),
        });
        self
    }

    pub fn where_neq(mut self, field: &str, value: impl Into<CypherValue>) -> Self {
        self.conditions.push(WhereClause::Condition {
            field: field.to_string(),
            op: WhereOp::Neq,
            value: value.into(),
        });
        self
    }

    pub fn where_lt(mut self, field: &str, value: impl Into<CypherValue>) -> Self {
        self.conditions.push(WhereClause::Condition {
            field: field.to_string(),
            op: WhereOp::Lt,
            value: value.into(),
        });
        self
    }

    pub fn where_lte(mut self, field: &str, value: impl Into<CypherValue>) -> Self {
        self.conditions.push(WhereClause::Condition {
            field: field.to_string(),
            op: WhereOp::Lte,
            value: value.into(),
        });
        self
    }

    pub fn where_gt(mut self, field: &str, value: impl Into<CypherValue>) -> Self {
        self.conditions.push(WhereClause::Condition {
            field: field.to_string(),
            op: WhereOp::Gt,
            value: value.into(),
        });
        self
    }

    pub fn where_gte(mut self, field: &str, value: impl Into<CypherValue>) -> Self {
        self.conditions.push(WhereClause::Condition {
            field: field.to_string(),
            op: WhereOp::Gte,
            value: value.into(),
        });
        self
    }

    pub fn where_in(mut self, field: &str, values: Vec<CypherValue>) -> Self {
        self.conditions.push(WhereClause::Condition {
            field: field.to_string(),
            op: WhereOp::In,
            value: CypherValue::List(values),
        });
        self
    }

    pub fn where_not_in(mut self, field: &str, values: Vec<CypherValue>) -> Self {
        self.conditions.push(WhereClause::Condition {
            field: field.to_string(),
            op: WhereOp::NotIn,
            value: CypherValue::List(values),
        });
        self
    }

    pub fn where_null(mut self, field: &str) -> Self {
        self.conditions.push(WhereClause::IsNull {
            field: field.to_string(),
        });
        self
    }

    /// Insert a raw WHERE clause fragment. No parameter safety.
    pub fn where_raw(mut self, clause: &str) -> Self {
        self.conditions.push(WhereClause::Raw(clause.to_string()));
        self
    }

    // ── ORDER BY, LIMIT, OFFSET ──────────────────────────────────────────

    pub fn order_by(mut self, field: &str, dir: SortDir) -> Self {
        self.order_by.push((field.to_string(), dir));
        self
    }

    pub fn limit(mut self, n: i64) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn offset(mut self, n: i64) -> Self {
        self.offset = Some(n);
        self
    }

    // ── Build ────────────────────────────────────────────────────────────

    /// Build the parameterized query.
    pub fn build(&self) -> PreparedQuery {
        let mut params = Vec::new();
        let mut counter: usize = 0;

        // MATCH
        let alias = &self.alias;
        let table = &self.table;
        let mut cypher = format!("MATCH ({alias}:{table})");

        // WHERE
        let where_parts = self.build_where(&mut params, &mut counter);
        if !where_parts.is_empty() {
            cypher.push_str("\nWHERE ");
            cypher.push_str(&where_parts.join(" AND "));
        }

        // RETURN
        if self.select.is_empty() {
            cypher.push_str(&format!("\nRETURN {alias}"));
        } else {
            let fields = self
                .select
                .iter()
                .map(|f| format!("{alias}.{f}"))
                .collect::<Vec<_>>()
                .join(", ");
            cypher.push_str(&format!("\nRETURN {fields}"));
        }

        // ORDER BY
        if !self.order_by.is_empty() {
            let clauses = self
                .order_by
                .iter()
                .map(|(f, d)| {
                    let dir = match d {
                        SortDir::Asc => "ASC",
                        SortDir::Desc => "DESC",
                    };
                    format!("{alias}.{f} {dir}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            cypher.push_str(&format!("\nORDER BY {clauses}"));
        }

        // SKIP (offset)
        if let Some(off) = self.offset {
            let name = format!("p{counter}");
            counter += 1;
            params.push(QueryParam::new(&name, off));
            cypher.push_str(&format!("\nSKIP ${name}"));
        }

        // LIMIT
        if let Some(lim) = self.limit {
            let name = format!("p{counter}");
            params.push(QueryParam::new(&name, lim));
            cypher.push_str(&format!("\nLIMIT ${name}"));
        }

        PreparedQuery { cypher, params }
    }

    fn build_where(
        &self,
        params: &mut Vec<QueryParam>,
        counter: &mut usize,
    ) -> Vec<String> {
        let alias = &self.alias;
        let mut parts = Vec::new();

        for clause in &self.conditions {
            match clause {
                WhereClause::Condition { field, op, value } => {
                    let name = format!("p{counter}");
                    *counter += 1;
                    params.push(QueryParam::new(&name, value.clone()));

                    match op {
                        WhereOp::In => {
                            parts.push(format!("{alias}.{field} IN ${name}"));
                        }
                        WhereOp::NotIn => {
                            parts.push(format!("NOT {alias}.{field} IN ${name}"));
                        }
                        _ => {
                            parts.push(format!("{alias}.{field} {} ${name}", op.sql()));
                        }
                    }
                }
                WhereClause::IsNull { field } => {
                    parts.push(format!("{alias}.{field} IS NULL"));
                }
                WhereClause::Raw(clause) => {
                    parts.push(clause.clone());
                }
            }
        }

        parts
    }
}

// ─── Insert helper ──────────────────────────────────────────────────────────

/// Build a parameterized INSERT query.
///
/// ```text
/// CREATE (:Document {_uuid: $_uuid, title: $title, body: $body})
/// ```
///
/// Returns only the Cypher template — the caller provides actual values as params.
pub fn build_insert(table: &str, columns: &[&str]) -> String {
    let props = columns
        .iter()
        .map(|c| format!("{c}: ${c}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("CREATE (:{table} {{{props}}})")
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_match() {
        let q = QueryBuilder::new("Document").build();
        assert_eq!(q.cypher, "MATCH (n:Document)\nRETURN n");
        assert!(q.params.is_empty());
    }

    #[test]
    fn select_fields() {
        let q = QueryBuilder::new("Document")
            .select(&["title", "body"])
            .build();
        assert!(q.cypher.contains("RETURN n.title, n.body"));
    }

    #[test]
    fn custom_alias() {
        let q = QueryBuilder::new("Document").alias("d").build();
        assert_eq!(q.cypher, "MATCH (d:Document)\nRETURN d");
    }

    #[test]
    fn where_eq() {
        let q = QueryBuilder::new("Document")
            .where_eq("title", "hello")
            .build();
        assert!(q.cypher.contains("WHERE n.title = $p0"));
        assert_eq!(q.params.len(), 1);
        assert_eq!(q.params[0].name, "p0");
        assert_eq!(q.params[0].value.as_str(), Some("hello"));
    }

    #[test]
    fn where_neq() {
        let q = QueryBuilder::new("T")
            .where_neq("status", "deleted")
            .build();
        assert!(q.cypher.contains("n.status <> $p0"));
    }

    #[test]
    fn where_comparisons() {
        let q = QueryBuilder::new("T")
            .where_lt("a", 1_i64)
            .where_lte("b", 2_i64)
            .where_gt("c", 3_i64)
            .where_gte("d", 4_i64)
            .build();
        assert!(q.cypher.contains("n.a < $p0"));
        assert!(q.cypher.contains("n.b <= $p1"));
        assert!(q.cypher.contains("n.c > $p2"));
        assert!(q.cypher.contains("n.d >= $p3"));
        assert_eq!(q.params.len(), 4);
    }

    #[test]
    fn where_null() {
        let q = QueryBuilder::new("T")
            .where_null("deleted_at")
            .build();
        assert!(q.cypher.contains("n.deleted_at IS NULL"));
        assert!(q.params.is_empty());
    }

    #[test]
    fn where_in() {
        let q = QueryBuilder::new("T")
            .where_in(
                "status",
                vec![CypherValue::from("active"), CypherValue::from("pending")],
            )
            .build();
        assert!(q.cypher.contains("n.status IN $p0"));
        assert_eq!(q.params.len(), 1);
    }

    #[test]
    fn where_not_in() {
        let q = QueryBuilder::new("T")
            .where_not_in(
                "role",
                vec![CypherValue::from("admin"), CypherValue::from("superuser")],
            )
            .build();
        assert!(q.cypher.contains("NOT n.role IN $p0"));
    }

    #[test]
    fn where_multiple() {
        let q = QueryBuilder::new("T")
            .where_eq("a", "x")
            .where_gt("b", 5_i64)
            .where_null("c")
            .build();
        assert!(q.cypher.contains("n.a = $p0 AND n.b > $p1 AND n.c IS NULL"));
    }

    #[test]
    fn where_raw() {
        let q = QueryBuilder::new("T")
            .where_raw("n.score > 0.5")
            .build();
        assert!(q.cypher.contains("WHERE n.score > 0.5"));
        assert!(q.params.is_empty());
    }

    #[test]
    fn order_by_single() {
        let q = QueryBuilder::new("T")
            .order_by("title", SortDir::Asc)
            .build();
        assert!(q.cypher.contains("ORDER BY n.title ASC"));
    }

    #[test]
    fn order_by_multi() {
        let q = QueryBuilder::new("T")
            .order_by("name", SortDir::Asc)
            .order_by("age", SortDir::Desc)
            .build();
        assert!(q.cypher.contains("ORDER BY n.name ASC, n.age DESC"));
    }

    #[test]
    fn limit_offset() {
        let q = QueryBuilder::new("T")
            .offset(10)
            .limit(20)
            .build();
        assert!(q.cypher.contains("SKIP $p0"));
        assert!(q.cypher.contains("LIMIT $p1"));
        assert_eq!(q.params.len(), 2);
        assert_eq!(q.params[0].value.as_i64(), Some(10));
        assert_eq!(q.params[1].value.as_i64(), Some(20));
    }

    #[test]
    fn params_unique_names() {
        let q = QueryBuilder::new("T")
            .where_eq("a", "x")
            .where_eq("b", "y")
            .where_gt("c", 1_i64)
            .offset(0)
            .limit(10)
            .build();
        let names: Vec<&str> = q.params.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["p0", "p1", "p2", "p3", "p4"]);
    }

    #[test]
    fn build_insert_basic() {
        let cypher = build_insert("Document", &["_uuid", "title", "body"]);
        assert_eq!(
            cypher,
            "CREATE (:Document {_uuid: $_uuid, title: $title, body: $body})"
        );
    }

    #[test]
    fn full_query() {
        let q = QueryBuilder::new("Document")
            .select(&["title", "body"])
            .where_eq("status", "published")
            .where_gt("page_count", 10_i64)
            .order_by("title", SortDir::Asc)
            .offset(5)
            .limit(25)
            .build();

        assert!(q.cypher.starts_with("MATCH (n:Document)"));
        assert!(q.cypher.contains("WHERE n.status = $p0 AND n.page_count > $p1"));
        assert!(q.cypher.contains("RETURN n.title, n.body"));
        assert!(q.cypher.contains("ORDER BY n.title ASC"));
        assert!(q.cypher.contains("SKIP $p2"));
        assert!(q.cypher.contains("LIMIT $p3"));
        assert_eq!(q.params.len(), 4);
    }
}

//! Filter parser: generates parameterized WHERE clauses from filter maps.
//!
//! Backend-agnostic via `SchemaDialect`. Supports:
//! - Simple equality: `{ "field": value }` → `n.field = $filter_p0`
//! - Null checks: `Direct(Null)` → `IS NULL`
//! - Arrays (IN): `List([val1, val2])` → `IN $filter_p0`
//! - Operators: `Ops([Gt(18), Lt(65)])` → `> $p0 AND < $p1`
//! - Cross-entity: `"Entity.field"` → JOIN/MATCH clause + WHERE
//! - List operations: HasAny, HasAll, HasNone (dialect-specific functions)

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::RelationDef;
use crate::connection::{CypherValue, QueryParam};
use crate::dialect::SchemaDialect;

// ─── Errors ─────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum FilterError {
    #[error("invalid {kind}: \"{name}\"")]
    InvalidIdentifier { kind: String, name: String },

    #[error("no relation found between \"{from}\" and \"{to}\"")]
    NoRelation { from: String, to: String },
}

// ─── Types ──────────────────────────────────────────────────────────────────

/// A single filter operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", content = "value")]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Eq(CypherValue),
    Neq(CypherValue),
    Lt(CypherValue),
    Lte(CypherValue),
    Gt(CypherValue),
    Gte(CypherValue),
    In(Vec<CypherValue>),
    HasAny(Vec<CypherValue>),
    HasAll(Vec<CypherValue>),
    HasNone(Vec<CypherValue>),
    IsNull,
    IsNotNull,
    IsEmpty,
    IsNotEmpty,
    Between(CypherValue, CypherValue),
    NotIn(Vec<CypherValue>),
    StartsWith(String),
    Contains(String),
    ValuesCount {
        min: Option<usize>,
        max: Option<usize>,
    },
}

/// A filter value: direct value, list (IN shorthand), or operator list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    /// Direct value: equality check, or IS NULL if `CypherValue::Null`.
    Direct(CypherValue),
    /// Array shorthand for IN clause.
    List(Vec<CypherValue>),
    /// One or more operators (combined with AND).
    Ops(Vec<FilterOp>),
}

/// Composable filter condition (Qdrant-like Must/Should/MustNot).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterCondition {
    /// Single field filter.
    Field { key: String, value: FilterValue },
    /// AND: all conditions must match.
    Must(Vec<FilterCondition>),
    /// OR: at least one condition must match.
    Should(Vec<FilterCondition>),
    /// NOT: none of the conditions must match.
    MustNot(Vec<FilterCondition>),
}

impl From<HashMap<String, FilterValue>> for FilterCondition {
    fn from(filters: HashMap<String, FilterValue>) -> Self {
        let fields: Vec<FilterCondition> = filters
            .into_iter()
            .map(|(key, value)| FilterCondition::Field { key, value })
            .collect();
        FilterCondition::Must(fields)
    }
}

/// Parsed filter result with parameterized clauses.
#[derive(Debug, Clone)]
pub struct ParsedFilter {
    /// WHERE clauses (to be joined with AND).
    pub where_clauses: Vec<String>,
    /// MATCH clauses for cross-entity filters.
    pub match_clauses: Vec<String>,
    /// Query parameters.
    pub params: Vec<QueryParam>,
    /// Entity → alias mapping (e.g. "Document" → "n", "Author" → "e1").
    pub aliases: HashMap<String, String>,
}

impl ParsedFilter {
    /// Combine all where clauses into a single string joined with ` AND `.
    pub fn combine_where(&self) -> String {
        self.where_clauses.join(" AND ")
    }
}

// ─── FilterParser ───────────────────────────────────────────────────────────

/// Parse filter maps into parameterized WHERE/JOIN clauses via dialect.
pub struct FilterParser<'a> {
    relations: &'a HashMap<String, RelationDef>,
    dialect: &'a dyn SchemaDialect,
    param_counter: usize,
}

impl<'a> FilterParser<'a> {
    pub fn new(relations: &'a HashMap<String, RelationDef>, dialect: &'a dyn SchemaDialect) -> Self {
        Self {
            relations,
            dialect,
            param_counter: 0,
        }
    }

    /// Parse a filter map into parameterized Cypher clauses.
    ///
    /// - `filters` — field → value map (supports `"Entity.field"` for cross-entity)
    /// - `result_entity` — the entity type that results come from
    /// - `result_alias` — alias for the result entity in the query (e.g. `"n"`)
    pub fn parse(
        &mut self,
        filters: &HashMap<String, FilterValue>,
        result_entity: &str,
        result_alias: &str,
    ) -> Result<ParsedFilter, FilterError> {
        self.param_counter = 0;

        let mut where_clauses = Vec::new();
        let mut match_clauses = Vec::new();
        let mut params = Vec::new();
        let mut aliases = HashMap::new();
        let mut alias_counter = 0_usize;

        validate_identifier(result_entity, "entity")?;
        aliases.insert(result_entity.to_string(), result_alias.to_string());

        for (key, value) in filters {
            let (entity, field) = if let Some((e, f)) = key.split_once('.') {
                (e.to_string(), f.to_string())
            } else {
                (result_entity.to_string(), key.clone())
            };

            validate_identifier(&entity, "entity")?;
            validate_identifier(&field, "field")?;

            // Get or create alias for this entity
            let alias = if let Some(a) = aliases.get(&entity) {
                a.clone()
            } else {
                alias_counter += 1;
                let a = format!("e{alias_counter}");
                aliases.insert(entity.clone(), a.clone());

                // Find relation between result entity and this entity
                let rel = find_relation(self.relations, result_entity, &entity).ok_or_else(
                    || FilterError::NoRelation {
                        from: result_entity.to_string(),
                        to: entity.clone(),
                    },
                )?;

                validate_identifier(&rel.name, "relation")?;

                let forward = rel.from == result_entity;
                match_clauses.push(self.dialect.filter_join_clause(
                    result_alias, &rel.name, &a, &entity, forward,
                ));

                a
            };

            if let Some(clause) = self.build_clause(&alias, &field, value, &mut params) {
                where_clauses.push(clause);
            }
        }

        Ok(ParsedFilter {
            where_clauses,
            match_clauses,
            params,
            aliases,
        })
    }

    /// Parse a `FilterCondition` tree into parameterized Cypher clauses.
    ///
    /// Supports recursive Must (AND), Should (OR), MustNot (NOT) composition.
    /// Cross-entity MATCH clauses are accumulated at the top level (not nested).
    pub fn parse_condition(
        &mut self,
        condition: &FilterCondition,
        result_entity: &str,
        result_alias: &str,
    ) -> Result<ParsedFilter, FilterError> {
        self.param_counter = 0;

        let mut match_clauses = Vec::new();
        let mut params = Vec::new();
        let mut aliases = HashMap::new();
        let mut alias_counter = 0_usize;

        validate_identifier(result_entity, "entity")?;
        aliases.insert(result_entity.to_string(), result_alias.to_string());

        let where_str = self.build_condition_clause(
            condition,
            result_entity,
            result_alias,
            &mut match_clauses,
            &mut params,
            &mut aliases,
            &mut alias_counter,
        )?;

        let where_clauses = if where_str.is_empty() {
            vec![]
        } else {
            vec![where_str]
        };

        Ok(ParsedFilter {
            where_clauses,
            match_clauses,
            params,
            aliases,
        })
    }

    fn build_condition_clause(
        &mut self,
        condition: &FilterCondition,
        result_entity: &str,
        result_alias: &str,
        match_clauses: &mut Vec<String>,
        params: &mut Vec<QueryParam>,
        aliases: &mut HashMap<String, String>,
        alias_counter: &mut usize,
    ) -> Result<String, FilterError> {
        match condition {
            FilterCondition::Field { key, value } => {
                let (entity, field) = if let Some((e, f)) = key.split_once('.') {
                    (e.to_string(), f.to_string())
                } else {
                    (result_entity.to_string(), key.clone())
                };

                validate_identifier(&entity, "entity")?;
                validate_identifier(&field, "field")?;

                let alias = if let Some(a) = aliases.get(&entity) {
                    a.clone()
                } else {
                    *alias_counter += 1;
                    let a = format!("e{alias_counter}");
                    aliases.insert(entity.clone(), a.clone());

                    let rel = find_relation(self.relations, result_entity, &entity)
                        .ok_or_else(|| FilterError::NoRelation {
                            from: result_entity.to_string(),
                            to: entity.clone(),
                        })?;
                    validate_identifier(&rel.name, "relation")?;

                    let forward = rel.from == result_entity;
                    match_clauses.push(self.dialect.filter_join_clause(
                        result_alias, &rel.name, &a, &entity, forward,
                    ));
                    a
                };

                Ok(self
                    .build_clause(&alias, &field, value, params)
                    .unwrap_or_default())
            }
            FilterCondition::Must(conditions) => {
                let clauses: Vec<String> = conditions
                    .iter()
                    .map(|c| {
                        self.build_condition_clause(
                            c,
                            result_entity,
                            result_alias,
                            match_clauses,
                            params,
                            aliases,
                            alias_counter,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect();
                if clauses.is_empty() {
                    Ok(String::new())
                } else if clauses.len() == 1 {
                    Ok(clauses.into_iter().next().unwrap())
                } else {
                    Ok(format!("({})", clauses.join(" AND ")))
                }
            }
            FilterCondition::Should(conditions) => {
                let clauses: Vec<String> = conditions
                    .iter()
                    .map(|c| {
                        self.build_condition_clause(
                            c,
                            result_entity,
                            result_alias,
                            match_clauses,
                            params,
                            aliases,
                            alias_counter,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect();
                if clauses.is_empty() {
                    Ok(String::new())
                } else if clauses.len() == 1 {
                    Ok(clauses.into_iter().next().unwrap())
                } else {
                    Ok(format!("({})", clauses.join(" OR ")))
                }
            }
            FilterCondition::MustNot(conditions) => {
                let clauses: Vec<String> = conditions
                    .iter()
                    .map(|c| {
                        self.build_condition_clause(
                            c,
                            result_entity,
                            result_alias,
                            match_clauses,
                            params,
                            aliases,
                            alias_counter,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect();
                if clauses.is_empty() {
                    Ok(String::new())
                } else if clauses.len() == 1 {
                    Ok(format!(
                        "NOT ({})",
                        clauses.into_iter().next().unwrap()
                    ))
                } else {
                    Ok(format!("NOT ({})", clauses.join(" AND ")))
                }
            }
        }
    }

    fn next_param(&mut self) -> String {
        let name = format!("filter_p{}", self.param_counter);
        self.param_counter += 1;
        name
    }

    fn build_clause(
        &mut self,
        alias: &str,
        field: &str,
        value: &FilterValue,
        params: &mut Vec<QueryParam>,
    ) -> Option<String> {
        let prop = format!("{alias}.{field}");

        match value {
            FilterValue::Direct(cv) => {
                if cv.is_null() {
                    Some(format!("{prop} IS NULL"))
                } else {
                    let p = self.next_param();
                    params.push(QueryParam::new(&p, cv.clone()));
                    Some(format!("{prop} = ${p}"))
                }
            }
            FilterValue::List(items) => {
                let p = self.next_param();
                params.push(QueryParam::new(&p, CypherValue::List(items.clone())));
                Some(format!("{prop} IN ${p}"))
            }
            FilterValue::Ops(ops) => {
                let clauses: Vec<String> = ops
                    .iter()
                    .filter_map(|op| self.build_op_clause(&prop, op, params))
                    .collect();
                if clauses.is_empty() {
                    None
                } else {
                    Some(clauses.join(" AND "))
                }
            }
        }
    }

    fn build_op_clause(
        &mut self,
        prop: &str,
        op: &FilterOp,
        params: &mut Vec<QueryParam>,
    ) -> Option<String> {
        let (operator, value) = match op {
            FilterOp::Eq(v) => ("=", v),
            FilterOp::Neq(v) => ("<>", v),
            FilterOp::Lt(v) => ("<", v),
            FilterOp::Lte(v) => ("<=", v),
            FilterOp::Gt(v) => (">", v),
            FilterOp::Gte(v) => (">=", v),
            // List ops handled separately
            FilterOp::In(_)
            | FilterOp::HasAny(_)
            | FilterOp::HasAll(_)
            | FilterOp::HasNone(_)
            | FilterOp::NotIn(_) => {
                return self.build_list_op_clause(prop, op, params);
            }
            // Special ops (no value or multi-value)
            FilterOp::IsNull
            | FilterOp::IsNotNull
            | FilterOp::IsEmpty
            | FilterOp::IsNotEmpty
            | FilterOp::Between(_, _)
            | FilterOp::StartsWith(_)
            | FilterOp::Contains(_)
            | FilterOp::ValuesCount { .. } => {
                return self.build_special_op_clause(prop, op, params);
            }
        };

        let p = self.next_param();
        params.push(QueryParam::new(&p, value.clone()));
        Some(format!("{prop} {operator} ${p}"))
    }

    fn build_list_op_clause(
        &mut self,
        prop: &str,
        op: &FilterOp,
        params: &mut Vec<QueryParam>,
    ) -> Option<String> {
        let p = self.next_param();
        match op {
            FilterOp::In(items) => {
                params.push(QueryParam::new(&p, CypherValue::List(items.clone())));
                Some(format!("{prop} IN ${p}"))
            }
            FilterOp::HasAny(items) => {
                params.push(QueryParam::new(&p, CypherValue::List(items.clone())));
                Some(self.dialect.filter_list_any_match(prop, &p))
            }
            FilterOp::HasAll(items) => {
                params.push(QueryParam::new(&p, CypherValue::List(items.clone())));
                Some(self.dialect.filter_list_all(prop, &p))
            }
            FilterOp::HasNone(items) => {
                params.push(QueryParam::new(&p, CypherValue::List(items.clone())));
                Some(self.dialect.filter_list_none(prop, &p))
            }
            FilterOp::NotIn(items) => {
                params.push(QueryParam::new(&p, CypherValue::List(items.clone())));
                Some(format!("NOT ({prop} IN ${p})"))
            }
            _ => None,
        }
    }
    fn build_special_op_clause(
        &mut self,
        prop: &str,
        op: &FilterOp,
        params: &mut Vec<QueryParam>,
    ) -> Option<String> {
        match op {
            FilterOp::IsNull => Some(format!("{prop} IS NULL")),
            FilterOp::IsNotNull => Some(format!("{prop} IS NOT NULL")),
            FilterOp::IsEmpty => Some(format!("{} = 0", self.dialect.filter_size_expr(prop))),
            FilterOp::IsNotEmpty => Some(format!("{} > 0", self.dialect.filter_size_expr(prop))),
            FilterOp::Between(lo, hi) => {
                let p_lo = self.next_param();
                let p_hi = self.next_param();
                params.push(QueryParam::new(&p_lo, lo.clone()));
                params.push(QueryParam::new(&p_hi, hi.clone()));
                Some(format!("{prop} >= ${p_lo} AND {prop} <= ${p_hi}"))
            }
            FilterOp::StartsWith(s) => {
                let p = self.next_param();
                params.push(QueryParam::new(&p, CypherValue::String(s.clone())));
                Some(self.dialect.filter_starts_with(prop, &p))
            }
            FilterOp::Contains(s) => {
                let p = self.next_param();
                params.push(QueryParam::new(&p, CypherValue::String(s.clone())));
                Some(self.dialect.filter_contains(prop, &p))
            }
            FilterOp::ValuesCount { min, max } => {
                let mut clauses = Vec::new();
                let size = self.dialect.filter_size_expr(prop);
                if let Some(min_val) = min {
                    let p = self.next_param();
                    params.push(QueryParam::new(&p, CypherValue::Int(*min_val as i64)));
                    clauses.push(format!("{size} >= ${p}"));
                }
                if let Some(max_val) = max {
                    let p = self.next_param();
                    params.push(QueryParam::new(&p, CypherValue::Int(*max_val as i64)));
                    clauses.push(format!("{size} <= ${p}"));
                }
                if clauses.is_empty() {
                    None
                } else {
                    Some(clauses.join(" AND "))
                }
            }
            _ => None,
        }
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

struct FoundRelation {
    name: String,
    from: String,
}

fn find_relation(
    relations: &HashMap<String, RelationDef>,
    entity_a: &str,
    entity_b: &str,
) -> Option<FoundRelation> {
    for (name, def) in relations {
        if (def.from == entity_a && def.to == entity_b)
            || (def.from == entity_b && def.to == entity_a)
        {
            return Some(FoundRelation {
                name: name.clone(),
                from: def.from.clone(),
            });
        }
    }
    None
}

/// Check whether a string is a valid Cypher identifier (`[a-zA-Z_][a-zA-Z0-9_]*`).
pub fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn validate_identifier(name: &str, kind: &str) -> Result<(), FilterError> {
    if is_valid_identifier(name) {
        Ok(())
    } else {
        Err(FilterError::InvalidIdentifier {
            kind: kind.to_string(),
            name: name.to_string(),
        })
    }
}

// ─── From impls ─────────────────────────────────────────────────────────────

impl From<CypherValue> for FilterValue {
    fn from(v: CypherValue) -> Self {
        Self::Direct(v)
    }
}

impl From<&str> for FilterValue {
    fn from(s: &str) -> Self {
        Self::Direct(CypherValue::from(s))
    }
}

impl From<i64> for FilterValue {
    fn from(n: i64) -> Self {
        Self::Direct(CypherValue::from(n))
    }
}

impl From<f64> for FilterValue {
    fn from(f: f64) -> Self {
        Self::Direct(CypherValue::from(f))
    }
}

impl From<bool> for FilterValue {
    fn from(b: bool) -> Self {
        Self::Direct(CypherValue::from(b))
    }
}

impl From<Vec<CypherValue>> for FilterValue {
    fn from(v: Vec<CypherValue>) -> Self {
        Self::List(v)
    }
}

// ─── FilterBuilder ──────────────────────────────────────────────────────────

/// Ergonomic builder for constructing filter conditions.
///
/// ```ignore
/// let filter = FilterBuilder::new()
///     .eq("status", "active")
///     .range("price", Some(CypherValue::from(10)), Some(CypherValue::from(100)))
///     .is_not_null("email")
///     .build();
/// ```
pub struct FilterBuilder {
    conditions: Vec<FilterCondition>,
}

impl FilterBuilder {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
        }
    }

    /// Add a raw field filter.
    pub fn field(mut self, key: &str, value: impl Into<FilterValue>) -> Self {
        self.conditions.push(FilterCondition::Field {
            key: key.to_string(),
            value: value.into(),
        });
        self
    }

    /// Equality filter: `field = value`.
    pub fn eq(mut self, key: &str, value: impl Into<CypherValue>) -> Self {
        self.conditions.push(FilterCondition::Field {
            key: key.to_string(),
            value: FilterValue::Ops(vec![FilterOp::Eq(value.into())]),
        });
        self
    }

    /// Not-equal filter: `field <> value`.
    pub fn neq(mut self, key: &str, value: impl Into<CypherValue>) -> Self {
        self.conditions.push(FilterCondition::Field {
            key: key.to_string(),
            value: FilterValue::Ops(vec![FilterOp::Neq(value.into())]),
        });
        self
    }

    /// Range filter: optional min (>=) and/or max (<=).
    pub fn range(
        mut self,
        key: &str,
        min: Option<CypherValue>,
        max: Option<CypherValue>,
    ) -> Self {
        let mut ops = Vec::new();
        if let Some(lo) = min {
            ops.push(FilterOp::Gte(lo));
        }
        if let Some(hi) = max {
            ops.push(FilterOp::Lte(hi));
        }
        if !ops.is_empty() {
            self.conditions.push(FilterCondition::Field {
                key: key.to_string(),
                value: FilterValue::Ops(ops),
            });
        }
        self
    }

    /// IN filter: `field IN [values]`.
    pub fn is_in(mut self, key: &str, values: Vec<CypherValue>) -> Self {
        self.conditions.push(FilterCondition::Field {
            key: key.to_string(),
            value: FilterValue::Ops(vec![FilterOp::In(values)]),
        });
        self
    }

    /// IS NULL filter.
    pub fn is_null(mut self, key: &str) -> Self {
        self.conditions.push(FilterCondition::Field {
            key: key.to_string(),
            value: FilterValue::Ops(vec![FilterOp::IsNull]),
        });
        self
    }

    /// IS NOT NULL filter.
    pub fn is_not_null(mut self, key: &str) -> Self {
        self.conditions.push(FilterCondition::Field {
            key: key.to_string(),
            value: FilterValue::Ops(vec![FilterOp::IsNotNull]),
        });
        self
    }

    /// HasAny: list contains at least one of the values.
    pub fn has_any(mut self, key: &str, values: Vec<CypherValue>) -> Self {
        self.conditions.push(FilterCondition::Field {
            key: key.to_string(),
            value: FilterValue::Ops(vec![FilterOp::HasAny(values)]),
        });
        self
    }

    /// StartsWith: string prefix match.
    pub fn starts_with(mut self, key: &str, prefix: &str) -> Self {
        self.conditions.push(FilterCondition::Field {
            key: key.to_string(),
            value: FilterValue::Ops(vec![FilterOp::StartsWith(prefix.to_string())]),
        });
        self
    }

    /// Contains: substring match.
    pub fn contains_str(mut self, key: &str, substring: &str) -> Self {
        self.conditions.push(FilterCondition::Field {
            key: key.to_string(),
            value: FilterValue::Ops(vec![FilterOp::Contains(substring.to_string())]),
        });
        self
    }

    /// Create an OR condition (static).
    pub fn or(conditions: Vec<FilterCondition>) -> FilterCondition {
        FilterCondition::Should(conditions)
    }

    /// Create a NOT condition (static).
    pub fn not(conditions: Vec<FilterCondition>) -> FilterCondition {
        FilterCondition::MustNot(conditions)
    }

    /// Build into a `FilterCondition::Must`.
    pub fn build(self) -> FilterCondition {
        FilterCondition::Must(self.conditions)
    }

    /// Build into a legacy `HashMap<String, FilterValue>` for retrocompatibility.
    ///
    /// Only works if all conditions are `Field` variants. Panics otherwise.
    pub fn build_map(self) -> HashMap<String, FilterValue> {
        let mut map = HashMap::new();
        for cond in self.conditions {
            match cond {
                FilterCondition::Field { key, value } => {
                    map.insert(key, value);
                }
                _ => panic!("build_map only supports Field conditions"),
            }
        }
        map
    }
}

impl Default for FilterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::Rag3dbDialect;

    fn no_relations() -> HashMap<String, RelationDef> {
        HashMap::new()
    }

    fn with_relation(name: &str, from: &str, to: &str) -> HashMap<String, RelationDef> {
        let mut rels = HashMap::new();
        rels.insert(
            name.to_string(),
            RelationDef {
                from: from.to_string(),
                to: to.to_string(),
                properties: None,
            },
        );
        rels
    }

    fn filters_one(key: &str, val: FilterValue) -> HashMap<String, FilterValue> {
        let mut f = HashMap::new();
        f.insert(key.to_string(), val);
        f
    }

    // ── empty ───────────────────────────────────────────────────────────

    #[test]
    fn empty_filters() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let result = parser.parse(&HashMap::new(), "Document", "n").unwrap();
        assert!(result.where_clauses.is_empty());
        assert!(result.match_clauses.is_empty());
        assert!(result.params.is_empty());
        assert_eq!(result.aliases["Document"], "n");
    }

    // ── direct values ───────────────────────────────────────────────────

    #[test]
    fn parse_simple_eq_string() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one("status", "active".into());
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(r.where_clauses, vec!["n.status = $filter_p0"]);
        assert_eq!(r.params.len(), 1);
        assert_eq!(r.params[0].name, "filter_p0");
        assert_eq!(r.params[0].value.as_str(), Some("active"));
        assert!(r.match_clauses.is_empty());
    }

    #[test]
    fn parse_simple_eq_int() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one("count", 42_i64.into());
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(r.where_clauses, vec!["n.count = $filter_p0"]);
        assert_eq!(r.params[0].value.as_i64(), Some(42));
    }

    #[test]
    fn parse_simple_eq_bool() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one("active", true.into());
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(r.where_clauses, vec!["n.active = $filter_p0"]);
        assert_eq!(r.params[0].value.as_bool(), Some(true));
    }

    #[test]
    fn parse_null() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one("field", CypherValue::Null.into());
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(r.where_clauses, vec!["n.field IS NULL"]);
        assert!(r.params.is_empty());
    }

    // ── array IN ────────────────────────────────────────────────────────

    #[test]
    fn parse_array_in() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "status",
            vec![CypherValue::from("active"), CypherValue::from("pending")].into(),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(r.where_clauses, vec!["n.status IN $filter_p0"]);
        assert_eq!(r.params.len(), 1);
        match &r.params[0].value {
            CypherValue::List(items) => assert_eq!(items.len(), 2),
            _ => panic!("expected list param"),
        }
    }

    // ── operators ───────────────────────────────────────────────────────

    #[test]
    fn parse_op_gt_lt() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "age",
            FilterValue::Ops(vec![
                FilterOp::Gt(CypherValue::from(18_i64)),
                FilterOp::Lt(CypherValue::from(65_i64)),
            ]),
        );
        let r = parser.parse(&filters, "Person", "n").unwrap();

        assert_eq!(
            r.where_clauses,
            vec!["n.age > $filter_p0 AND n.age < $filter_p1"]
        );
        assert_eq!(r.params.len(), 2);
        assert_eq!(r.params[0].value.as_i64(), Some(18));
        assert_eq!(r.params[1].value.as_i64(), Some(65));
    }

    #[test]
    fn parse_op_lte_gte() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "score",
            FilterValue::Ops(vec![
                FilterOp::Gte(CypherValue::from(0.0_f64)),
                FilterOp::Lte(CypherValue::from(1.0_f64)),
            ]),
        );
        let r = parser.parse(&filters, "Result", "n").unwrap();

        assert_eq!(
            r.where_clauses,
            vec!["n.score >= $filter_p0 AND n.score <= $filter_p1"]
        );
    }

    #[test]
    fn parse_op_neq() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "status",
            FilterValue::Ops(vec![FilterOp::Neq(CypherValue::from("deleted"))]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(r.where_clauses, vec!["n.status <> $filter_p0"]);
    }

    #[test]
    fn parse_op_eq() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "type",
            FilterValue::Ops(vec![FilterOp::Eq(CypherValue::from("article"))]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(r.where_clauses, vec!["n.type = $filter_p0"]);
    }

    #[test]
    fn parse_op_in() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "status",
            FilterValue::Ops(vec![FilterOp::In(vec![
                CypherValue::from("a"),
                CypherValue::from("b"),
            ])]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(r.where_clauses, vec!["n.status IN $filter_p0"]);
    }

    // ── list operations ─────────────────────────────────────────────────

    #[test]
    fn parse_has_any() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "tags",
            FilterValue::Ops(vec![FilterOp::HasAny(vec![
                CypherValue::from("rust"),
                CypherValue::from("python"),
            ])]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(
            r.where_clauses,
            vec!["list_any_match(n.tags, v -> list_contains($filter_p0, v))"]
        );
    }

    #[test]
    fn parse_has_all() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "tags",
            FilterValue::Ops(vec![FilterOp::HasAll(vec![
                CypherValue::from("rust"),
                CypherValue::from("wasm"),
            ])]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(
            r.where_clauses,
            vec!["list_all($filter_p0, v -> list_contains(n.tags, v))"]
        );
    }

    #[test]
    fn parse_has_none() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "tags",
            FilterValue::Ops(vec![FilterOp::HasNone(vec![CypherValue::from("legacy")])]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(
            r.where_clauses,
            vec!["NOT list_any_match(n.tags, v -> list_contains($filter_p0, v))"]
        );
    }

    // ── new ops ─────────────────────────────────────────────────────────

    #[test]
    fn parse_op_is_null() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "deleted_at",
            FilterValue::Ops(vec![FilterOp::IsNull]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();
        assert_eq!(r.where_clauses, vec!["n.deleted_at IS NULL"]);
        assert!(r.params.is_empty());
    }

    #[test]
    fn parse_op_is_not_null() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "email",
            FilterValue::Ops(vec![FilterOp::IsNotNull]),
        );
        let r = parser.parse(&filters, "Person", "n").unwrap();
        assert_eq!(r.where_clauses, vec!["n.email IS NOT NULL"]);
        assert!(r.params.is_empty());
    }

    #[test]
    fn parse_op_is_empty() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "tags",
            FilterValue::Ops(vec![FilterOp::IsEmpty]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();
        assert_eq!(r.where_clauses, vec!["size(n.tags) = 0"]);
    }

    #[test]
    fn parse_op_is_not_empty() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "tags",
            FilterValue::Ops(vec![FilterOp::IsNotEmpty]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();
        assert_eq!(r.where_clauses, vec!["size(n.tags) > 0"]);
    }

    #[test]
    fn parse_op_between() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "price",
            FilterValue::Ops(vec![FilterOp::Between(
                CypherValue::from(10.0_f64),
                CypherValue::from(100.0_f64),
            )]),
        );
        let r = parser.parse(&filters, "Product", "n").unwrap();
        assert_eq!(
            r.where_clauses,
            vec!["n.price >= $filter_p0 AND n.price <= $filter_p1"]
        );
        assert_eq!(r.params.len(), 2);
        assert_eq!(r.params[0].value.as_f64(), Some(10.0));
        assert_eq!(r.params[1].value.as_f64(), Some(100.0));
    }

    #[test]
    fn parse_op_not_in() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "status",
            FilterValue::Ops(vec![FilterOp::NotIn(vec![
                CypherValue::from("deleted"),
                CypherValue::from("archived"),
            ])]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();
        assert_eq!(r.where_clauses, vec!["NOT (n.status IN $filter_p0)"]);
        assert_eq!(r.params.len(), 1);
    }

    #[test]
    fn parse_op_starts_with() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "name",
            FilterValue::Ops(vec![FilterOp::StartsWith("Dr.".to_string())]),
        );
        let r = parser.parse(&filters, "Person", "n").unwrap();
        assert_eq!(r.where_clauses, vec!["starts_with(n.name, $filter_p0)"]);
        assert_eq!(r.params[0].value.as_str(), Some("Dr."));
    }

    #[test]
    fn parse_op_contains() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "description",
            FilterValue::Ops(vec![FilterOp::Contains("important".to_string())]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();
        assert_eq!(
            r.where_clauses,
            vec!["contains(n.description, $filter_p0)"]
        );
        assert_eq!(r.params[0].value.as_str(), Some("important"));
    }

    #[test]
    fn parse_op_values_count() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "tags",
            FilterValue::Ops(vec![FilterOp::ValuesCount {
                min: Some(2),
                max: Some(10),
            }]),
        );
        let r = parser.parse(&filters, "Document", "n").unwrap();
        assert_eq!(
            r.where_clauses,
            vec!["size(n.tags) >= $filter_p0 AND size(n.tags) <= $filter_p1"]
        );
        assert_eq!(r.params.len(), 2);
        assert_eq!(r.params[0].value.as_i64(), Some(2));
        assert_eq!(r.params[1].value.as_i64(), Some(10));
    }

    // ── cross-entity ────────────────────────────────────────────────────

    #[test]
    fn parse_cross_entity_outgoing() {
        let rels = with_relation("WROTE", "Document", "Author");
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one("Author.name", "John".into());
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(r.match_clauses, vec!["MATCH (n)-[:WROTE]->(e1:Author)"]);
        assert_eq!(r.where_clauses, vec!["e1.name = $filter_p0"]);
        assert_eq!(r.params[0].value.as_str(), Some("John"));
        assert_eq!(r.aliases["Author"], "e1");
    }

    #[test]
    fn parse_cross_entity_incoming() {
        // Relation defined from Author to Document: Author -[:WROTE]-> Document
        // Filtering from Document perspective → incoming arrow
        let rels = with_relation("WROTE", "Author", "Document");
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one("Author.name", "John".into());
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(r.match_clauses, vec!["MATCH (n)<-[:WROTE]-(e1:Author)"]);
        assert_eq!(r.where_clauses, vec!["e1.name = $filter_p0"]);
    }

    #[test]
    fn parse_no_relation_error() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one("Author.name", "John".into());
        let err = parser.parse(&filters, "Document", "n").unwrap_err();

        match err {
            FilterError::NoRelation { from, to } => {
                assert_eq!(from, "Document");
                assert_eq!(to, "Author");
            }
            _ => panic!("expected NoRelation error"),
        }
    }

    #[test]
    fn cross_entity_reuses_alias() {
        let rels = with_relation("WROTE", "Document", "Author");
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let mut filters = HashMap::new();
        filters.insert("Author.name".to_string(), FilterValue::from("John"));
        filters.insert("Author.age".to_string(), FilterValue::from(30_i64));
        let r = parser.parse(&filters, "Document", "n").unwrap();

        // Only one MATCH clause even though two filters on Author
        assert_eq!(r.match_clauses.len(), 1);
        assert_eq!(r.where_clauses.len(), 2);
        // Both WHERE clauses use the same alias
        assert!(r.where_clauses.iter().all(|c| c.starts_with("e1.")));
    }

    // ── multiple filters ────────────────────────────────────────────────

    #[test]
    fn parse_multiple_filters() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let mut filters = HashMap::new();
        filters.insert("status".to_string(), FilterValue::from("active"));
        filters.insert("count".to_string(), FilterValue::from(10_i64));
        let r = parser.parse(&filters, "Document", "n").unwrap();

        assert_eq!(r.where_clauses.len(), 2);
        assert_eq!(r.params.len(), 2);
        // Order is not guaranteed (HashMap), but both clauses should be present
        let joined = r.combine_where();
        assert!(joined.contains("n.status"));
        assert!(joined.contains("n.count"));
    }

    // ── combine_where ───────────────────────────────────────────────────

    #[test]
    fn combine_where_empty() {
        let r = ParsedFilter {
            where_clauses: vec![],
            match_clauses: vec![],
            params: vec![],
            aliases: HashMap::new(),
        };
        assert_eq!(r.combine_where(), "");
    }

    #[test]
    fn combine_where_single() {
        let r = ParsedFilter {
            where_clauses: vec!["n.x = $p".to_string()],
            match_clauses: vec![],
            params: vec![],
            aliases: HashMap::new(),
        };
        assert_eq!(r.combine_where(), "n.x = $p");
    }

    #[test]
    fn combine_where_multiple() {
        let r = ParsedFilter {
            where_clauses: vec!["n.x = $p0".to_string(), "n.y > $p1".to_string()],
            match_clauses: vec![],
            params: vec![],
            aliases: HashMap::new(),
        };
        assert_eq!(r.combine_where(), "n.x = $p0 AND n.y > $p1");
    }

    // ── identifier validation ───────────────────────────────────────────

    #[test]
    fn valid_identifiers() {
        assert!(is_valid_identifier("Document"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("my_table_2"));
        assert!(is_valid_identifier("A"));
    }

    #[test]
    fn invalid_identifiers() {
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("123bad"));
        assert!(!is_valid_identifier("has space"));
        assert!(!is_valid_identifier("semi;colon"));
        assert!(!is_valid_identifier("drop()"));
    }

    #[test]
    fn invalid_entity_in_parse() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one("status", "active".into());
        let err = parser.parse(&filters, "123Bad", "n").unwrap_err();
        matches!(err, FilterError::InvalidIdentifier { .. });
    }

    #[test]
    fn invalid_field_in_filter() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one("has space", "active".into());
        let err = parser.parse(&filters, "Document", "n").unwrap_err();
        matches!(err, FilterError::InvalidIdentifier { .. });
    }

    // ── param naming ────────────────────────────────────────────────────

    #[test]
    fn param_names_are_sequential() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let filters = filters_one(
            "age",
            FilterValue::Ops(vec![
                FilterOp::Gt(CypherValue::from(18_i64)),
                FilterOp::Lt(CypherValue::from(65_i64)),
                FilterOp::Neq(CypherValue::from(42_i64)),
            ]),
        );
        let r = parser.parse(&filters, "Person", "n").unwrap();

        assert_eq!(r.params[0].name, "filter_p0");
        assert_eq!(r.params[1].name, "filter_p1");
        assert_eq!(r.params[2].name, "filter_p2");
    }

    #[test]
    fn param_counter_resets_between_parses() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);

        let f1 = filters_one("a", "x".into());
        let r1 = parser.parse(&f1, "Doc", "n").unwrap();
        assert_eq!(r1.params[0].name, "filter_p0");

        // Second parse should reset counter
        let f2 = filters_one("b", "y".into());
        let r2 = parser.parse(&f2, "Doc", "n").unwrap();
        assert_eq!(r2.params[0].name, "filter_p0");
    }

    // ── FilterCondition ─────────────────────────────────────────────────

    #[test]
    fn condition_must_and() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let cond = FilterCondition::Must(vec![
            FilterCondition::Field {
                key: "status".to_string(),
                value: "active".into(),
            },
            FilterCondition::Field {
                key: "count".to_string(),
                value: FilterValue::Ops(vec![FilterOp::Gt(CypherValue::from(5_i64))]),
            },
        ]);
        let r = parser
            .parse_condition(&cond, "Document", "n")
            .unwrap();
        let where_str = r.combine_where();
        assert!(where_str.contains("n.status = $filter_p0"));
        assert!(where_str.contains("AND"));
        assert!(where_str.contains("n.count > $filter_p1"));
        assert_eq!(r.params.len(), 2);
    }

    #[test]
    fn condition_should_or() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let cond = FilterCondition::Should(vec![
            FilterCondition::Field {
                key: "status".to_string(),
                value: "active".into(),
            },
            FilterCondition::Field {
                key: "status".to_string(),
                value: "pending".into(),
            },
        ]);
        let r = parser
            .parse_condition(&cond, "Document", "n")
            .unwrap();
        let where_str = r.combine_where();
        assert!(where_str.contains("OR"));
        assert!(where_str.contains("n.status = $filter_p0"));
        assert!(where_str.contains("n.status = $filter_p1"));
    }

    #[test]
    fn condition_must_not() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let cond = FilterCondition::MustNot(vec![FilterCondition::Field {
            key: "deleted".to_string(),
            value: true.into(),
        }]);
        let r = parser
            .parse_condition(&cond, "Document", "n")
            .unwrap();
        let where_str = r.combine_where();
        assert!(where_str.starts_with("NOT ("));
        assert!(where_str.contains("n.deleted = $filter_p0"));
    }

    #[test]
    fn condition_nested_must_should() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        // Must(Should(a, b), Field(c))
        let cond = FilterCondition::Must(vec![
            FilterCondition::Should(vec![
                FilterCondition::Field {
                    key: "status".to_string(),
                    value: "active".into(),
                },
                FilterCondition::Field {
                    key: "status".to_string(),
                    value: "pending".into(),
                },
            ]),
            FilterCondition::Field {
                key: "type".to_string(),
                value: "article".into(),
            },
        ]);
        let r = parser
            .parse_condition(&cond, "Document", "n")
            .unwrap();
        let where_str = r.combine_where();
        // Should produce: ((... OR ...) AND n.type = ...)
        assert!(where_str.contains("OR"));
        assert!(where_str.contains("AND"));
        assert_eq!(r.params.len(), 3);
    }

    #[test]
    fn condition_from_hashmap_retrocompat() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let mut filters = HashMap::new();
        filters.insert("status".to_string(), FilterValue::from("active"));
        let cond: FilterCondition = filters.into();
        let r = parser
            .parse_condition(&cond, "Document", "n")
            .unwrap();
        assert!(r.combine_where().contains("n.status = $filter_p0"));
        assert_eq!(r.params.len(), 1);
    }

    // ── FilterBuilder ───────────────────────────────────────────────────

    #[test]
    fn builder_basic_chain() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let cond = FilterBuilder::new()
            .eq("status", CypherValue::from("active"))
            .is_not_null("email")
            .starts_with("name", "Dr.")
            .build();
        let r = parser
            .parse_condition(&cond, "Person", "n")
            .unwrap();
        let w = r.combine_where();
        assert!(w.contains("n.status = $filter_p0"));
        assert!(w.contains("n.email IS NOT NULL"));
        assert!(w.contains("starts_with(n.name, $filter_p"));
        assert_eq!(r.params.len(), 2); // eq + starts_with (is_not_null has no params)
    }

    #[test]
    fn builder_with_or() {
        let rels = no_relations();
        let mut parser = FilterParser::new(&rels, &Rag3dbDialect);
        let cond = FilterCondition::Must(vec![
            FilterBuilder::or(vec![
                FilterCondition::Field {
                    key: "status".to_string(),
                    value: "active".into(),
                },
                FilterCondition::Field {
                    key: "status".to_string(),
                    value: "pending".into(),
                },
            ]),
            FilterCondition::Field {
                key: "type".to_string(),
                value: "article".into(),
            },
        ]);
        let r = parser
            .parse_condition(&cond, "Document", "n")
            .unwrap();
        let w = r.combine_where();
        assert!(w.contains("OR"));
        assert!(w.contains("AND"));
    }

    #[test]
    fn builder_build_map_retrocompat() {
        let map = FilterBuilder::new()
            .eq("status", CypherValue::from("active"))
            .is_null("deleted_at")
            .build_map();
        assert!(map.contains_key("status"));
        assert!(map.contains_key("deleted_at"));
        assert_eq!(map.len(), 2);
    }

}

//! The family `count` of the memory connection (owner: codeparsers). See [`crate::memory_dialect`].
//!
//! - `count` (`Count`): `Rows` and `Matching` without a filter, one row, one column; `Edges`, for each start of
//!   `$uuids`, its degree in the relation in one direction (a start without an edge has no row). The filtered forms
//!   (`Matching` with a filter, `EdgesMatching`) wait for the one evaluator of predicates, in `read`.
//! - `count_links`: the edges of a relation, in one row.

use serde_json::Value;

use super::{not_written, param, State};
use crate::connection::{CypherValue, DbError, QueryParam, QueryResult};

pub(super) fn execute(state: &mut State, op: &str, stmt: &Value, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    match op {
        "count" => {
            let form: rag3weaver_ir::Count = serde_json::from_value(stmt["form"].clone())
                .map_err(|e| DbError::QueryError(format!("connexion mémoire : un compte illisible ({e})")))?;
            count(state, &form, params)
        }
        "count_links" => {
            let relation = stmt["relation"].as_str().unwrap_or("");
            Ok(one(edges_of(state, relation)))
        }
        _ => Err(not_written("count", op)),
    }
}

fn one(n: usize) -> QueryResult {
    QueryResult { columns: vec!["count".into()], rows: vec![vec![CypherValue::Int(n as i64)]] }
}

/// The edges of a relation: from every node of its source table.
fn edges_of(state: &State, relation: &str) -> usize {
    let Some(table) = state.rel_tables.get(relation).map(|r| r.from.clone()) else { return 0 };
    state.image.nodes_of_table(&table).iter().map(|from| state.image.outgoing(relation, from).len()).sum()
}

fn count(state: &State, form: &rag3weaver_ir::Count, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    use rag3weaver_ir::Count as C;
    match form {
        C::Rows { table } | C::Matching { table, filter: None } => Ok(one(state.image.nodes_of_table(table).len())),
        C::Edges { relation, direction, .. } => {
            let Some(CypherValue::List(uuids)) = param(params, "uuids") else {
                return Err(DbError::QueryError(format!("connexion mémoire : le degré de « {relation} » attend $uuids, une liste")));
            };
            let mut rows = Vec::new();
            for u in uuids {
                let CypherValue::String(u) = u else { continue };
                let n = match direction {
                    rag3weaver_ir::Direction::Outgoing => state.image.outgoing(relation, u).len(),
                    rag3weaver_ir::Direction::Incoming => state.image.incoming(relation, u).len(),
                };
                if n > 0 {
                    rows.push(vec![CypherValue::String(u.clone()), CypherValue::Int(n as i64)]);
                }
            }
            Ok(QueryResult { columns: vec!["_uuid".into(), "count".into()], rows })
        }
        C::Matching { .. } => Err(not_written("count", "count (Matching filtré : l'évaluateur de read)")),
        C::EdgesMatching { .. } => Err(not_written("count", "count (EdgesMatching : l'évaluateur de read)")),
    }
}

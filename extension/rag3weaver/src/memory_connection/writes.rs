//! The family `writes` of the memory connection (owner: codeparsers). See [`crate::memory_dialect`].
//!
//! The writes of the IR that are neither an upsert nor a link, as rag3db translates them (`write_by_methods`):
//! - `Update { table, columns }`: `$items`, one map per row (`_uuid` and the columns), `MATCH … SET`: a row that is not
//!   there is skipped, the listed columns are set (a NULL takes the column away), the others keep their values;
//! - `Mark { table, set }`: `$uuids`, each field set to its named parameter, or emptied;
//! - `Delete { table, by, cascade, count }`: the rows whose `by` is in `$uuids`, with their edges (the image has no
//!   dangling edge: a plain delete takes them too); with `count`, one row per value that deleted something:
//!   `(value, count)`;
//! - `Unlink { relation }`: `$items`, one map per edge (`from`, `to`), the pair taken away, its ends kept;
//! - `Detach { relation, table, side, filter }`: the edges of `relation` whose node on `side` (`Outgoing`: the source,
//!   `Incoming`: the target) is a row of `table` that `filter` keeps, by the one evaluator `read::matches`.

use serde_json::Value;

use super::load::plain;
use super::{not_written, param, State};
use crate::connection::{CypherValue, DbError, QueryParam, QueryResult};

pub(super) fn execute(state: &mut State, op: &str, stmt: &Value, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    if op != "write" {
        return Err(not_written("writes", op));
    }
    let form: rag3weaver_ir::Write =
        serde_json::from_value(stmt["form"].clone()).map_err(|e| DbError::QueryError(format!("connexion mémoire : une écriture illisible ({e})")))?;
    use rag3weaver_ir::Write as W;
    match form {
        W::Update { table, columns } => update(state, &table, &columns, params),
        W::Mark { table, set } => mark(state, &table, &set, params),
        W::Delete { table, by, count, .. } => delete(state, &table, &by, count, params),
        W::Unlink { relation } => unlink(state, &relation, params),
        W::Detach { relation, table, side, filter } => detach(state, &relation, &table, side, &filter, params),
        other => Err(not_written("writes", &format!("write ({other:?})"))),
    }
}

fn list(params: &[QueryParam], name: &str, what: &str) -> Result<Vec<CypherValue>, DbError> {
    match param(params, name).map(plain) {
        Some(CypherValue::List(v)) => Ok(v),
        _ => Err(DbError::QueryError(format!("connexion mémoire : {what} attend ${name}, une liste"))),
    }
}

/// Set the fields of a row of `table` (a NULL takes the field away), keeping the others and its file.
fn set_fields(state: &mut State, table: &str, uuid: &str, fields: impl IntoIterator<Item = (String, CypherValue)>) {
    let Some(node) = state.image.node(uuid).filter(|n| n.table == table) else { return };
    let mut props = node.props;
    for (k, v) in fields {
        if matches!(v, CypherValue::Null) {
            props.remove(&k);
        } else {
            props.insert(k, v);
        }
    }
    state.image.insert_node(table, uuid, node.file.as_deref(), props);
}

fn update(state: &mut State, table: &str, columns: &[String], params: &[QueryParam]) -> Result<QueryResult, DbError> {
    for item in list(params, "items", &format!("la mise à jour de « {table} »"))? {
        let CypherValue::Map(item) = item else { continue };
        let Some(CypherValue::String(uuid)) = item.get("_uuid") else { continue };
        let fields: Vec<(String, CypherValue)> =
            columns.iter().filter(|c| c.as_str() != "_uuid").map(|c| (c.clone(), item.get(c).cloned().unwrap_or(CypherValue::Null))).collect();
        set_fields(state, table, uuid, fields);
    }
    Ok(QueryResult::default())
}

fn mark(state: &mut State, table: &str, set: &[(String, Option<String>)], params: &[QueryParam]) -> Result<QueryResult, DbError> {
    let fields: Vec<(String, CypherValue)> =
        set.iter().map(|(f, p)| (f.clone(), p.as_ref().and_then(|p| param(params, p)).map(plain).unwrap_or(CypherValue::Null))).collect();
    for u in list(params, "uuids", &format!("la marque de « {table} »"))? {
        if let CypherValue::String(u) = u {
            set_fields(state, table, &u, fields.clone());
        }
    }
    Ok(QueryResult::default())
}

fn delete(state: &mut State, table: &str, by: &str, count: bool, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    let mut rows = Vec::new();
    for value in list(params, "uuids", &format!("la suppression de « {table} »"))? {
        let doomed: Vec<String> = if by == "_uuid" {
            match &value {
                CypherValue::String(u) if state.image.node(u).is_some_and(|n| n.table == table) => vec![u.clone()],
                _ => Vec::new(),
            }
        } else {
            state.image.find(table, by, &value)
        };
        let n = doomed.iter().filter(|u| state.image.remove_node(u)).count();
        if count && n > 0 {
            rows.push(vec![value, CypherValue::Int(n as i64)]);
        }
    }
    Ok(if count { QueryResult { columns: vec!["uuid".into(), "cnt".into()], rows } } else { QueryResult::default() })
}

fn unlink(state: &mut State, relation: &str, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    for item in list(params, "items", &format!("la déliaison de « {relation} »"))? {
        let CypherValue::Map(item) = item else { continue };
        if let (Some(CypherValue::String(from)), Some(CypherValue::String(to))) = (item.get("from"), item.get("to")) {
            state.image.remove_rel(relation, from, to);
        }
    }
    Ok(QueryResult::default())
}

fn detach(
    state: &mut State,
    relation: &str,
    table: &str,
    side: rag3weaver_ir::Direction,
    filter: &rag3weaver_ir::Predicate,
    params: &[QueryParam],
) -> Result<QueryResult, DbError> {
    let mut doomed: Vec<(String, String)> = Vec::new();
    for u in state.image.nodes_of_table(table) {
        let Some(node) = state.image.node(&u) else { continue };
        if !super::read::matches(state, filter, &u, &node.props, params)? {
            continue;
        }
        match side {
            rag3weaver_ir::Direction::Outgoing => doomed.extend(state.image.outgoing(relation, &u).into_iter().map(|(to, _)| (u.clone(), to))),
            rag3weaver_ir::Direction::Incoming => doomed.extend(state.image.incoming(relation, &u).into_iter().map(|(from, _)| (from, u.clone()))),
        }
    }
    for (from, to) in doomed {
        state.image.remove_rel(relation, &from, &to);
    }
    Ok(QueryResult::default())
}

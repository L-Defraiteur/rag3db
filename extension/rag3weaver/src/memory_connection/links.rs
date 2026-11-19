//! The family `links` of the memory connection (owner: codeparsers). See [`crate::memory_dialect`].
//!
//! - `write` (`Write::Link`) and `batch_link`: `$items`, one map per edge (`from_uuid`, `to_uuid`, the properties), as
//!   `MATCH … MERGE (a)-[r]->(b) SET r.c = item.c`: a pair with a missing end is skipped, a pair already there is not
//!   doubled and takes the listed properties, the others keep theirs; in the batch, the last occurrence wins.
//! - `existing_links`: for each source of `$froms`, its edges of the relation toward the declared end table, with the
//!   asked properties after the two ends.

use std::collections::BTreeMap;

use serde_json::Value;

use super::load::{plain, untyped};
use super::{not_written, param, State};
use crate::connection::{CypherValue, DbError, QueryParam, QueryResult};

pub(super) fn execute(state: &mut State, op: &str, stmt: &Value, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    match op {
        "write" => {
            let form = &stmt["form"]["Link"];
            if form.is_null() {
                return Err(not_written("links", "write (une forme autre que Write::Link)"));
            }
            link(state, text(&form["relation"]), &strings(&form["props"]), params)
        }
        "batch_link" => link(state, text(&stmt["relation"]), &strings(&stmt["props"]), params),
        "existing_links" => existing(state, text(&stmt["relation"]), &stmt["ends"], &strings(&stmt["props"]), params),
        _ => Err(not_written("links", op)),
    }
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

fn link(state: &mut State, relation: &str, props: &[String], params: &[QueryParam]) -> Result<QueryResult, DbError> {
    let Some(CypherValue::List(items)) = param(params, "items").map(untyped) else {
        return Err(DbError::QueryError(format!("connexion mémoire : la pose de « {relation} » attend $items, une liste")));
    };
    for item in items {
        let CypherValue::Map(item) = untyped(item) else {
            return Err(DbError::QueryError(format!("connexion mémoire : un élément de $items de « {relation} » n'est pas une carte")));
        };
        let (Some(CypherValue::String(from)), Some(CypherValue::String(to))) = (item.get("from_uuid").map(untyped), item.get("to_uuid").map(untyped)) else {
            return Err(DbError::QueryError(format!("connexion mémoire : un lien de « {relation} » sans from_uuid ni to_uuid")));
        };
        // `MERGE … SET r.c = item.c` sets the listed properties only (a missing one becomes NULL); the others keep theirs.
        let mut merged: BTreeMap<String, CypherValue> =
            state.image.outgoing(relation, from).into_iter().find(|(t, _)| t == to).map(|(_, p)| p).unwrap_or_default();
        for p in props {
            match item.get(p).map(untyped) {
                Some(CypherValue::Null) | None => {
                    merged.remove(p);
                }
                Some(v) => {
                    merged.insert(p.clone(), plain(v));
                }
            }
        }
        state.image.insert_rel(relation, from, to, merged);
    }
    Ok(QueryResult::default())
}

fn existing(state: &mut State, relation: &str, ends: &Value, props: &[String], params: &[QueryParam]) -> Result<QueryResult, DbError> {
    let to_table = ends.get(1).and_then(Value::as_str);
    let Some(CypherValue::List(froms)) = param(params, "froms").map(untyped) else {
        return Err(DbError::QueryError(format!("connexion mémoire : les liens de « {relation} » attendent $froms, une liste")));
    };
    let mut rows = Vec::new();
    for from in froms {
        let CypherValue::String(from) = untyped(from) else { continue };
        for (to, p) in state.image.outgoing(relation, from) {
            if to_table.is_some_and(|t| state.image.node(&to).is_none_or(|n| n.table != t)) {
                continue;
            }
            let mut row = vec![CypherValue::String(from.clone()), CypherValue::String(to)];
            row.extend(props.iter().map(|k| p.get(k).cloned().unwrap_or(CypherValue::Null)));
            rows.push(row);
        }
    }
    let mut columns = vec!["a._uuid".to_string(), "b._uuid".to_string()];
    columns.extend(props.iter().map(|p| format!("r.{p}")));
    Ok(QueryResult { columns, rows })
}

//! The family `load` of the memory connection (owner: codeparsers). See [`crate::memory_dialect`].
//!
//! No bulk load here: the memory dialect refuses `Write::Load` (a CSV has no types), and the writers fall back on the
//! typed upsert. `write` (`Write::Upsert`) and `batch_upsert` take `$items`, one map per row, as
//! `MERGE (n {_uuid}) SET n.c = item.c RETURN ID(n), item._uuid`: a row already there takes the listed columns and
//! keeps the others. The id given back is `table:uuid`. A row's file is its `file_path`, else its `path` (the code
//! schema: `Scope` and `File`).

use std::collections::BTreeMap;

use serde_json::Value;

use super::{not_written, param, State};
use crate::connection::{CypherValue, DbError, QueryParam, QueryResult};

pub(super) fn execute(state: &mut State, op: &str, stmt: &Value, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    match op {
        "write" => {
            let form = &stmt["form"]["Upsert"];
            if form.is_null() {
                return Err(not_written("load", "write (une forme autre que Write::Upsert)"));
            }
            upsert(state, form["table"].as_str().unwrap_or(""), &strings(&form["columns"]), params)
        }
        "batch_upsert" => upsert(state, stmt["table"].as_str().unwrap_or(""), &strings(&stmt["columns"]), params),
        _ => Err(not_written("load", op)),
    }
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

/// A parameter value without its type annotation (`typed_payload_param`), lists and maps included.
pub(super) fn plain(v: &CypherValue) -> CypherValue {
    match v {
        CypherValue::Typed { value, .. } => plain(value),
        CypherValue::List(items) => CypherValue::List(items.iter().map(plain).collect()),
        CypherValue::Map(m) => CypherValue::Map(m.iter().map(|(k, v)| (k.clone(), plain(v))).collect()),
        other => other.clone(),
    }
}

fn upsert(state: &mut State, table: &str, columns: &[String], params: &[QueryParam]) -> Result<QueryResult, DbError> {
    let Some(CypherValue::List(items)) = param(params, "items").map(plain) else {
        return Err(DbError::QueryError(format!("connexion mémoire : l'upsert de « {table} » attend $items, une liste")));
    };
    let mut rows = Vec::with_capacity(items.len());
    for item in items {
        let CypherValue::Map(item) = item else {
            return Err(DbError::QueryError(format!("connexion mémoire : un élément de $items de « {table} » n'est pas une carte")));
        };
        let Some(CypherValue::String(uuid)) = item.get("_uuid") else {
            return Err(DbError::QueryError(format!("connexion mémoire : une ligne de « {table} » sans _uuid")));
        };
        let mut props: BTreeMap<String, CypherValue> = state.image.node(uuid).filter(|n| n.table == table).map(|n| n.props).unwrap_or_default();
        for c in columns.iter().filter(|c| c.as_str() != "_uuid") {
            match item.get(c) {
                Some(CypherValue::Null) | None => {
                    props.remove(c);
                }
                Some(v) => {
                    props.insert(c.clone(), v.clone());
                }
            }
        }
        let file = ["file_path", "path"].iter().find_map(|f| props.get(*f).and_then(|v| v.as_str()).map(str::to_string));
        state.image.insert_node(table, uuid, file.as_deref(), props);
        rows.push(vec![CypherValue::String(format!("{table}:{uuid}")), CypherValue::String(uuid.clone())]);
    }
    Ok(QueryResult { columns: vec!["id".into(), "_uuid".into()], rows })
}

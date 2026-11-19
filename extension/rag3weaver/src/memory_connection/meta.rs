//! The family `meta` of the memory connection (owner: recherche). See [`crate::memory_dialect`].
//!
//! The `_catalog_meta` table as a map, the cell nodes (`_Org`, `_Project`) as nodes of the image, and the count of a
//! table's rows.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{not_written, param, State};
use crate::connection::{CypherValue, DbError, QueryParam, QueryResult};

fn text_param(params: &[QueryParam], stmt: &Value, key: &str) -> Result<String, DbError> {
    let name = stmt[key].as_str().unwrap_or_default();
    match param(params, name) {
        Some(CypherValue::String(s)) => Ok(s.clone()),
        Some(other) => Ok(format!("{other:?}")),
        None => Err(DbError::QueryError(format!("connexion mémoire : paramètre « {name} » absent ({key})"))),
    }
}

pub(super) fn execute(state: &mut State, op: &str, stmt: &Value, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    match op {
        "upsert_meta" => {
            let key = text_param(params, stmt, "key")?;
            let value = text_param(params, stmt, "value")?;
            state.meta.insert(key, value);
            Ok(QueryResult::default())
        }
        "load_meta_by_prefix" => {
            let prefix = text_param(params, stmt, "prefix")?;
            let rows = state
                .meta
                .range(prefix.clone()..)
                .take_while(|(k, _)| k.starts_with(&prefix))
                .map(|(k, v)| vec![CypherValue::String(k.clone()), CypherValue::String(v.clone())])
                .collect();
            Ok(QueryResult { columns: vec!["m._key".into(), "m._value".into()], rows })
        }
        "upsert_scope_node" => {
            let table = stmt["table"].as_str().unwrap_or_default();
            let id = text_param(params, stmt, "id")?;
            if state.image.node(&id).is_none() {
                state.image.insert_node(table, &id, None, BTreeMap::from([("name".to_string(), CypherValue::String(id.clone()))]));
            }
            Ok(QueryResult::default())
        }
        "count_rows" => {
            let table = stmt["table"].as_str().unwrap_or_default();
            let n = state.image.nodes_of_table(table).len() as i64;
            Ok(QueryResult { columns: vec!["cnt".into()], rows: vec![vec![CypherValue::Int(n)]] })
        }
        other => Err(not_written("meta", other)),
    }
}

//! The family `aside` of the memory connection (owner: codeparsers). See [`crate::memory_dialect`].
//!
//! The copies set aside by a sync, one per key, kept in the image as rows of `_snapshot_aside` (uuid
//! `\u{1}aside:<key>`, no file). A copy is alive while its `_row` is not empty: `select_aside` (`$keys`) and
//! `select_aside_by_session` (`$entity`, `$session`) read the live ones (`_uuid`, `_content_hash`, `_row`, `_chunks`,
//! `_session`); `clear_aside` (`$keys`) empties `_row` and `_chunks`; `purge_aside_before` empties at most `limit`
//! copies of `$entity` set aside before `$before`, and gives back how many.

use std::collections::BTreeMap;

use serde_json::Value;

use super::load::plain;
use super::{not_written, param, State};
use crate::connection::{CypherValue, DbError, QueryParam, QueryResult};

const TABLE: &str = "_snapshot_aside";

fn uuid_of(key: &str) -> String {
    format!("\u{1}aside:{key}")
}

fn s(v: &str) -> CypherValue {
    CypherValue::String(v.to_string())
}

fn list(params: &[QueryParam], name: &str, op: &str) -> Result<Vec<CypherValue>, DbError> {
    match param(params, name).map(plain) {
        Some(CypherValue::List(v)) => Ok(v),
        _ => Err(DbError::QueryError(format!("connexion mémoire : « {op} » attend ${name}, une liste"))),
    }
}

fn text(params: &[QueryParam], name: &str) -> String {
    param(params, name).map(plain).and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

fn alive(props: &BTreeMap<String, CypherValue>) -> bool {
    props.get("_row").and_then(|v| v.as_str()).is_some_and(|r| !r.is_empty())
}

fn row_of(props: &BTreeMap<String, CypherValue>) -> Vec<CypherValue> {
    ["_uuid", "_content_hash", "_row", "_chunks", "_session"].iter().map(|k| props.get(*k).cloned().unwrap_or(CypherValue::Null)).collect()
}

fn result(rows: Vec<Vec<CypherValue>>) -> QueryResult {
    QueryResult { columns: ["a._uuid", "a._content_hash", "a._row", "a._chunks", "a._session"].map(String::from).to_vec(), rows }
}

fn clear(state: &mut State, uuid: &str) {
    let Some(n) = state.image.node(uuid) else { return };
    let mut props = n.props;
    props.insert("_row".into(), s(""));
    props.insert("_chunks".into(), s(""));
    state.image.insert_node(TABLE, uuid, None, props);
}

pub(super) fn execute(state: &mut State, op: &str, stmt: &Value, params: &[QueryParam]) -> Result<QueryResult, DbError> {
    match op {
        "upsert_aside" => {
            for item in list(params, "items", op)? {
                let CypherValue::Map(item) = item else { continue };
                let Some(key) = item.get("key").and_then(|v| v.as_str()) else { continue };
                let uuid = uuid_of(key);
                let mut props = state.image.node(&uuid).map(|n| n.props).unwrap_or_default();
                props.insert("_key".into(), s(key));
                for (field, column) in [("entity", "_entity"), ("uuid", "_uuid"), ("hash", "_content_hash"), ("row", "_row"), ("chunks", "_chunks"), ("session", "_session"), ("at", "_removed_at")] {
                    props.insert(column.into(), item.get(field).cloned().unwrap_or(CypherValue::Null));
                }
                state.image.insert_node(TABLE, &uuid, None, props);
            }
            Ok(QueryResult::default())
        }
        "select_aside" => {
            let mut rows = Vec::new();
            for key in list(params, "keys", op)? {
                let Some(key) = key.as_str() else { continue };
                if let Some(n) = state.image.node(&uuid_of(key)).filter(|n| alive(&n.props)) {
                    rows.push(row_of(&n.props));
                }
            }
            Ok(result(rows))
        }
        "select_aside_by_session" => {
            let (entity, session) = (text(params, "entity"), text(params, "session"));
            let rows = state
                .image
                .nodes_of_table(TABLE)
                .iter()
                .filter_map(|u| state.image.node(u))
                .filter(|n| alive(&n.props) && n.props.get("_entity").and_then(|v| v.as_str()) == Some(&entity) && n.props.get("_session").and_then(|v| v.as_str()) == Some(&session))
                .map(|n| row_of(&n.props))
                .collect();
            Ok(result(rows))
        }
        "clear_aside" => {
            for key in list(params, "keys", op)? {
                if let Some(key) = key.as_str() {
                    clear(state, &uuid_of(key));
                }
            }
            Ok(QueryResult::default())
        }
        "purge_aside_before" => {
            let limit = stmt["limit"].as_u64().unwrap_or(0) as usize;
            let entity = text(params, "entity");
            let before = param(params, "before").map(plain);
            let older = |at: Option<&CypherValue>| match (at, before.as_ref()) {
                (Some(CypherValue::Int(a)), Some(CypherValue::Int(b))) => a < b,
                (Some(CypherValue::Float(a)), Some(CypherValue::Float(b))) => a < b,
                (Some(CypherValue::String(a)), Some(CypherValue::String(b))) => a < b,
                _ => false,
            };
            let chosen: Vec<String> = state
                .image
                .nodes_of_table(TABLE)
                .into_iter()
                .filter(|u| {
                    state.image.node(u).is_some_and(|n| alive(&n.props) && n.props.get("_entity").and_then(|v| v.as_str()) == Some(&entity) && older(n.props.get("_removed_at")))
                })
                .take(limit)
                .collect();
            for u in &chosen {
                clear(state, u);
            }
            Ok(QueryResult { columns: vec!["count(a)".into()], rows: vec![vec![CypherValue::Int(chosen.len() as i64)]] })
        }
        _ => Err(not_written("aside", op)),
    }
}

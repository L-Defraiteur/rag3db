//! The family `ddl` of the memory connection (owner: recherche). See [`crate::memory_dialect`].
//!
//! The image has no schema of its own: a node table records its columns, a relation table its two ends and its
//! properties (the links of `insert_rel` check nothing more). The meta, blob and aside tables need nothing.

use serde_json::Value;

use super::{not_written, RelTable, State};
use crate::connection::{DbError, QueryParam, QueryResult};

fn strings(v: &Value) -> Vec<String> {
    v.as_array().into_iter().flatten().filter_map(|s| s.as_str().map(str::to_string)).collect()
}

pub(super) fn execute(state: &mut State, op: &str, stmt: &Value, _params: &[QueryParam]) -> Result<QueryResult, DbError> {
    let text = |k: &str| stmt[k].as_str().unwrap_or_default().to_string();
    match op {
        "create_table" => {
            let columns = strings(&stmt["columns"]);
            state.node_tables.entry(text("name")).or_insert(columns);
        }
        "create_rel_table" => {
            let rel = RelTable { from: text("from"), to: text("to"), props: strings(&stmt["props"]) };
            state.rel_tables.entry(text("name")).or_insert(rel);
        }
        "alter_add_column_default" => {
            let columns = state.node_tables.entry(text("table")).or_default();
            let column = text("column");
            if !columns.contains(&column) {
                columns.push(column);
            }
        }
        "create_meta_table" | "create_blob_table" | "create_aside_table" => {}
        other => return Err(not_written("ddl", other)),
    }
    Ok(QueryResult::default())
}

//! **The memory connection** (B of the reference tools before the index, recherche and codeparsers, 18 November 2026):
//! a [`DbConnection`] that executes the statements of [`crate::memory_dialect::MemoryDialect`] on a
//! [`MemoryImage`]. A `Catalog` set on it (`set_dialect(MemoryDialect)`) ingests and resolves code into the image by
//! its own path; the reference tools read it the same way.
//!
//! [`DbConnection::execute_with_params`] parses `MEMOIRE:{"family", "op", …}` and switches by family, one module per
//! family (see the table of [`crate::memory_dialect`]). A family not written yet refuses by its name and its op; a
//! statement that is not of the memory dialect — the named refusal of a method it does not translate, or any other
//! text — is refused with the statement itself.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde_json::Value;

use crate::connection::{DbConnection, DbError, QueryParam, QueryResult};
use crate::memory_dialect::MEMORY_PREFIX;
use crate::memory_image::MemoryImage;

mod aside;
mod count;
mod ddl;
mod links;
mod load;
mod meta;
mod read;

/// A declared relation table: its two ends and its properties.
// Read by the families as they are written (the skeleton refuses them all).
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub(crate) struct RelTable {
    pub from: String,
    pub to: String,
    pub props: Vec<String>,
}

/// What the connection holds: the image, the meta, and the tables the DDL declared.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct State {
    pub image: MemoryImage,
    pub meta: BTreeMap<String, String>,
    pub node_tables: BTreeMap<String, Vec<String>>,
    pub rel_tables: BTreeMap<String, RelTable>,
}

/// The memory connection. See the module.
#[derive(Debug, Default)]
pub struct MemoryConnection {
    state: Mutex<State>,
}

impl MemoryConnection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the image (for a probe or a witness).
    pub fn with_image<T>(&self, f: impl FnOnce(&MemoryImage) -> T) -> T {
        f(&self.state.lock().unwrap_or_else(|e| e.into_inner()).image)
    }

    /// Change the image (for a witness, or for codeparsers' invalidation by file).
    pub fn with_image_mut<T>(&self, f: impl FnOnce(&mut MemoryImage) -> T) -> T {
        f(&mut self.state.lock().unwrap_or_else(|e| e.into_inner()).image)
    }
}

/// A family not written yet, refused by its name and its op.
pub(crate) fn not_written(family: &str, op: &str) -> DbError {
    DbError::QueryError(format!("connexion mémoire : la famille « {family} » n'est pas encore écrite (op « {op} »)"))
}

/// The value of a named parameter.
#[allow(dead_code)]
pub(crate) fn param<'a>(params: &'a [QueryParam], name: &str) -> Option<&'a crate::connection::CypherValue> {
    params.iter().find(|p| p.name == name).map(|p| &p.value)
}

impl DbConnection for MemoryConnection {
    fn execute(&self, statement: &str) -> Result<QueryResult, DbError> {
        self.execute_with_params(statement, &[])
    }

    fn execute_with_params(&self, statement: &str, params: &[QueryParam]) -> Result<QueryResult, DbError> {
        let Some(json) = statement.strip_prefix(MEMORY_PREFIX) else {
            return Err(DbError::QueryError(format!("connexion mémoire : instruction hors du dialecte mémoire, refusée : {statement}")));
        };
        let stmt: Value = serde_json::from_str(json).map_err(|e| DbError::QueryError(format!("connexion mémoire : instruction illisible ({e}) : {json}")))?;
        let family = stmt["family"].as_str().unwrap_or("");
        let op = stmt["op"].as_str().unwrap_or("");
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        match family {
            "ddl" => ddl::execute(&mut state, op, &stmt, params),
            "meta" => meta::execute(&mut state, op, &stmt, params),
            "read" => read::execute(&mut state, op, &stmt, params),
            "load" => load::execute(&mut state, op, &stmt, params),
            "count" => count::execute(&mut state, op, &stmt, params),
            "links" => links::execute(&mut state, op, &stmt, params),
            "aside" => aside::execute(&mut state, op, &stmt, params),
            other => Err(not_written(other, op)),
        }
    }

    // One writer, in memory: a transaction holds by construction; nothing is undone on a rollback, and it says so.
    fn begin(&self) -> Result<(), DbError> {
        Ok(())
    }
    fn begin_read_only(&self) -> Result<(), DbError> {
        Ok(())
    }
    fn commit(&self) -> Result<(), DbError> {
        Ok(())
    }
    fn rollback(&self) -> Result<(), DbError> {
        Err(DbError::QueryError("connexion mémoire : un ROLLBACK ne défait rien ici ; l'image est à rebâtir".into()))
    }
}

#[cfg(test)]
mod tests;

/// **No checkpoints in memory**: an image is rebuilt, never resumed — an ingestion interrupted in memory starts over.
/// This store keeps nothing and finds nothing incomplete; [`crate::memory_dialect::MemoryDialect`] hands it to the
/// catalog, which otherwise refuses to start without a durable store (`DurableStoreMissing`).
#[derive(Debug, Default)]
pub struct NoCheckpoints;

impl crate::dataflow::checkpoint::CheckpointStore for NoCheckpoints {
    fn initialize(&self) -> Result<(), String> {
        Ok(())
    }
    fn create_execution(&self, _checkpoint: &crate::dataflow::checkpoint::ExecutionCheckpoint) -> Result<(), String> {
        Ok(())
    }
    fn load_execution(&self, _execution_id: &str) -> Result<Option<crate::dataflow::checkpoint::ExecutionCheckpoint>, String> {
        Ok(None)
    }
    fn find_incomplete(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
    fn save_node_completed(
        &self,
        _execution_id: &str,
        _node_name: &str,
        _outputs: &std::collections::HashMap<String, crate::dataflow::checkpoint::CheckpointPortValue>,
        _undo_context: Option<&serde_json::Value>,
        _duration_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
    fn save_node_failed(&self, _execution_id: &str, _node_name: &str, _error: &str) -> Result<(), String> {
        Ok(())
    }
    fn mark_completed(&self, _execution_id: &str) -> Result<(), String> {
        Ok(())
    }
    fn mark_failed(&self, _execution_id: &str, _error: &str) -> Result<(), String> {
        Ok(())
    }
    fn delete(&self, _execution_id: &str) -> Result<(), String> {
        Ok(())
    }
}

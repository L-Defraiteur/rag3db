//! The family `read` of the memory connection (owner: recherche). See [`crate::memory_dialect`].

use serde_json::Value;

use super::{not_written, State};
use crate::connection::{DbError, QueryParam, QueryResult};

pub(super) fn execute(_state: &mut State, op: &str, _stmt: &Value, _params: &[QueryParam]) -> Result<QueryResult, DbError> {
    Err(not_written("read", op))
}

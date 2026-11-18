//! The family `count` of the memory connection (owner: codeparsers). See [`crate::memory_dialect`].

use serde_json::Value;

use super::{not_written, State};
use crate::connection::{DbError, QueryParam, QueryResult};

pub(super) fn execute(_state: &mut State, op: &str, _stmt: &Value, _params: &[QueryParam]) -> Result<QueryResult, DbError> {
    Err(not_written("count", op))
}

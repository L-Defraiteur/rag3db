//! **A run as a tree, read-only** (`GET /ide/run?backend=&run=`; the IDE's agent page and the
//! reviewer, 21 Nov 2026). A root run's journal holds its events and its children's: this module
//! gives them back as one tree, each run with its nodes played, its model calls and its tool calls
//! (arguments, result excerpt, outcome, duration). It writes nothing.

use serde_json::{json, Value};

use super::DaemonHost;

impl DaemonHost {
    /// The tree of the run `run` of the backend `backend`, from its journal.
    pub fn run_tree(&self, backend: &str, run: &str) -> Result<Value, String> {
        let _ = (backend, run);
        Err("not yet".into())
    }
}

#[cfg(test)]
#[path = "run_tree_tests.rs"]
mod tests;

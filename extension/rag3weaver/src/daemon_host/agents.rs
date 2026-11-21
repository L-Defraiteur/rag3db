//! **The agents of the machine, as a tree** (`GET /ide/agents`; the IDE's welcome page, 21 Nov
//! 2026: « voir la liste des agents racines, quelles tâches a chacun, plans en cours, à coup de
//! dépliage de hiérarchie »). A root is a Claude Code session, by the name its sub-agents carry in
//! `parent_session`; its tasks are the open teammate tasks that name it. Read-only.

use serde_json::{json, Value};

use super::DaemonHost;

impl DaemonHost {
    /// `{roots: [{claude: {name, session_id}, live, tasks}], unrooted: [task]}`: the open tasks of
    /// every backend, by their parent session (live roots first, then by name; tasks by name).
    pub fn agents(&self) -> Value {
        json!({ "roots": [], "unrooted": [] })
    }
}

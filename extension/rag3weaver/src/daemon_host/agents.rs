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
        // The live sessions' Claude names: the roots that are live, and their session id.
        let vivantes: std::collections::BTreeMap<String, Option<String>> = self
            .sessions
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .filter_map(|s| s.claude_name.clone().map(|n| (n, s.claude_session.clone())))
            .collect();
        let mut taches: Vec<super::taches::Tache> = self.taches.lock().unwrap_or_else(|p| p.into_inner()).values().cloned().collect();
        taches.sort_by(|a, b| a.nom.cmp(&b.nom));
        let mut racines: std::collections::BTreeMap<String, Vec<Value>> = std::collections::BTreeMap::new();
        let mut sans_racine = Vec::new();
        for t in &taches {
            let rendu = self.agent_task(t);
            match t.parent_session.as_deref().filter(|p| !p.is_empty()) {
                Some(p) => racines.entry(p.to_string()).or_default().push(rendu),
                None => sans_racine.push(rendu),
            }
        }
        let mut roots: Vec<Value> = racines
            .into_iter()
            .map(|(nom, tasks)| {
                let session = vivantes.get(&nom);
                json!({
                    "claude": { "name": nom, "session_id": session.cloned().flatten() },
                    "live": session.is_some(),
                    "tasks": tasks,
                })
            })
            .collect();
        // Live roots first, then by name (the map already gave the names in order).
        roots.sort_by_key(|r| r["live"] != json!(true));
        json!({ "roots": roots, "unrooted": sans_racine })
    }

    /// One task of the tree: its backend and repository, whether a step runs, its plan's doing
    /// item, its views, and the last line of its mailbox.
    fn agent_task(&self, t: &super::taches::Tache) -> Value {
        let files = self.teammate_files(&t.parent, &t.nom).ok();
        let doing = files.as_ref().and_then(|f| {
            let plans = f.read_plans();
            let plan = plans.active()?;
            plan.items
                .iter()
                .find(|i| i.status == super::teammate::Status::Doing)
                .map(|i| json!({ "id": i.id, "text": i.text }))
        });
        let last_line = files.as_ref().and_then(|f| f.mailbox().last().map(|l| json!({ "at": l["at"], "from": l["from"], "kind": l["kind"] })));
        json!({
            "backend": t.parent,
            "repo": t.depot.file_name().map(|n| n.to_string_lossy().to_string()),
            "name": t.nom,
            // Open only, until a finished task keeps its folder and its parent (page of the reviewer).
            "state": "open",
            "running": self.teammate_running(&t.parent, &t.nom),
            "doing": doing,
            "views": t.vues,
            "active_view": t.vue_active,
            "last_line": last_line,
        })
    }
}

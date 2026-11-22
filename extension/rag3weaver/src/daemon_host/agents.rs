//! **The agents of the machine, as a tree** (`GET /ide/agents`; the IDE's welcome page, 21 Nov
//! 2026: « voir la liste des agents racines, quelles tâches a chacun, plans en cours, à coup de
//! dépliage de hiérarchie »). A root is a Claude Code session, by the name its sub-agents carry in
//! `parent_session`; its tasks are the open teammate tasks that name it. Read-only.

use serde_json::{json, Value};

use super::DaemonHost;

impl DaemonHost {
    /// `{roots: [{claude: {name, session_id}, live, tasks}], unrooted: [task]}`: the open tasks of
    /// every backend, by their parent session (live roots first, then by name; tasks by name), then
    /// its closed tasks (`state` rejected, merged or orphan; the latest closed first).
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
        // The closed agents, under their root after its open tasks, the latest closed first: an agent can be reopened
        // from its archive (`--pop-agent`, 22 November 2026).
        let mut fermees: Vec<Value> = self.taches_fermees.lock().unwrap_or_else(|p| p.into_inner()).iter().map(closed_agent_task).collect();
        fermees.sort_by_key(|t| std::cmp::Reverse(t["closed_ms"].as_u64().unwrap_or(0)));
        for (rendu, racine) in fermees.into_iter().map(|t| {
            let racine = t["parent_session"].as_str().filter(|p| !p.is_empty()).map(str::to_string);
            (t, racine)
        }) {
            match racine {
                Some(p) => racines.entry(p).or_default().push(rendu),
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
        let derniere = files.as_ref().and_then(|f| f.mailbox().last().cloned());
        let last_line = derniere.as_ref().map(|l| json!({ "at": l["at"], "from": l["from"], "kind": l["kind"] }));
        // The last activity: its last mailbox line, else its opening.
        let last_activity_ms = derniere.as_ref().and_then(|l| l["at"].as_u64()).unwrap_or(t.ouverte_ms).max(t.ouverte_ms);
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
            "agent_key": t.agent_key,
            "last_activity_ms": last_activity_ms,
        })
    }
}

/// One closed task of the tree, from its record in the registry (`fermees`) and its archive: its state, its agent key,
/// its closing, its archive and the last line of its archived mailbox, and whether it can be reopened (its archive is
/// there).
fn closed_agent_task(t: &Value) -> Value {
    let archive = t["archive"].as_str().map(std::path::PathBuf::from);
    let derniere = archive
        .as_ref()
        .and_then(|a| std::fs::read_to_string(a.join("mailbox.jsonl")).ok())
        .and_then(|texte| texte.lines().rev().find(|l| !l.trim().is_empty()).and_then(|l| serde_json::from_str::<Value>(l).ok()));
    let closed_ms = t["closed_ms"].as_u64();
    let last_activity_ms = derniere.as_ref().and_then(|l| l["at"].as_u64()).into_iter().chain(closed_ms).max();
    let state = match t["etat"].as_str() {
        Some("rejetee") => "rejected",
        Some("fusionnee") => "merged",
        Some("orpheline") => "orphan",
        _ => "closed",
    };
    json!({
        "backend": t["parent"],
        "name": t["nom"],
        "state": state,
        "parent_session": t["parent_session"],
        "agent_key": t["agent_key"],
        "views": t["views"],
        "closed_ms": closed_ms,
        "head": t["head"],
        "archive": t["archive"],
        "last_line": derniere.as_ref().map(|l| json!({ "at": l["at"], "from": l["from"], "kind": l["kind"] })),
        "last_activity_ms": last_activity_ms,
        "running": false,
        "reopenable": archive.as_ref().is_some_and(|a| a.is_dir()),
    })
}

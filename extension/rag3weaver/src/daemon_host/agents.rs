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
        let mut fermees: Vec<Value> = self.taches_fermees.lock().unwrap_or_else(|p| p.into_inner()).iter().map(|t| closed_agent_task(t, &machine())).collect();
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
        let machine = machine();
        let mut roots: Vec<Value> = racines
            .into_iter()
            .map(|(nom, tasks)| {
                let session = vivantes.get(&nom);
                // The root's last activity: its latest task's.
                let derniere = tasks.iter().max_by_key(|t| t["last_activity"]["at"].as_u64().unwrap_or(0)).map(|t| t["last_activity"].clone());
                json!({
                    "claude": { "name": nom, "session_id": session.cloned().flatten() },
                    "live": session.is_some(),
                    "id": format!("session:{nom}"),
                    "name": nom,
                    "state": if session.is_some() { "active" } else { "stopped" },
                    "machine": machine,
                    "parent": Value::Null,
                    "children": tasks.iter().map(|t| t["id"].clone()).collect::<Vec<_>>(),
                    "last_activity": derniere.unwrap_or_else(|| json!({ "at": null, "phrase": "aucune activité", "command": null })),
                    "unread": tasks.iter().filter_map(|t| t["unread"].as_u64()).sum::<u64>(),
                    "contacts": [],
                    "pending": tasks.iter().filter_map(|t| t["pending"].as_u64()).sum::<u64>(),
                    "tasks": tasks,
                })
            })
            .collect();
        // Live roots first, then by name (the map already gave the names in order).
        roots.sort_by_key(|r| r["live"] != json!(true));
        json!({ "roots": roots, "unrooted": sans_racine, "fields": fields() })
    }

    /// One task of the tree: its backend and repository, whether a step runs, its plan's doing
    /// item, its views, and the last line of its mailbox.
    fn agent_task(&self, t: &super::taches::Tache) -> Value {
        let files = self.teammate_files(&t.parent, &t.nom).ok();
        // The step of its plan (the left bar's second line): the item doing, else the first not done.
        let etape = files.as_ref().and_then(|f| {
            let plans = f.read_plans();
            let plan = plans.active()?;
            let (n, item) = plan
                .items
                .iter()
                .enumerate()
                .find(|(_, i)| i.status == super::teammate::Status::Doing)
                .or_else(|| plan.items.iter().enumerate().find(|(_, i)| i.status != super::teammate::Status::Done))?;
            Some(json!({ "current": n + 1, "total": plan.items.len(), "title": item.text }))
        });
        let doing = files.as_ref().and_then(|f| {
            let plans = f.read_plans();
            let plan = plans.active()?;
            plan.items
                .iter()
                .find(|i| i.status == super::teammate::Status::Doing)
                .map(|i| json!({ "id": i.id, "text": i.text }))
        });
        let lignes = files.as_ref().map(|f| f.mailbox()).unwrap_or_default();
        let derniere = lignes.last().cloned();
        let last_line = derniere.as_ref().map(|l| json!({ "at": l["at"], "from": l["from"], "kind": l["kind"] }));
        // The last activity: its last mailbox line, else its opening.
        let last_activity_ms = derniere.as_ref().and_then(|l| l["at"].as_u64()).unwrap_or(t.ouverte_ms).max(t.ouverte_ms);
        let id = t.agent_key.clone().unwrap_or_else(|| format!("task:{}/{}", t.parent, t.nom));
        let running = self.teammate_running(&t.parent, &t.nom);
        // Paused (archi's form): `<its folder>/paused.json`, or every agent by the host's `agents-paused.json`; its
        // pending lines are those after the teammate's reading cursor.
        let pause = files.as_ref().is_some_and(|f| f.dir.join("paused.json").is_file()) || all_paused();
        let pending = match (&files, pause) {
            (Some(f), true) => lignes.len().saturating_sub(f.read_cursor()),
            _ => 0,
        };
        let unread = files.as_ref().and_then(|f| f.supervisor().map(|s| f.pending_for(&s).len())).unwrap_or(0);
        let mut rendu = json!({
            "id": id,
            "state": if pause { "paused" } else if running { "active" } else { "waiting" },
            "machine": machine(),
            "parent": t.parent_session.as_deref().filter(|p| !p.is_empty()).map(|p| format!("session:{p}")),
            "children": [],
            "last_activity": last_activity(derniere.as_ref(), last_activity_ms),
            "unread": unread,
            "contacts": contacts(&lignes, &id),
            "pending": pending,
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
            "forked_from": t.forked_from,
        });
        if let Some(e) = etape {
            rendu["plan"] = e;
        }
        rendu
    }
}

/// One closed task of the tree, from its record in the registry (`fermees`) and its archive: its state, its agent key,
/// its closing, its archive and the last line of its archived mailbox, and whether it can be reopened (its archive is
/// there).
fn closed_agent_task(t: &Value, machine: &str) -> Value {
    let archive = t["archive"].as_str().map(std::path::PathBuf::from);
    let lignes: Vec<Value> = archive
        .as_ref()
        .and_then(|a| std::fs::read_to_string(a.join("mailbox.jsonl")).ok())
        .map(|texte| texte.lines().filter_map(|l| serde_json::from_str::<Value>(l).ok()).collect())
        .unwrap_or_default();
    let derniere = lignes.last().cloned();
    let closed_ms = t["closed_ms"].as_u64();
    let last_activity_ms = derniere.as_ref().and_then(|l| l["at"].as_u64()).into_iter().chain(closed_ms).max();
    let state = match t["etat"].as_str() {
        Some("rejetee") => "rejected",
        Some("fusionnee") => "merged",
        Some("orpheline") => "orphan",
        _ => "closed",
    };
    let id = t["agent_key"].as_str().map(str::to_string).unwrap_or_else(|| format!("task:{}/{}", t["parent"].as_str().unwrap_or_default(), t["nom"].as_str().unwrap_or_default()));
    json!({
        "id": id,
        "machine": machine,
        "parent": t["parent_session"].as_str().filter(|p| !p.is_empty()).map(|p| format!("session:{p}")),
        "children": [],
        "last_activity": last_activity(derniere.as_ref(), last_activity_ms.unwrap_or(0)),
        "unread": 0,
        "contacts": contacts(&lignes, &id),
        "pending": 0,
        "closed_state": state,
        "backend": t["parent"],
        "name": t["nom"],
        "state": "stopped",
        "parent_session": t["parent_session"],
        "agent_key": t["agent_key"],
        "views": t["views"],
        "closed_ms": closed_ms,
        "head": t["head"],
        "archive": t["archive"],
        "last_line": derniere.as_ref().map(|l| json!({ "at": l["at"], "from": l["from"], "kind": l["kind"] })),
        "running": false,
        "reopenable": archive.as_ref().is_some_and(|a| a.is_dir()),
    })
}

/// This host's machine: its hostname.
fn machine() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|h| h.trim().to_string())
        .ok()
        .filter(|h| !h.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "?".into())
}

/// Every agent paused (archi's form): `<the host's state folder>/agents-paused.json`.
fn all_paused() -> bool {
    crate::daemon_lifecycle::log_path().parent().is_some_and(|d| d.join("agents-paused.json").is_file())
}

/// `{at, phrase, command}`: the last activity of an agent, its short phrase and its command. Until archi's
/// `activity::phrase` reads the last action of the runs' journal, the phrase is the last mailbox line's kind and the
/// start of its text, and the command is null.
fn last_activity(derniere: Option<&Value>, at: u64) -> Value {
    let phrase = match derniere {
        Some(l) => {
            let texte: String = l["text"].as_str().unwrap_or_default().chars().take(60).collect();
            format!("{} : {}", l["kind"].as_str().unwrap_or("ligne"), texte.trim())
        }
        None => "ouvert, rien encore".to_string(),
    };
    json!({ "at": at, "phrase": phrase, "command": null })
}

/// The agents `own` exchanged lines with (a line from or to an `agent:` key other than its own), the latest first.
fn contacts(lignes: &[Value], own: &str) -> Value {
    // By contact: the time and the line of the last message exchanged (a later line replaces an earlier one).
    let mut derniers: std::collections::BTreeMap<String, (u64, &Value)> = std::collections::BTreeMap::new();
    for l in lignes {
        let at = l["at"].as_u64().unwrap_or(0);
        for k in ["from", "to"] {
            if let Some(autre) = l[k].as_str().filter(|a| a.starts_with("agent:") && *a != own) {
                let e = derniers.entry(autre.to_string()).or_insert((0, l));
                if at >= e.0 {
                    *e = (at, l);
                }
            }
        }
    }
    let mut v: Vec<(String, (u64, &Value))> = derniers.into_iter().collect();
    v.sort_by_key(|(_, (at, _))| std::cmp::Reverse(*at));
    json!(v
        .into_iter()
        .map(|(id, (at, l))| {
            // The sender of the last message: `teammate` is the agent itself.
            let from = match l["from"].as_str() {
                Some("teammate") => own.to_string(),
                Some(f) => f.to_string(),
                None => String::new(),
            };
            json!({ "id": id, "last_message_at": at, "last_message": { "from": from, "text": couper(l["text"].as_str().unwrap_or_default(), 120) } })
        })
        .collect::<Vec<_>>())
}

/// `texte` cut at `n` characters, an ellipsis said when it was longer.
fn couper(texte: &str, n: usize) -> String {
    let texte = texte.trim();
    if texte.chars().count() <= n {
        texte.to_string()
    } else {
        format!("{}…", texte.chars().take(n).collect::<String>().trim_end())
    }
}

/// The fields of the left bar's contract, described in the answer (one name a field, in English).
fn fields() -> Value {
    json!({
        "id": "the agent's identity: its agent key (agent:<key>) for a task, session:<name> for a root",
        "name": "the agent's name: its task's name, or the Claude session's",
        "state": "active (a step runs, or a live root), paused (paused.json, or every agent paused), stopped (closed, or a root no longer live), waiting (open, no step running)",
        "machine": "the hostname of the host that runs the agent",
        "parent": "the id of the root that launched the task (session:<name>), null for a root or a task without a parent",
        "children": "the ids of the agents it launched",
        "last_activity": "{at: ms, phrase: a short phrase of its last action, command: the command of that action or null}",
        "unread": "the lines of its mailbox awaiting its supervisor (a question, a report not yet seen)",
        "contacts": "the agents it exchanged messages with, [{id, last_message_at, last_message: {from, text}}], the latest first; text cut at 120 characters, from the sender's id",
        "plan": "{current, total, title}: its plan's step doing (else its first not done), its number among the plan's items and its text; absent without a plan",
        "pending": "while paused, the lines of its mailbox after its reading cursor; else 0",
    })
}

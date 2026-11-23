//! **The agents of the machine, as a tree** (`GET /ide/agents`). A module of its own.

use super::*;

/// Two sub-agents of the session « archi » (one live host session carries that name), and a
/// teammate opened without a parent: the tree says the root, its tasks with their views and their
/// plan's doing item, and the unrooted task apart.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_agents_of_the_machine_are_a_tree_of_roots_and_tasks() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("no model here".into()) });
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let corps = json!({ "client": "mcp (pid 1)", "claude": { "session_id": "S-archi", "name": "archi" } }).to_string();
    let _ = host.handle("POST", "/session", None, corps.as_bytes());
    for nom in ["archi-b", "archi-a"] {
        host.ouvrir_une_tache(&h.id, nom).expect("la tâche s'ouvre");
        host.donner_les_vues(&h.id, nom, "archi", vec!["backend".into()], Some("backend".into())).unwrap();
    }
    host.ouvrir_une_tache(&h.id, "seule").expect("la tâche s'ouvre");
    let files = host.teammate_files(&h.id, "archi-a").unwrap();
    let plans: super::super::teammate::Plans = serde_json::from_value(json!({ "plans": [{ "number": 1, "request": "r", "proposal": "p", "state": "active",
        "items": [{ "id": "lire", "text": "Lire la porte", "status": "doing" }, { "id": "ecrire", "text": "Écrire", "status": "todo" }] }] })).unwrap();
    files.write_plans(&plans).unwrap();
    files.append("supervisor", "request", None, "Fais-le.").unwrap();
    let a = host.agents();
    let roots = a["roots"].as_array().expect("roots");
    assert_eq!(roots.len(), 1, "one root: {a:#}");
    assert_eq!(roots[0]["claude"], json!({ "name": "archi", "session_id": "S-archi" }));
    assert_eq!(roots[0]["live"], json!(true));
    let taches: Vec<&str> = roots[0]["tasks"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(taches, ["archi-a", "archi-b"], "its tasks, by name");
    let t = &roots[0]["tasks"][0];
    assert_eq!(t["backend"], json!(h.id));
    assert_eq!((&t["state"], &t["lifecycle"]), (&json!("waiting"), &json!("open")), "an open task waits, its lifecycle open: {t}");
    assert_eq!(t["running"], json!(false));
    assert_eq!(t["doing"], json!({ "id": "lire", "text": "Lire la porte" }));
    assert_eq!(t["views"], json!(["backend"]));
    assert_eq!(t["active_view"], json!("backend"));
    assert_eq!(t["last_line"]["kind"], json!("request"));
    assert_eq!(roots[0]["tasks"][1]["doing"], Value::Null, "no plan, no doing");
    let seules: Vec<&str> = a["unrooted"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(seules, ["seule"], "the task without a parent session, apart: {a:#}");
}

/// **The tree says each agent's last activity and its closed agents** (c7 for Lucie, 22 November 2026: reopen an agent
/// already launched, `--pop-agent`): an open task gives its agent key and `last_activity.at` (its last mailbox line, else
/// its opening); a task closed (rejected here) stays under its root, stopped (`lifecycle: "rejected"`), its agent key, its closing
/// time, its archive, the last line of its archived mailbox, and `reopenable: true` while its archive is there.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_agents_say_their_last_activity_and_their_closed_agents() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("no model here".into()) });
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    for nom in ["archi-vive", "archi-finie"] {
        host.ouvrir_une_tache(&h.id, nom).expect("la tâche s'ouvre");
        host.donner_les_vues(&h.id, nom, "archi", vec!["backend".into()], Some("backend".into())).unwrap();
    }
    let ligne = host.teammate_files(&h.id, "archi-vive").unwrap().append("supervisor", "request", None, "Fais-le.").unwrap();
    host.teammate_files(&h.id, "archi-finie").unwrap().append("supervisor", "request", None, "Et toi, ceci.").unwrap();
    // The task's id read first: the registry's lock is released before agent_key_of_backend takes it again.
    let id_finie = host.taches.lock().unwrap().values().find(|t| t.nom == "archi-finie").unwrap().id.clone();
    let cle_finie = host.agent_key_of_backend(&id_finie).expect("a key");
    host.rejeter_une_tache(&h.id, "archi-finie", "le témoin la ferme").expect("rejected");

    let a = host.agents();
    let taches = a["roots"][0]["tasks"].as_array().expect("the root's tasks");
    let vive = taches.iter().find(|t| t["name"] == "archi-vive").expect("the open task");
    assert!(vive["agent_key"].as_str().is_some_and(|k| k.starts_with("agent:")), "the open task says no agent key: {vive}");
    assert_eq!(vive["last_activity"]["at"], ligne["at"], "the open task's last activity is not its last mailbox line: {vive}");
    let finie = taches.iter().find(|t| t["name"] == "archi-finie").unwrap_or_else(|| panic!("the closed task is not under its root: {a:#}"));
    assert_eq!((&finie["state"], &finie["lifecycle"]), (&json!("stopped"), &json!("rejected")), "{finie}");
    assert_eq!(finie["agent_key"], json!(cle_finie), "{finie}");
    assert!(finie["closed_ms"].as_u64().is_some(), "{finie}");
    assert!(finie["archive"].as_str().is_some_and(|p| std::path::Path::new(p).is_dir()), "{finie}");
    assert_eq!(finie["last_line"]["kind"], json!("request"), "the closed task does not say its archived mailbox's last line: {finie}");
    assert!(finie["last_activity"]["at"].as_u64() >= finie["closed_ms"].as_u64(), "a closed task's last activity is its closing: {finie}");
    assert_eq!(finie["reopenable"], json!(true), "{finie}");
}

/// **The contract of the IDE's left bar** (c7 for Lucie, 22 November 2026, mocked by recherche): each agent of the tree,
/// root or task, says `id`, `name`, `state` (active, paused, stopped, waiting), `machine`, `parent`, `children`,
/// `last_activity: {at, phrase, command}`, `unread`, `contacts: [{id, last_message_at}]` and `pending`; the answer
/// describes its fields. A task is paused by `<its folder>/paused.json` (archi's form); a closed task is stopped.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_agents_give_the_contract_of_the_left_bar() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("no model here".into()) });
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let corps = json!({ "client": "mcp (pid 1)", "claude": { "session_id": "S-archi", "name": "archi" } }).to_string();
    let _ = host.handle("POST", "/session", None, corps.as_bytes());
    for nom in ["archi-a", "archi-b", "archi-c"] {
        host.ouvrir_une_tache(&h.id, nom).expect("la tâche s'ouvre");
        host.donner_les_vues(&h.id, nom, "archi", vec!["backend".into()], Some("backend".into())).unwrap();
    }
    let a = host.teammate_files(&h.id, "archi-a").unwrap();
    a.append_with("teammate", "message", None, "Tu as fini la porte ?", &json!({ "to": "agent:autre-123" })).unwrap();
    let reponse = a.append_with("agent:autre-123", "message", None, "Oui, elle est posée.", &json!({ "to": "teammate" })).unwrap();
    let b = host.teammate_files(&h.id, "archi-b").unwrap();
    b.append("supervisor", "request", None, "Première.").unwrap();
    b.append("supervisor", "request", None, "Seconde.").unwrap();
    std::fs::write(b.dir.join("paused.json"), json!({ "since_ms": 1, "by": "user" }).to_string()).unwrap();
    host.rejeter_une_tache(&h.id, "archi-c", "le témoin la ferme").unwrap();

    let tout = host.agents();
    for champ in ["id", "name", "state", "machine", "parent", "children", "last_activity", "unread", "contacts", "pending"] {
        assert!(tout["fields"][champ].as_str().is_some_and(|t| !t.is_empty()), "the answer does not describe {champ}: {}", tout["fields"]);
    }
    let racine = &tout["roots"][0];
    assert_eq!(racine["id"], json!("session:archi"), "{racine}");
    assert_eq!(racine["state"], json!("active"), "a live root is active: {racine}");
    let taches = racine["tasks"].as_array().unwrap();
    let par_nom = |n: &str| taches.iter().find(|t| t["name"] == n).unwrap_or_else(|| panic!("{n} is not in the tree: {tout:#}")).clone();
    let (ta, tb, tc) = (par_nom("archi-a"), par_nom("archi-b"), par_nom("archi-c"));
    let enfants: Vec<&Value> = racine["children"].as_array().expect("the root's children").iter().collect();
    for t in [&ta, &tb, &tc] {
        assert!(t["id"].as_str().is_some_and(|i| i.starts_with("agent:")), "an agent's id is its key: {t}");
        assert_eq!(t["parent"], json!("session:archi"), "{t}");
        assert_eq!(t["children"], json!([]), "{t}");
        assert!(t["machine"].as_str().is_some_and(|m| !m.is_empty()), "{t}");
        assert!(enfants.contains(&&t["id"]), "the root does not name {} among its children: {racine}", t["id"]);
        assert!(t["last_activity"]["phrase"].as_str().is_some_and(|p| !p.is_empty()), "no phrase: {t}");
        assert!(t["last_activity"].get("command").is_some(), "{t}");
    }
    assert_eq!(ta["state"], json!("waiting"), "an open task with no step running waits: {ta}");
    assert_eq!(ta["lifecycle"], json!("open"), "{ta}");
    assert_eq!(ta["last_activity"]["at"], reponse["at"], "{ta}");
    assert_eq!(ta["contacts"], json!([{ "id": "agent:autre-123", "last_message_at": reponse["at"] }]), "{ta}");
    assert_eq!(tb["state"], json!("paused"), "{tb}");
    assert_eq!(tb["pending"], json!(2), "a paused agent counts the lines after its reading cursor: {tb}");
    assert_eq!(ta["pending"], json!(0), "{ta}");
    assert_eq!(tc["state"], json!("stopped"), "a closed agent is stopped: {tc}");
    assert!(ta["unread"].as_u64().is_some(), "{ta}");
}

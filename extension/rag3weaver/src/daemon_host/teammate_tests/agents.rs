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
    assert_eq!(t["running"], json!(false));
    assert_eq!(t["doing"], json!({ "id": "lire", "text": "Lire la porte" }));
    assert_eq!(t["views"], json!(["backend"]));
    assert_eq!(t["active_view"], json!("backend"));
    assert_eq!(t["last_line"]["kind"], json!("request"));
    assert_eq!(roots[0]["tasks"][1]["doing"], Value::Null, "no plan, no doing");
    let seules: Vec<&str> = a["unrooted"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(seules, ["seule"], "the task without a parent session, apart: {a:#}");
}

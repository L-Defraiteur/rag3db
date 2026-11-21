//! **Closing a task keeps the agent** (Lucie, 21 Nov 2026: a finished sub-agent stays in its parent's
//! history and can be relaunched). A module of its own.

use super::*;

/// A rejected child task: its folder (mailbox, plan) is archived, not erased; the closed card
/// carries its parent session, views, active view, agent key, final plan, archive and branch head.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_closed_task_keeps_its_folder_and_its_card_says_who_it_was() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("no model here".into()) });
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let t = host.ouvrir_une_tache(&h.id, "archi-lot").expect("la tâche s'ouvre");
    host.donner_les_vues(&h.id, "archi-lot", "archi", vec!["backend".into()], Some("backend".into())).unwrap();
    let files = host.teammate_files(&h.id, "archi-lot").unwrap();
    files.append("supervisor", "request", None, "Fais-le.").unwrap();
    let plans: super::super::teammate::Plans = serde_json::from_value(json!({ "plans": [{ "number": 1, "request": "r", "proposal": "p", "state": "active",
        "items": [{ "id": "lire", "text": "Lire", "status": "doing" }] }] })).unwrap();
    files.write_plans(&plans).unwrap();
    let tete = String::from_utf8(std::process::Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&t.arbre).output().unwrap().stdout).unwrap().trim().to_string();
    let dossier = t.arbre.parent().unwrap().to_path_buf();
    let fin = host.rejeter_une_tache(&h.id, "archi-lot", "plus utile").unwrap();
    assert!(!dossier.exists(), "the task's folder left its place");
    let archive = std::path::PathBuf::from(fin["archive"].as_str().expect("the card names its archive"));
    assert!(archive.join("mailbox.jsonl").is_file() && archive.join("plan.json").is_file(), "mailbox and plan kept: {}", archive.display());
    assert!(std::fs::read_to_string(archive.join("mailbox.jsonl")).unwrap().contains("Fais-le."));
    assert_eq!(fin["parent_session"], json!("archi"));
    assert_eq!(fin["views"], json!(["backend"]));
    assert_eq!(fin["active_view"], json!("backend"));
    assert_eq!(fin["agent_key"], json!(t.agent_key));
    assert!(fin["agent_key"].as_str().is_some_and(|k| k.starts_with("agent:")), "{fin}");
    assert_eq!(fin["head"], json!(tete), "the branch's head, to find it again by the reflog");
    assert_eq!(fin["plan"]["plans"][0]["items"][0]["id"], json!("lire"), "the final plan");
    let fermees = host.taches_fermees_de(&h.id);
    assert!(fermees.iter().any(|f| f["nom"] == "archi-lot" && f["archive"] == fin["archive"]), "the registry keeps the card: {fermees:#?}");
}

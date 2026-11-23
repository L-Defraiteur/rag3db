//! **Reopen a closed agent** (`--pop-agent`, c7 for Lucie, 22 November 2026): a new task that takes back the closed
//! task's agent key, its conversation, its mailbox and its plan, its branch starting from the closed head.

use super::*;

/// A task of the root « archi » works (a commit on its branch, two mailbox lines, a plan, a conversation), then is
/// rejected. Reopened under a new name, the new task carries the same agent key, its branch starts at the closed head,
/// its mailbox and its plan are the archived ones, and its conversation is the closed task's.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_closed_agent_reopens_with_its_key_its_head_its_mailbox_its_plan_and_its_conversation() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("no model here".into()) });
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.ouvrir_une_tache(&h.id, "archi-x").expect("la tâche s'ouvre");
    host.donner_les_vues(&h.id, "archi-x", "archi", vec!["backend".into()], Some("backend".into())).unwrap();
    let t = host.tache(&h.id, "archi-x").unwrap();
    let git = |dans: &std::path::Path, args: &[&str]| -> String {
        let o = std::process::Command::new("git").arg("-C").arg(dans).args(["-c", "user.name=Personne", "-c", "user.email=personne@example.org"]).args(args).output().unwrap();
        assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    };
    std::fs::write(t.arbre.join("travail.txt"), "le travail de l'agent\n").unwrap();
    git(&t.arbre, &["add", "travail.txt"]);
    git(&t.arbre, &["commit", "-q", "-m", "le travail"]);
    let tete = git(&t.arbre, &["rev-parse", "HEAD"]);
    assert_ne!(tete, git(&depot, &["rev-parse", "HEAD"]), "the task's head is the repository's");
    let files = host.teammate_files(&h.id, "archi-x").unwrap();
    files.append("supervisor", "request", None, "Écris le travail.").unwrap();
    files.append("teammate", "answer", None, "C'est écrit.").unwrap();
    let plans: super::super::teammate::Plans = serde_json::from_value(json!({ "plans": [{ "number": 1, "request": "r", "proposal": "p", "state": "active",
        "items": [{ "id": "ecrire", "text": "Écrire le travail", "status": "done" }] }] })).unwrap();
    files.write_plans(&plans).unwrap();
    let session = crate::chat::ChatSession { turns: vec![crate::llm::Turn::user("Bonjour"), crate::llm::Turn::assistant("Je m'y mets.")], ..Default::default() };
    std::fs::create_dir_all(d.path().join("chats")).unwrap();
    std::fs::write(d.path().join("chats").join(format!("{}.json", t.id)), json!({ "manifest": t.manifest, "session": session, "last_call": null }).to_string()).unwrap();
    host.reread_chats();
    let cle = t.agent_key.clone().expect("a key");
    host.rejeter_une_tache(&h.id, "archi-x", "le témoin la ferme").expect("rejected");

    let n = host.reopen_agent(&h.id, "archi-x", Some("archi-x2")).expect("the closed agent reopens");
    assert_eq!(n.nom, "archi-x2");
    assert_eq!(n.agent_key.as_deref(), Some(cle.as_str()), "the reopened agent has another key");
    assert_eq!(n.parent_session.as_deref(), Some("archi"), "the reopened agent left its root");
    assert_eq!(git(&n.arbre, &["rev-parse", "HEAD"]), tete, "the reopened agent's branch does not start at the closed head");
    assert!(n.arbre.join("travail.txt").is_file());
    let lignes: Vec<String> = host.teammate_files(&h.id, "archi-x2").unwrap().mailbox().iter().filter_map(|l| l["text"].as_str().map(str::to_string)).collect();
    assert!(lignes.iter().any(|l| l == "Écris le travail.") && lignes.iter().any(|l| l == "C'est écrit."), "the archived mailbox is not taken back: {lignes:?}");
    let repris = host.teammate_files(&h.id, "archi-x2").unwrap().read_plans();
    assert_eq!(repris.active().map(|p| p.items[0].id.clone()).as_deref(), Some("ecrire"), "the archived plan is not taken back");
    let etat = host.chat_state(&n.id).unwrap();
    let tours: Vec<&str> = etat["turns"].as_array().unwrap().iter().filter_map(|t| t["content"].as_str()).collect();
    assert_eq!(tours, ["Bonjour", "Je m'y mets."], "the conversation is not taken back: {etat}");
}

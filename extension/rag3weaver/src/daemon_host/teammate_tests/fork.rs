//! **Fork an agent** (`--fork-agent`, c7 for Lucie, 22 November 2026): a task-copy on a new work branch from the head
//! of the original, its mailbox, its plan and its conversation copied so far, then two independent agents.

use super::*;

/// The task « archi-x » works (a commit, two mailbox lines, a plan, a conversation) and holds a durable root granted by
/// the user, and a grant for one command. Forked as « archi-y »: the copy has a new agent key and says whose copy it
/// is (`forked_from`), its branch starts at the original's head, its mailbox, its plan and its conversation are the
/// original's so far, it inherits the durable root (said inherited, granted by the same) and not the grant for one
/// command; the original stays open, and a line written to it after the fork does not reach the copy.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn an_agent_forks_into_an_independent_copy_from_its_head() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
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
    let files = host.teammate_files(&h.id, "archi-x").unwrap();
    files.append("supervisor", "request", None, "Écris le travail.").unwrap();
    files.append("teammate", "answer", None, "C'est écrit.").unwrap();
    let plans: super::super::teammate::Plans = serde_json::from_value(json!({ "plans": [{ "number": 1, "request": "r", "proposal": "p", "state": "active",
        "items": [{ "id": "ecrire", "text": "Écrire le travail", "status": "doing" }] }] })).unwrap();
    files.write_plans(&plans).unwrap();
    let session = crate::chat::ChatSession { turns: vec![crate::llm::Turn::user("Bonjour"), crate::llm::Turn::assistant("Je m'y mets.")], ..Default::default() };
    std::fs::create_dir_all(d.path().join("chats")).unwrap();
    std::fs::write(d.path().join("chats").join(format!("{}.json", t.id)), json!({ "manifest": t.manifest, "session": session, "last_call": null }).to_string()).unwrap();
    host.reread_chats();
    let cle = t.agent_key.clone().expect("a key");
    let durable = d.path().join("durable");
    let une_fois = d.path().join("une-fois");
    std::fs::create_dir_all(&durable).unwrap();
    std::fs::create_dir_all(&une_fois).unwrap();
    let preuve = |q: &str| crate::daemon_host::root_grants::Proof { quote: q.into(), at: None, turn: None };
    host.grant_root(&cle, &durable, preuve("tu peux écrire dans durable"), "user", "write").unwrap();
    host.grant_once(&cle, &une_fois, preuve("lis-le une fois"), "user", "read").unwrap();

    let n = host.fork_agent(&h.id, "archi-x", Some("archi-y")).expect("the agent forks");
    assert_eq!(n.nom, "archi-y");
    let neuve = n.agent_key.clone().expect("the copy's key");
    assert!(neuve.starts_with("agent:") && neuve != cle, "the copy has the original's key: {neuve}");
    assert_eq!(n.forked_from, Some(json!({ "agent": cle, "task": "archi-x", "head": tete })), "the copy does not say whose copy it is");
    assert_eq!(n.parent_session.as_deref(), Some("archi"));
    assert_eq!(git(&n.arbre, &["rev-parse", "HEAD"]), tete, "the copy's branch does not start at the original's head");
    assert_ne!(n.branche, t.branche, "the copy shares the original's branch");
    let textes = |nom: &str| -> Vec<String> { host.teammate_files(&h.id, nom).unwrap().mailbox().iter().filter_map(|l| l["text"].as_str().map(str::to_string)).collect() };
    assert_eq!(textes("archi-y"), ["Écris le travail.", "C'est écrit."], "the mailbox is not copied");
    assert_eq!(host.teammate_files(&h.id, "archi-y").unwrap().read_plans().active().map(|p| p.items[0].id.clone()).as_deref(), Some("ecrire"));
    let tours: Vec<String> = host.chat_state(&n.id).unwrap()["turns"].as_array().unwrap().iter().filter_map(|t| t["content"].as_str().map(str::to_string)).collect();
    assert_eq!(tours, ["Bonjour", "Je m'y mets."], "the conversation is not copied");
    let herites: Vec<_> = host.root_grants(Some(&neuve)).into_iter().filter(|g| g.agent == neuve).collect();
    assert_eq!(herites.len(), 1, "the copy does not inherit exactly the durable root: {herites:?}");
    assert_eq!((herites[0].granted_by.as_str(), herites[0].mode.as_str()), ("user", "write"));
    assert!(herites[0].quote.contains("tu peux écrire dans durable") && herites[0].quote.contains(&format!("hérité de {cle} au fork")), "{:?}", herites[0]);
    assert!(host.call_sandbox_for(&n.id).roots.iter().all(|(r, _)| !r.ends_with("une-fois")), "the copy inherits a grant for one command");
    assert!(host.tache(&h.id, "archi-x").is_some(), "the original is no longer open");
    host.teammate_files(&h.id, "archi-x").unwrap().append("supervisor", "request", None, "Seulement pour l'original.").unwrap();
    assert!(!textes("archi-y").iter().any(|l| l.contains("Seulement")), "a line to the original reaches the copy");
    let a = host.agents();
    let copie = a["roots"][0]["tasks"].as_array().unwrap().iter().find(|t| t["name"] == "archi-y").expect("the copy in the tree").clone();
    assert_eq!(copie["forked_from"]["agent"], json!(cle), "{copie}");
}

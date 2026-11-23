//! **A path she wrote herself is read without a card, for ten minutes** (Lucie, 23 Nov 2026, ~21 h: « si moi je donne
//! un lien externe explicite, ça ne devrait pas le bloquer pour aller le lire ; un accès temporaire de 10 min à la
//! racine du fichier »). A path outside the task's roots, written in a human's request of the turn, is granted to the
//! task's agent for reading, until it expires; a path from anywhere else still goes to the card. A module of its own.

use super::mode_auto_v1::{ended, v1_with};
use crate::daemon_host::DaemonHost;
use serde_json::json;

fn ailleurs(d: &std::path::Path) -> std::path::PathBuf {
    let a = d.join("ailleurs");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::write(a.join("notes.txt"), "les notes d'ailleurs\n").unwrap();
    a.canonicalize().unwrap()
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_path_she_wrote_is_read_without_a_card() {
    let mut root = std::path::PathBuf::new();
    // The judge never says yes here: only her own words open the path.
    let v = v1_with(
        |d| {
            root = ailleurs(d);
            let fichier = root.join("notes.txt").display().to_string();
            (vec![format!("cat {fichier}")], format!("Lis {fichier} et dis-moi ce qu'il contient."))
        },
        &["__jamais__"],
    );
    v.wait("the task ends", &ended);
    let b = v.files.mailbox();
    let par: Vec<String> = v.host.root_grants(None).into_iter().filter(|g| g.root == root).map(|g| g.granted_by).collect();
    assert_eq!(par, ["mention"], "the path she wrote is granted by her mention: {b:#?}");
    assert!(!b.iter().any(|l| l["kind"] == "approval" || l["kind"] == "card"), "no card for a path she wrote: {b:#?}");
    assert!(v.seen.lock().unwrap().iter().any(|r| r.contains("les notes d'ailleurs")), "the file was read: {:?}", v.seen.lock().unwrap());
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn an_expired_grant_no_longer_covers_its_root() {
    let d = tempfile::tempdir().unwrap();
    let host = DaemonHost::new().with_racines_de_code(crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") });
    let root = ailleurs(d.path());
    let preuve = crate::daemon_host::root_grants::Proof { quote: "lis ce fichier".into(), at: Some(1), turn: None };
    let passe = crate::catalog::retard::now_ms() - 60_000;
    host.grant_root_until("agent:k", &root, preuve, "user", "read", Some(passe)).unwrap();
    assert!(host.root_grant_for("agent:k", &root.join("notes.txt")).is_none(), "a grant whose time is over covers nothing");
}

/// **Only her own words count**: a path from an agent's request, from a tool's result, or from a turn already ended is
/// not a mention; hers is, with its quotes and its final dot.
#[test]
fn only_a_humans_line_of_the_turn_mentions_a_path() {
    use crate::daemon_host::gate::cited_by_a_human;
    let refuse = std::path::Path::new("/srv/donnees/notes.txt");
    let humaine = |texte: &str| json!({ "kind": "request", "from": "lucie", "by": { "kind": "human" }, "text": texte });
    assert!(cited_by_a_human(&[humaine("Lis « /srv/donnees/notes.txt ».")], refuse).is_some(), "her path, quoted, with its dot");
    assert!(cited_by_a_human(&[humaine("Regarde dans /srv/donnees, stp")], refuse).is_some(), "the folder that holds it");
    let agent = json!({ "kind": "request", "from": "superviseur", "by": { "kind": "agent" }, "text": "Lis /srv/donnees/notes.txt" });
    assert!(cited_by_a_human(&[agent], refuse).is_none(), "an agent's path is not her mention");
    let outil = json!({ "kind": "report", "from": "teammate", "text": "/srv/donnees/notes.txt" });
    assert!(cited_by_a_human(&[humaine("Lis le rapport."), outil], refuse).is_none(), "a path from the agent's own lines is not hers");
    let fini = json!({ "kind": "ended", "from": "host", "text": "fait" });
    assert!(cited_by_a_human(&[humaine("Lis /srv/donnees/notes.txt"), fini, humaine("autre chose")], refuse).is_none(), "a turn already ended does not count");
    assert!(cited_by_a_human(&[humaine("Lis /srv")], refuse).is_none(), "a root too broad is no mention");
}

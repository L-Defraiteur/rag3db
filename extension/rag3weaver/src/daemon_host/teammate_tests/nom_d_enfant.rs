//! **A child of any Claude session can be opened** (recherche, 21 Nov 2026: a session named
//! « rag3weaver archi » could not open a sub-agent, its name is not a task name). A module of its own.

use super::*;
use super::super::taches::nom_d_enfant;

/// The child's name is a task name whatever the session's name: lowercase, accents dropped, the rest
/// a dash, within 64 characters; a name that already carries the session's slug is kept.
#[test]
fn a_childs_name_is_a_task_name_whatever_the_sessions_name() {
    assert_eq!(nom_d_enfant("archi", "fouille"), "archi-fouille");
    assert_eq!(nom_d_enfant("rag3weaver archi", "lot"), "rag3weaver-archi-lot");
    assert_eq!(nom_d_enfant("arbre principal", "lot"), "arbre-principal-lot");
    assert_eq!(nom_d_enfant("Mémoire", "B6"), "memoire-b6");
    assert_eq!(nom_d_enfant("coeur c++/hotfixs", "x"), "coeur-c-hotfixs-x");
    assert_eq!(nom_d_enfant("rag3weaver archi", "rag3weaver-archi-lot"), "rag3weaver-archi-lot", "already named");
    let long = nom_d_enfant(&"a".repeat(80), "lot");
    assert!(long.len() <= 64 && long.ends_with("-lot"), "within 64, the task's own name kept: {long}");
}

/// The host opens such a child, and its parent session keeps the exact name.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_session_with_spaces_opens_a_child() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("no model here".into()) });
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let enfant = nom_d_enfant("rag3weaver archi", "lot");
    host.ouvrir_une_tache(&h.id, &enfant).expect("the child opens");
    host.donner_les_vues(&h.id, &enfant, "rag3weaver archi", vec!["backend".into()], None).unwrap();
    let t = host.tache(&h.id, &enfant).unwrap();
    assert_eq!(t.parent_session.as_deref(), Some("rag3weaver archi"), "the exact name is kept");
}

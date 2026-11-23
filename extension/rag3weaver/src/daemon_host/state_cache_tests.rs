//! **The host's state does not reread every index at each read** (a measure on 23 November 2026: `/ide/etat` took 6.5
//! to 7 s with 84 backends, each reading its index; the IDE paid ~14 s at each arrival on a chat). The index part of a
//! backend's state is kept until an event says it changed: an event relayed for that backend, or its opening or
//! closing.
use super::*;
use crate::daemon_host::atelier_fichiers::tests::{atelier, attendre_l_index, petit_depot};
use std::sync::atomic::Ordering;

#[test]
fn a_second_read_of_the_state_rereads_no_index_until_an_event_says_it_changed() {
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let depot = petit_depot(d.path());
    a.appeler("attach_worktree", serde_json::json!({ "path": depot })).unwrap();
    attendre_l_index(&a, &depot);
    let etat = a.host.state();
    let manifeste = etat["backends"]
        .as_array()
        .and_then(|b| b.iter().find(|b| b["manifest"].as_str().is_some_and(|m| m.contains("code-rag3db-essai")) && b["tache"].is_null()))
        .and_then(|b| b["manifest"].as_str())
        .expect("the repository is in the state")
        .to_string();
    let avant = a.host.lectures_d_index.load(Ordering::SeqCst);
    let _ = a.host.state();
    assert_eq!(a.host.lectures_d_index.load(Ordering::SeqCst), avant, "the second read rereads no index");
    // An event relayed for the repository: its index part is read again, and its alone.
    a.host.ide_flux().publier("files", &manifeste, serde_json::json!({ "kind": "FileChanged", "path": "src/lib.rs" }));
    let _ = a.host.state();
    assert_eq!(a.host.lectures_d_index.load(Ordering::SeqCst), avant + 1, "the repository's index is read again, the others not");
}

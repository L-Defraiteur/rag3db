//! **The host's state does not reread every index at each read** (a measure on 23 November 2026: `/ide/etat` took 6.5
//! to 7 s with 84 backends, each reading its index; the IDE paid ~14 s at each arrival on a chat). The index part of a
//! backend's state is kept until an event says it changed: an event relayed for that backend, or its opening or
//! closing.
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
    // An event relayed for the repository: its index part is read again, and its alone — even while another read of the
    // state runs beside (the host publishes its state in the background at an index's end: a full lib pass on 23 Nov.
    // counted 5 reads for 4, the two reads both missing the cache before either kept it).
    a.host.ide_flux().publier("files", &manifeste, serde_json::json!({ "kind": "FileChanged", "path": "src/lib.rs" }));
    let par_manifeste = || a.host.lectures_par_manifeste.lock().unwrap().clone();
    let avant = par_manifeste();
    std::thread::scope(|s| {
        s.spawn(|| a.host.state());
        let _ = a.host.state();
    });
    let apres = par_manifeste();
    let relus: Vec<&String> = apres.iter().filter(|(m, n)| avant.get(*m) != Some(*n)).map(|(m, _)| m).collect();
    assert_eq!(relus, vec![&manifeste], "the repository's index is read again (once, or once by each read that missed it), the others not");
}

/// **Only a settled index part is kept** (a navigator witness on 23 November 2026: the bar never said « rag indexé »
/// after an index ended — the state read between the end's event and the base saying « ready » was kept, and no event
/// came after to forget it). An index part still in progress is read again at each read; a settled one is kept.
#[test]
fn only_a_settled_index_part_is_kept() {
    let d = tempfile::tempdir().unwrap();
    let a = atelier(d.path());
    let depot = petit_depot(d.path());
    a.appeler("attach_worktree", serde_json::json!({ "path": depot })).unwrap();
    attendre_l_index(&a, &depot);
    let _ = a.host.state();
    // In progress: read again at each read.
    *a.host.index_force.lock().unwrap() = Some(serde_json::json!({ "File": { "text": "indexing", "text_percent": 40, "vectors": "never", "relations": "never" } }));
    a.host.etats_d_index.lock().unwrap().clear();
    let avant = a.host.lectures_d_index.load(Ordering::SeqCst);
    let _ = a.host.state();
    let _ = a.host.state();
    let lues = a.host.lectures_d_index.load(Ordering::SeqCst) - avant;
    let ouverts = a.host.state()["backends"].as_array().map_or(0, |b| b.iter().filter(|b| b["open"] == true).count()) as u64;
    assert!(lues >= 2 * ouverts, "an index part in progress is read at each read: {lues} reads for {ouverts} open backends, twice");
    // Settled: kept.
    *a.host.index_force.lock().unwrap() = Some(serde_json::json!({ "File": { "text": "ready", "text_percent": null, "vectors": "ready", "relations": "never" } }));
    a.host.etats_d_index.lock().unwrap().clear();
    let _ = a.host.state();
    let avant = a.host.lectures_d_index.load(Ordering::SeqCst);
    let _ = a.host.state();
    assert_eq!(a.host.lectures_d_index.load(Ordering::SeqCst), avant, "a settled index part is kept");
}

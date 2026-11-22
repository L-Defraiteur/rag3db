//! **A base analysed under another version repasses on open** (ticket
//! `docs/codeparsers/tickets/2026-11-22-une-montee-des-regles-n-est-vue-qu-a-un-index-entier.md`, c7's choice).
//!
//! Each `File` row carries the version of the analysis it was made with (`analyse`, `code::version_de_l_analyse`:
//! the codeparsers commit, `REGLES_DE_L_ANALYSE`, the code's template). Only a full sync (`index`) compared it: on
//! 22 November, the host put in place under rules 16 reopened Lucie's repository, its watch took up the 6 files changed
//! on disk, and nothing was analysed again. A backend opened by the host whose files carry another version starts its
//! full pass by itself, in the background.

use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// A small repository indexed; its `File` rows set to an analysis of before; closed, reopened by the host: without any
/// explicit `index`, every row carries the current analysis again.
#[test]
fn a_base_analysed_under_another_version_repasses_on_open() {
    let d = tempfile::tempdir().unwrap();
    let (manifeste, _arbre) = crate::daemon_host::symboles_cites::tests::petit_depot(d.path());
    let manifeste = PathBuf::from(manifeste);
    let adresse = super::tests::service_granite();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["embeddings"]["catch_up_on_open"] = json!(false);
    m["embeddings"]["address"] = json!(adresse);
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let host = Arc::new(crate::daemon_host::tests::hote_isole());
    let h = host.hosted(&manifeste).unwrap();
    let files_of_another_analysis = || -> i64 {
        host.with_backend(&h, |b| {
            let cat = b.catalog.lock().unwrap();
            let courante = cat.version_de_l_analyse.clone();
            let r = cat.execute_raw(&format!("MATCH (f:File) WHERE f.analyse <> '{courante}' RETURN count(f)")).unwrap();
            r.rows[0][0].as_i64().unwrap()
        })
        .unwrap()
    };
    let wait_for_no_indexing = |what: &str| {
        let debut = std::time::Instant::now();
        while host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap() > 0 {
            assert!(debut.elapsed() < std::time::Duration::from_secs(120), "{what}: the index did not finish in 120 s");
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    };
    host.with_backend(&h, |b| b.call("index", json!({ "confirm": true }))).unwrap().unwrap();
    wait_for_no_indexing("the first index");
    assert_eq!(files_of_another_analysis(), 0, "the first index writes the current analysis");

    // The rows of an analysis of before, as a base indexed under older rules.
    host.with_backend(&h, |b| b.catalog.lock().unwrap().execute_raw("MATCH (f:File) SET f.analyse = 'an-analysis-of-before'").map(|_| ()))
        .unwrap()
        .unwrap();
    let avant = files_of_another_analysis();
    assert!(avant > 0, "the decor has rows of another analysis");
    let s = host.open_session("the witness", None, Vec::new(), None);
    assert_eq!(host.close(Some(&s), &manifeste)["ok"], true);

    // Reopened by the host: its files repass by themselves.
    host.with_backend(&h, |b| b.index_states()).unwrap().expect("the state");
    let debut = std::time::Instant::now();
    loop {
        let restes = files_of_another_analysis();
        let en_cours = host.with_backend(&h, |b| crate::dataflow::index_nodes::indexings_running(&b.catalog)).unwrap();
        if restes == 0 && en_cours == 0 {
            break;
        }
        assert!(
            debut.elapsed() < std::time::Duration::from_secs(120),
            "120 s after the reopening, {restes} of {avant} files still carry the analysis of before (indexings running: {en_cours})"
        );
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    let s = host.open_session("the witness", None, Vec::new(), None);
    assert_eq!(host.close(Some(&s), &manifeste)["ok"], true);
}

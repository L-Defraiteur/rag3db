//! **A turn that brings hundreds of files is said, not landed in silence** (c7, the terminal's night, 23 Nov 2026: a
//! `git add -A` after an install takes `node_modules` with it). Past a few hundred files, the host does not
//! fast-forward the user's folder: a `merge_held` line says how many, and the command that takes the branch anyway.
//! A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::{ended, teammate};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_turn_of_hundreds_of_files_is_held_and_said() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    crate::daemon_host::chat_qui_code::en_copie(&manifeste);
    let commandes = vec![
        "sh -c 'mkdir -p deps && for i in $(seq 1 400); do echo $i > deps/f$i.txt; done'".to_string(),
        "git add deps".to_string(),
        "git commit -q -m deps".to_string(),
    ];
    let gate = d.path().join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d.path(), teammate(commandes, Arc::new(Mutex::new(Vec::new())))).with_runs_root(d.path().join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Installe les dépendances et commite.", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    assert!(!depot.join("deps").exists(), "400 files are not landed in silence: {b:#?}");
    let dit = b.iter().find(|l| l["from"] == "host" && l["kind"] == "merge_held").unwrap_or_else(|| panic!("said: {b:#?}"));
    assert!(dit["why"].as_str().unwrap_or_default().contains("400"), "{dit}");
    assert!(dit["command"].as_str().unwrap_or_default().contains("merge --ff-only"), "{dit}");
}

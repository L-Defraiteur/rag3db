//! **A new project works in the auto mode from its first request** (Lucie, 22 Nov 2026: « c'est pas auto si tous les
//! git status me demandent »; c7: a new project's posture is « do, and ask for what breaks or leaves »). A repository
//! added from the shipped code template, without any setting touched: the teammate writes a file, runs it, commits it,
//! and no card opens. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::{ended, teammate};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn write_run_commit_ends_with_a_commit_and_no_card() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let commandes = vec!["sh -c 'echo fizz > fizz.txt'".to_string(), "cat fizz.txt".to_string(), "git status --short".to_string(), "git add fizz.txt".to_string(), "git commit -q -m fizz".to_string()];
    let gate = d.path().join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d.path(), teammate(commandes, seen.clone())).with_runs_root(d.path().join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Écris fizz.txt, lis-le, puis commite-le.", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    let boite = loop {
        let b = files.mailbox();
        if ended(&b) || b.iter().any(|l| l["kind"] == "approval") {
            break b;
        }
        assert!(Instant::now() < fin, "in 120 s: {b:#?}");
        std::thread::sleep(Duration::from_millis(100));
    };
    let cartes: Vec<&Value> = boite.iter().filter(|l| l["kind"] == "approval").collect();
    assert!(cartes.is_empty(), "no card for writing, running and committing in its own tree: {cartes:#?}; results {:?}", seen.lock().unwrap());
    let t = host.tache(&h.id, "doc").expect("the task");
    let arbre = t.workspace.as_ref().map(|w| w.racine.clone()).unwrap_or(t.arbre);
    let log = std::process::Command::new("git").arg("-C").arg(&arbre).args(["log", "--format=%s", "-1"]).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&log.stdout).trim(), "fizz", "the commit is on the task's branch");
}

//! **The work lands in the user's folder at each turn that commits** (c7, 22 Nov 2026, the terminal's night: Lucie
//! wants her files in her folder). In the auto mode, when a step ends with new commits on the task's branch, the host
//! fast-forwards the user's current branch to it, if her tree is clean and her branch has not moved: the files arrive,
//! and a host line `merged` says how many. Otherwise nothing is touched, and a line `merge_held` says why and the
//! command that takes the branch. Never a non-fast-forward merge, never a forced checkout. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use crate::daemon_host::teammate::TeammateFiles;
use crate::daemon_host::DaemonHost;
use super::mode_auto_v1::{ended, teammate};
use super::*;
use std::time::{Duration, Instant};

fn lancer(d: &std::path::Path, salir: bool) -> (Arc<DaemonHost>, std::path::PathBuf, TeammateFiles) {
    let (depot, manifeste) = depot_inscrit(d, true);
    crate::daemon_host::chat_qui_code::en_copie(&manifeste);
    if salir {
        std::fs::write(depot.join("README.md"), "# Petit dépôt, modifié par la personne\n").unwrap();
    }
    let commandes = vec!["sh -c 'echo fizz > fizz.txt'".to_string(), "git add fizz.txt".to_string(), "git commit -q -m fizz".to_string()];
    let gate = d.join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d, teammate(commandes, Arc::new(Mutex::new(Vec::new())))).with_runs_root(d.join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Écris fizz.txt et commite-le.", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    (host, depot, files)
}

fn head(dir: &std::path::Path) -> String {
    String::from_utf8_lossy(&std::process::Command::new("git").arg("-C").arg(dir).args(["log", "--format=%s", "-1"]).output().unwrap().stdout).trim().to_string()
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_committed_turn_lands_in_the_users_folder() {
    let d = tempfile::tempdir().unwrap();
    let (_host, depot, files) = lancer(d.path(), false);
    assert!(depot.join("fizz.txt").is_file(), "the file is in her folder: {:#?}", files.mailbox());
    assert_eq!(head(&depot), "fizz", "her branch has the commit");
    let b = files.mailbox();
    let dit = b.iter().find(|l| l["from"] == "host" && l["kind"] == "merged").unwrap_or_else(|| panic!("said: {b:#?}"));
    assert_eq!(dit["into"], json!(depot.display().to_string()), "{dit}");
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_dirty_folder_is_not_touched_and_the_line_says_why() {
    let d = tempfile::tempdir().unwrap();
    let (_host, depot, files) = lancer(d.path(), true);
    assert!(!depot.join("fizz.txt").exists(), "nothing landed in a dirty folder");
    assert_ne!(head(&depot), "fizz");
    let b = files.mailbox();
    let dit = b.iter().find(|l| l["from"] == "host" && l["kind"] == "merge_held").unwrap_or_else(|| panic!("said: {b:#?}"));
    assert!(dit["command"].as_str().unwrap_or_default().contains("merge --ff-only"), "{dit}");
}

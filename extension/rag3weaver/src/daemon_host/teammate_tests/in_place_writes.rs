//! **An agent in the user's folder writes there** (the replay of 23 Nov 2026, 18 h 26: in place, `write` and
//! `replace_text` were refused as « the tree the user holds », and the refusal sent the agent to open a branch of its
//! own in her repository). A task in place is granted its own folder. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_task_in_place_writes_a_tracked_file_of_its_folder() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let seen = seen.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let seen = seen.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                seen.lock().unwrap().extend(results.iter().cloned());
                match results.len() {
                    0 => answer(turns, opts, sink, "Je complète le README.", calls(&[("replace_text", json!({ "path": "README.md", "old": "# Petit dépôt", "new": "# Petit dépôt, écrit en place" }))])),
                    _ => answer(turns, opts, sink, "C'est fait : le titre du README dit qu'il est écrit en place.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Complète le README.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(60);
    while !ended(&files.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let seen = seen.lock().unwrap().join("\n");
    assert!(!seen.contains("n'a pas ouvert lui-même"), "the task in place is not refused its own folder: {seen}");
    assert!(std::fs::read_to_string(depot.join("README.md")).unwrap_or_default().contains("écrit en place"), "the file is written in her folder: {seen}");
}

/// **The user's branch is not changed behind her back** (the replay of 23 Nov 2026, 18 h 40: the agent ran
/// `git checkout -b particles` in her folder, and the turn's commit went there): a turn that ends on another branch
/// than it started on says so in one line, naming both.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_turn_that_changed_the_users_branch_says_so() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let avant = std::process::Command::new("git").arg("-C").arg(&depot).args(["rev-parse", "--abbrev-ref", "HEAD"]).output().unwrap();
    let avant = String::from_utf8_lossy(&avant.stdout).trim().to_string();
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, |turns, opts, sink| match tool_results(turns).len() {
            0 => answer(turns, opts, sink, "Je prépare une branche.", calls(&[("run_command", json!({ "command": "git checkout -b autre" }))])),
            1 => answer(turns, opts, sink, "", calls(&[("edit_file", json!({ "path": "b.txt", "content": "b\n" }))])),
            _ => answer(turns, opts, sink, "C'est fait.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Écris b.txt.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(60);
    while !ended(&files.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    assert!(
        b.iter().any(|l| l["from"] == "host" && l["branch_changed"]["from"] == avant.as_str() && l["branch_changed"]["to"] == "autre"),
        "the turn says it changed her branch, naming both: {b:#?}"
    );
}

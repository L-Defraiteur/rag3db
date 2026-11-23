//! **The facts of a turn, by the host** (c7, 23 Nov 2026, 01:50: a report said « ajout d'un test dédié » after seven
//! commands and no write; the test did not exist). Before the end of a turn, the host says what git sees changed in
//! the task's tree since the turn's start, and says it too when nothing changed: the claim stands next to the fact.
//! A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

fn scripted(steps: Vec<(&'static str, Value)>, seen: Arc<Mutex<Vec<String>>>) -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let steps = steps.clone();
        let seen = seen.clone();
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            let results = tool_results(turns);
            if let Some(r) = results.last() {
                seen.lock().unwrap().push(r.clone());
            }
            match steps.get(results.len()) {
                Some((tool, args)) => answer(turns, opts, sink, "", calls(&[(tool, args.clone())])),
                None => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
            }
        })))
    })
}

fn tour(steps: Vec<(&'static str, Value)>) -> Vec<Value> {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let gate = d.path().join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d.path(), scripted(steps, Arc::new(Mutex::new(Vec::new())))).with_runs_root(d.path().join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Ajoute un test.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    files.mailbox()
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_end_of_a_turn_says_the_files_it_changed() {
    let b = tour(vec![("edit_file", json!({ "path": "tests/test_notes.py", "content": "def test_x():\n    assert True\n" }))]);
    let dit = b.iter().find(|l| l["from"] == "host" && l["kind"] == "turn_changes").unwrap_or_else(|| panic!("the turn's facts: {b:#?}"));
    assert!(dit["text"].as_str().unwrap_or_default().contains("tests/test_notes.py"), "{dit}");
    assert_eq!(dit["files"], json!(["tests/test_notes.py"]), "{dit}");
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_turn_that_changed_nothing_says_so() {
    let b = tour(vec![("run_command", json!({ "command": "git status --short" }))]);
    let dit = b.iter().find(|l| l["from"] == "host" && l["kind"] == "turn_changes").unwrap_or_else(|| panic!("the turn's facts: {b:#?}"));
    assert!(dit["text"].as_str().unwrap_or_default().contains("aucun fichier modifié"), "{dit}");
}

//! **The work of a turn reaches the user's folder even if the agent did not commit** (c7, 23 Nov 2026, 01:27: the real
//! three.js request ended with every file in the task's tree, untracked, and an empty folder for the user). At the end
//! of a step with a gate level, the host commits what is left in the task's tree (its `.gitignore` kept, the message
//! taken from the step done or the request), then the turn lands. A module of its own.

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

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_turn_that_writes_without_committing_lands_in_the_users_folder() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    crate::daemon_host::chat_qui_code::en_copie(&manifeste);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let steps = vec![
        ("edit_file", json!({ "path": "particules.ts", "content": "export const n = 1000;\n" })),
        ("edit_file", json!({ "path": ".gitignore", "content": "dist/\n" })),
        ("edit_file", json!({ "path": "dist/bundle.js", "content": "bâti\n" })),
    ];
    let gate = d.path().join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d.path(), scripted(steps, seen.clone())).with_runs_root(d.path().join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Crée les particules.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    assert!(depot.join("particules.ts").is_file(), "the turn's files are in her folder though the agent did not commit: {b:#?}; {:?}", seen.lock().unwrap());
    assert!(depot.join(".gitignore").is_file());
    assert!(!depot.join("dist/bundle.js").exists(), "what the .gitignore leaves out stays out");
    assert!(b.iter().any(|l| l["from"] == "host" && l["kind"] == "merged"), "the line says where the work is: {b:#?}");
    // The host's commit is signed by rag3weaver, never by the machine's git config (c7, 23 Nov 2026).
    let auteur = std::process::Command::new("git").arg("-C").arg(&depot).args(["log", "-1", "--format=%an <%ae>"]).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&auteur.stdout).trim(), "rag3weaver <rag3weaver@localhost>", "the host's commit's author");
}

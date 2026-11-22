//! **A command in the background does not hold the others** (archi, the terminal's night, 23 Nov 2026): a dev server
//! started in the background, then `ls`, returns at once. Every command of the sandbox took the lock of the shared
//! cargo target, and a background command kept it its whole life: the next command waited until its delay, and the
//! delay's kill took the server with it. Only cargo, whose target it is, takes the lock. A module of its own.

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
fn a_command_after_a_background_one_returns_at_once() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let steps = vec![
        ("run_command", json!({ "command": "sleep 120", "background": true })),
        ("run_command", json!({ "command": "ls", "timeout_s": 10 })),
    ];
    let gate = d.path().join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d.path(), scripted(steps, seen.clone())).with_runs_root(d.path().join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Lance le serveur, puis liste.", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let vus = seen.lock().unwrap().clone();
    let ls = vus.get(1).cloned().unwrap_or_default();
    assert!(ls.contains("code 0") && !ls.contains("délai"), "ls returns at once while a command runs in the background: {vus:#?}");
}

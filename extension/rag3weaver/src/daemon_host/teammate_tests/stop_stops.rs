//! **A stop stops** (the terminal's trial of 23 Nov 2026, 11 h 38: after a stop the agent still called a tool, a
//! `wait_output` on a `sleep 300` held 300 s, the file written stayed uncommitted, and the next message was eaten). A
//! stop cuts the tool call in flight, calls the model no more, lands what the turn wrote (a commit said interrupted),
//! marks the turn's end (`ended {stopped, by, landed}`), and is worth for the turn in progress only: the next message
//! goes through. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_stop_cuts_the_call_in_flight_lands_the_work_and_lets_the_next_message_through() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let calls = Arc::new(Mutex::new(0usize));
    let model: ChatModel = {
        let calls = calls.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let calls = calls.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let n = { let mut c = calls.lock().unwrap(); *c += 1; *c };
                let got = tool_results(turns).len();
                // First turn: write a file, then a long command; the second turn ends at once.
                match (n, got) {
                    (_, 0) if n <= 2 => answer(turns, opts, sink, "", calls_of(&[("edit_file", json!({ "path": "a.txt", "content": "a\n" }))])),
                    (_, 1) if n <= 3 => answer(turns, opts, sink, "", calls_of(&[("run_command", json!({ "command": "sleep 300", "timeout_s": 600 }))])),
                    _ => answer(turns, opts, sink, "", calls_of(&[("report", json!({ "text": "noté" })), ("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let by = json!({ "by": { "kind": "human", "client": "tty" } });
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Écris a.txt puis attends.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    // The long command runs.
    let end = Instant::now() + Duration::from_secs(60);
    while !files.mailbox().iter().any(|l| l["kind"] == "ended") && *calls.lock().unwrap() < 2 {
        assert!(Instant::now() < end, "the first calls: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(1500));
    let t0 = Instant::now();
    host.post_to_teammate(&h.id, "doc", "lucie", "stop", None, "", &by).unwrap();
    let end = Instant::now() + Duration::from_secs(20);
    while !files.mailbox().iter().any(|l| l["kind"] == "ended") {
        assert!(Instant::now() < end, "the stop is honoured: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(t0.elapsed() < Duration::from_secs(5), "the stop cuts the call in flight: {:?}", t0.elapsed());
    let b = files.mailbox();
    let fin = b.iter().find(|l| l["kind"] == "ended").unwrap().clone();
    assert_eq!(fin["stopped"], true, "{fin}");
    assert_eq!(fin["by"], "human", "{fin}");
    assert!(fin["landed"].as_array().is_some_and(|a| a.iter().any(|p| p == "a.txt")), "{fin}");
    assert!(depot.join("a.txt").is_file(), "what the turn wrote is in her folder");
    // The next message goes through.
    let avant = *calls.lock().unwrap();
    host.post_to_teammate(&h.id, "doc", "lucie", "instruction", None, "Et maintenant ?", &by).unwrap();
    let end = Instant::now() + Duration::from_secs(60);
    while files.mailbox().iter().filter(|l| l["kind"] == "ended").count() < 2 {
        assert!(Instant::now() < end, "the next message: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(*calls.lock().unwrap() > avant, "the next message reaches the model, it is not eaten by the stop");
}

fn calls_of(v: &[(&str, Value)]) -> Vec<(String, String)> {
    calls(v)
}

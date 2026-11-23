//! **Each model call's record says its duration** (banc's protocol for the conversation's policies, 23 Nov 2026: the
//! tokens read in cache and out of it, the output, and how long each call took). The run's journal keeps a `Consumed`
//! event per model call; it carries `ms`. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn each_model_call_of_a_turn_says_its_duration_in_the_run() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, |turns, opts, sink| {
            answer(turns, opts, sink, "", calls(&[("report", json!({ "text": "noté" })), ("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Dis bonjour.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let end = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < end, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    // The host's journal of runs, for the repository and for its task: their `Consumed` events.
    let tache = host.tache(&h.id, "doc").expect("the task").id;
    let mut consumed = Vec::new();
    for backend in [h.id.as_str(), tache.as_str()] {
        for run in host.journal.runs_de(backend) {
            if let Some(v) = host.journal.lire(backend, &run) {
                consumed.extend(v["evenements"].as_array().into_iter().flatten().filter(|e| e["kind"] == "Consumed").cloned());
            }
        }
    }
    assert!(!consumed.is_empty(), "the run's journal keeps its model calls");
    assert!(consumed.iter().all(|c| c["ms"].is_u64()), "each model call says its duration: {consumed:#?}");
}

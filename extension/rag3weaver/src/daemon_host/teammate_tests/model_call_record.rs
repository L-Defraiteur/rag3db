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
            // A provider's usage: a call with nothing to count is not recorded.
            let (fin, mut usage) = answer(turns, opts, sink, "", calls(&[("report", json!({ "text": "noté" })), ("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))?;
            usage.prompt_tokens = 100;
            usage.cached_prompt_tokens = 40;
            usage.completion_tokens = 10;
            usage.ms = 7;
            Ok((fin, usage))
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
    // The host's journal of runs, for the repository and for its task: their `Consumed` events. The journal writes a
    // run when its root run ends, by a thread that follows the bus: waited for, 15 s at most.
    let tache = host.tache(&h.id, "doc").expect("the task").id;
    let lire = || {
        let mut consumed = Vec::new();
        for backend in [h.id.as_str(), tache.as_str()] {
            for run in host.journal.runs_de(backend) {
                if let Some(v) = host.journal.lire(backend, &run) {
                    consumed.extend(v["evenements"].as_array().into_iter().flatten().filter(|e| e["kind"] == "Consumed").cloned());
                }
            }
        }
        consumed
    };
    let end = Instant::now() + Duration::from_secs(15);
    let mut consumed = lire();
    while consumed.is_empty() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(200));
        consumed = lire();
    }
    assert!(!consumed.is_empty(), "the run's journal keeps its model calls");
    assert!(consumed.iter().all(|c| c["ms"] == 7), "each model call says its duration: {consumed:#?}");
}

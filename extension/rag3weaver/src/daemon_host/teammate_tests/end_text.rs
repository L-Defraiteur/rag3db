//! **A step that used tools ends with a word to the user** (the replay of 23 Nov 2026, 18 h 26: a turn of 5 minutes
//! and 15 files whose only text was its first sentence; the end-of-turn explanation never came, the refusal of a
//! silent step_done not playing since a text existed). In a turn a human opened, a step_done is refused while no text
//! followed the step's last tool call. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_step_done_right_after_a_tool_is_refused_until_the_agent_explains() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, |turns, opts, sink| {
            let results = tool_results(turns);
            let refused = results.iter().any(|r| r.contains("\"ok\":false"));
            match (results.len(), refused) {
                (0, _) => answer(turns, opts, sink, "Je regarde le projet.", calls(&[("list_files", json!({}))])),
                (_, false) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
                (_, true) => answer(turns, opts, sink, "J'ai parcouru le projet : il tient en une bibliothèque et son README.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
            }
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Regarde le projet.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(60);
    while !ended(&files.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    assert!(
        b.iter().any(|l| l["from"] == "teammate" && l["text"].as_str().is_some_and(|t| t.contains("J'ai parcouru le projet"))),
        "the explanation after the last tool reaches her: {b:#?}"
    );
}

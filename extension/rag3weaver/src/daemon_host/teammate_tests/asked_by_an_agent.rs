//! **A question an agent asked gets its answer too** (the replay of 23 Nov 2026, 20 h 30: a supervisor agent asked
//! « comment je lance la page ? »; the teammate wrote one sentence, read a file, and closed the turn with a silent
//! step_done: no answer). The closing step_done of a turn opened by a request, an agent's as a human's, is refused
//! while no text followed its last tool call. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_question_an_agent_asked_is_answered_after_the_last_tool() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, |turns, opts, sink| {
            let results = tool_results(turns);
            let refused = results.iter().any(|r| r.contains("\"ok\":false"));
            match (results.len(), refused) {
                (0, _) => answer(turns, opts, sink, "Je vérifie les commandes du projet.", calls(&[("list_files", json!({}))])),
                (_, false) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "expliqué", "remaining": "" }))])),
                (_, true) => answer(turns, opts, sink, "Lance npm install puis npm run dev, et ouvre l'adresse que Vite affiche.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "expliqué", "remaining": "" }))])),
            }
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "supervisor", "request", None, "comment je lance la page ?", &json!({ "by": { "kind": "agent" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(60);
    while !ended(&files.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    assert!(
        b.iter().any(|l| l["from"] == "teammate" && l["text"].as_str().is_some_and(|t| t.contains("npm run dev"))),
        "the agent that asked gets the answer after the last tool: {b:#?}"
    );
}

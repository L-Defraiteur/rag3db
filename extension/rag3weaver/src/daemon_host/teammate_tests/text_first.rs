//! **step_done waits for a word to the user** (Lucie's trial of 23 Nov 2026, `~/essai-3`: « bonjour! », two calls
//! that closed by step_done without a word, a relaunch, a content judge of 42 s, and her screen showed a description).
//! In a turn opened by a human, step_done is refused while no text was written; the model reads the refusal in the
//! same call and writes. Twice refused, the host closes the step with a human line, never the `done`. A module of its
//! own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

fn run(model: ChatModel) -> (Vec<Value>, tempfile::TempDir) {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "bonjour!", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(60);
    while !files.mailbox().iter().any(|l| l["kind"] == "ended") && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    (files.mailbox(), d)
}

fn done() -> Value {
    json!({ "item": "", "succeeded": true, "done": "Salutations adressées à l'utilisateur.", "remaining": "" })
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_refused_step_done_makes_the_model_write_its_answer() {
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, |turns, opts, sink| {
            // It closes without a word; once refused, it writes.
            let refused = tool_results(turns).iter().any(|r| r.contains("\"ok\":false"));
            if refused {
                answer(turns, opts, sink, "Bonjour ! Que veux-tu faire aujourd'hui ?", calls(&[("step_done", done())]))
            } else {
                answer(turns, opts, sink, "", calls(&[("step_done", done())]))
            }
        })))
    });
    let (b, _d) = run(model);
    assert!(b.iter().any(|l| l["from"] == "teammate" && l["text"].as_str().is_some_and(|t| t.contains("Bonjour !"))), "the answer reaches her: {b:#?}");
    assert!(!b.iter().any(|l| l["text"].as_str().is_some_and(|t| t.contains("relance"))), "no relaunch of a whole step: {b:#?}");
    assert!(b.iter().any(|l| l["kind"] == "ended"), "the turn ends: {b:#?}");
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_model_that_never_writes_ends_on_a_human_line_never_its_done() {
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, |turns, opts, sink| answer(turns, opts, sink, "", calls(&[("step_done", done())])))))
    });
    let (b, _d) = run(model);
    assert!(b.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("l'agent n'a pas répondu"))), "a human line says it: {b:#?}");
    assert!(!b.iter().any(|l| l["kind"] == "report" && l["text"].as_str().is_some_and(|t| t.contains("Salutations"))), "never the done shown as an answer: {b:#?}");
    assert!(b.iter().any(|l| l["kind"] == "ended"), "the turn ends: {b:#?}");
}

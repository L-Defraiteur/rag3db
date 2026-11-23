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

/// **Only the step_done that closes the turn asks for the explanation** (Lucie, 23 Nov 2026: an explanation at the
/// end of the turn, not a comment after each step): a plan of three items, no text asked at the first two step_dones,
/// asked at the last.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn only_the_step_done_that_closes_the_plan_asks_for_the_explanation() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let refused = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let refused = refused.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let refused = refused.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
                let step = step_of(turns);
                let results = tool_results(turns);
                let item = ["a", "b", "c"][(step.max(1) - 1).min(2)];
                let done = json!({ "item": item, "succeeded": true, "done": format!("{item} fait"), "remaining": "" });
                if results.iter().any(|r| r.contains("\"ok\":false")) {
                    refused.lock().unwrap().push(item.to_string());
                    return answer(turns, opts, sink, "Tout est fait : les trois éléments du plan sont posés.", calls(&[("step_done", done)]));
                }
                match (step, results.len()) {
                    (1, 0) => answer(turns, opts, sink, "Je pose un plan en trois éléments.", calls(&[("edit_plan", json!({ "plan": {
                        "request": "Trois petites choses", "proposal": "Une par pas",
                        "items": [
                            { "id": "a", "text": "a", "status": "todo", "alone": true },
                            { "id": "b", "text": "b", "status": "todo", "alone": true },
                            { "id": "c", "text": "c", "status": "todo", "alone": true }
                        ] } }))])),
                    (1, 1) | (_, 0) => answer(turns, opts, sink, "", calls(&[("list_files", json!({}))])),
                    _ => answer(turns, opts, sink, "", calls(&[("step_done", done)])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Fais les trois choses.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(90);
    while !ended(&files.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let r = refused.lock().unwrap().clone();
    assert_eq!(r, vec!["c".to_string()], "only the last step_done asks for the explanation: {:#?}", files.mailbox());
}

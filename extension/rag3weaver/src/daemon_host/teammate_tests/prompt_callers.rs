//! **The agent is told when to look one hop away** (the bench's sixth trial, 23 Nov 2026: the agent never called
//! `usages` nor `impact`, even when the cause was a caller of the function it had found). The code agent's prompt
//! and the descriptions of `usages` and `impact` say when: once the function that carries the symptom is found, its
//! callers and what it touches are often the cause. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_prompt_and_the_tools_say_when_to_look_at_callers() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let first = Arc::new(Mutex::new(None::<(String, Vec<(String, String)>)>));
    let model: ChatModel = {
        let first = first.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let first = first.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let mut f = first.lock().unwrap();
                if f.is_none() {
                    let system = turns.iter().map(|t| t.content.clone()).collect::<Vec<_>>().join("\n");
                    *f = Some((system, opts.tools.iter().map(|t| (t.name.clone(), t.description.clone())).collect()));
                }
                answer(turns, opts, sink, "", calls(&[("report", json!({ "text": "noté" })), ("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Trouve pourquoi la sauvegarde perd des données.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let end = Instant::now() + Duration::from_secs(120);
    while first.lock().unwrap().is_none() {
        assert!(Instant::now() < end, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = ended;
    let (system, tools) = first.lock().unwrap().clone().unwrap();
    assert!(system.contains("usages") && system.contains("impact") && system.contains("symptôme"), "the prompt says when to look at callers: {}", &system[..system.len().min(600)]);
    let description = |n: &str| tools.iter().find(|(t, _)| t == n).map(|(_, d)| d.clone()).unwrap_or_default();
    assert!(description("usages").contains("symptôme"), "usages says when: {}", description("usages"));
    assert!(description("impact").contains("symptôme"), "impact says when: {}", description("impact"));
}

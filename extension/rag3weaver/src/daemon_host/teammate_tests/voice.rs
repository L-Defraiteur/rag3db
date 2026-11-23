//! **The agent speaks to the user in the text of its reply, and signs nothing** (Lucie's trial of 23 Nov 2026: all 24
//! calls had an empty text, the only words she read came through the `report` tool, whole, at the end; and its pages
//! carried its provider's marks). The prompt names the text of the reply as the channel to the user, never `report`;
//! and it says, with the session's own model and provider filled in by the host, that what the agent produces bears no
//! mark of itself, its model, its provider or another AI. No provider's name is written in the prompt itself. A module
//! of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

#[test]
fn the_prompt_names_the_reply_as_the_channel_and_writes_no_provider() {
    let p = Step::prompt_for_witnesses();
    assert!(p.contains("the text of your reply"), "the channel to the user is the reply's text: {p}");
    assert!(!p.contains("in report") && !p.contains("(report)"), "report is not the channel: {p}");
    for name in ["Gemini", "Google", "Anthropic", "Claude", "OpenAI", "GPT", "Mistral", "Llama"] {
        assert!(!p.contains(name), "no provider written in the prompt ({name}): {p}");
    }
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_step_says_to_sign_nothing_with_the_session_model_and_provider() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let seen = seen.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let seen = seen.clone();
            Ok(Arc::new(CallbackLlm::new("acme/wonder-1", 16384, move |turns, opts, sink| {
                seen.lock().unwrap().extend(turns.iter().map(|t| t.content.clone()));
                answer(turns, opts, sink, "Bonjour.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Fais une page d'accueil.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(60);
    while !files.mailbox().iter().any(|l| l["kind"] == "ended") && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let seen = seen.lock().unwrap().join("\n");
    assert!(seen.contains("belongs to the user") && seen.contains("wonder-1") && seen.contains("acme"), "the step says to sign nothing, naming the session's model and provider: {seen}");
    assert!(!seen.contains("dans report") && !seen.contains("rends compte (report)"), "no step pushes report as the channel: {seen}");
}

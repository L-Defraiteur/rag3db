//! **The prompt says to write to the user first, in plain words** (the isolated calls of 23 Nov 2026: with the
//! teammate's prompt and its tools, `gemini-3.8-flash` answered « bonjour! » by tool calls only, 0 text in 18 calls;
//! with this sentence at the head of the prompt, text 3 times in 3, then the tool, in the same answer). The sentence is
//! declared (`teammate.reply_first`) and heads the teammate's prompt. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_teammate_prompt_begins_by_saying_to_write_to_the_user_first() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let system = Arc::new(Mutex::new(String::new()));
    let model: ChatModel = {
        let system = system.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let system = system.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
                if let Some(t) = turns.iter().find(|t| t.role == "system") {
                    *system.lock().unwrap() = t.content.clone();
                }
                answer(turns, opts, sink, "Bonjour !", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
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
    let s = system.lock().unwrap().clone();
    let phrase = "Always write a short passage in plain words to the user";
    let teammate = s.find("You are a teammate").unwrap_or(usize::MAX);
    assert!(s.find(phrase).is_some_and(|i| i < teammate), "the sentence heads the teammate's prompt: {s}");
    assert!(s.contains("answering first what they wrote") && s.contains("at the end of the turn, to explain what you did"), "the answer to what she wrote first, and a word at the end of the turn: {s}");
    // Measured on 23 Nov 2026: without it, a text before the first tool 6 times in 16; with it, 16 in 16.
    assert!(s.contains("Before your first tool call of a turn, write one short sentence"), "a word before the first tool: {s}");
}

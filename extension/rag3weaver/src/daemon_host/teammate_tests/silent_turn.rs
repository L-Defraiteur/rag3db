//! **A turn never ends without a word to its user** (Lucie's trial of 23 Nov 2026: the agent wrote files, called
//! step_done, and never wrote her a sentence). A turn opened by a human (a request or an instruction) that ends with no line of the agent is
//! relaunched once, saying step_done closes a step and is not an answer. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_turn_that_wrote_nothing_to_its_user_is_relaunched_once() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let calls = Arc::new(Mutex::new(0usize));
    let model: ChatModel = {
        let calls = calls.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let calls = calls.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let n = { let mut c = calls.lock().unwrap(); *c += 1; *c };
                let done = json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" });
                match n {
                    1 => answer(turns, opts, sink, "", calls_of(&[("edit_file", json!({ "path": "a.txt", "content": "a\n" }))])),
                    2 => answer(turns, opts, sink, "", calls_of(&[("step_done", done)])),
                    _ => answer(turns, opts, sink, "J'ai écrit a.txt ; il ne reste rien.", calls_of(&[("step_done", done)])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Écris a.txt.", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let end = Instant::now() + Duration::from_secs(60);
    while !files.mailbox().iter().any(|l| l["kind"] == "ended") {
        assert!(Instant::now() < end, "the turn ends: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    // Since the refusal of a silent step_done (23 Nov 2026), the agent writes in the same step: no step relaunched.
    assert!(
        b.iter().any(|l| l["from"] == "teammate" && l["text"].as_str().is_some_and(|t| t.contains("J'ai écrit a.txt"))),
        "the step, its silent step_done refused, writes to its user: {b:#?}"
    );
    assert_eq!(*calls.lock().unwrap(), 3, "one call more, not a whole step");
}

fn calls_of(v: &[(&str, Value)]) -> Vec<(String, String)> {
    calls(v)
}

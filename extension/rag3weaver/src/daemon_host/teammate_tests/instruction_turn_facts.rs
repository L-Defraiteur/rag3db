//! **A turn opened by an instruction says its facts too** (Lucie's trial, 23 Nov 2026: the second and third turns,
//! opened by `instruction` lines, had no `turn_changes`; the guard looked for the last `request` and found the first
//! turn's). Each turn says its facts once, whatever line opened it. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_turn_opened_by_an_instruction_says_its_facts() {
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
    let files = host.teammate_files(&h.id, "doc").ok();
    for (i, (kind, text)) in [("request", "Premier tour."), ("instruction", "Second tour, par une instruction.")].iter().enumerate() {
        host.post_to_teammate(&h.id, "doc", "lucie", kind, None, text, &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
        let files = host.teammate_files(&h.id, "doc").unwrap();
        let end = Instant::now() + Duration::from_secs(120);
        while files.mailbox().iter().filter(|l| l["kind"] == "ended").count() <= i {
            assert!(Instant::now() < end, "in 120 s: {:#?}", files.mailbox());
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let _ = files;
    let b = host.teammate_files(&h.id, "doc").unwrap().mailbox();
    let faits = b.iter().filter(|l| l["from"] == "host" && l["kind"] == "turn_changes").count();
    assert_eq!(faits, 2, "each turn says its facts, the instruction's too: {b:#?}");
}

//! **A reply cut at the output limit is not a silent step** (Lucie's trial of the terminal, 23 Nov 2026, 10 h 02:
//! three calls cut at max_tokens=4096 with no output, 75 s, an empty folder, and « step 1 ended without step_done and
//! without an answer »). A step's output limit is the teammate's (`teammate.max_output_tokens`, larger than the chat's
//! 4 096); a cut reply is relaunched saying what to do (a large file is written in parts); and a step that ends cut
//! says so to the user, with the reason. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_step_cut_at_the_output_limit_says_so_and_is_told_to_write_in_parts() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let seen = Arc::new(Mutex::new(Vec::<(usize, String)>::new()));
    let model: ChatModel = {
        let seen = seen.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let seen = seen.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let tout = turns.iter().map(|t| t.content.clone()).collect::<Vec<_>>().join("\n");
                let mut s = seen.lock().unwrap();
                s.push((opts.max_tokens, tout));
                // The first step's three calls are cut, with nothing out; then the work ends.
                if s.len() <= 3 {
                    let usage = crate::llm::Usage { prompt_tokens: 100, ..Default::default() };
                    return Ok((crate::llm::Finish::max_tokens(), usage));
                }
                answer(turns, opts, sink, "", calls(&[("report", json!({ "text": "noté" })), ("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Fais-moi un site three.js.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let end = Instant::now() + Duration::from_secs(120);
    while seen.lock().unwrap().len() < 4 {
        assert!(Instant::now() < end, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(500));
    let s = seen.lock().unwrap().clone();
    assert!(s[0].0 > 4096, "a step's output limit is the teammate's, above the chat's 4 096: {}", s[0].0);
    assert!(s[1].1.contains("en plusieurs morceaux") || s[1].1.contains("in parts"), "the relaunch says how to write a large file");
    let b = files.mailbox();
    let dit = b.iter().find(|l| l["from"] == "host" && l["cut"] == true).unwrap_or_else(|| panic!("the step cut is said: {b:#?}"));
    assert!(dit["text"].as_str().unwrap_or_default().contains("limite de sortie"), "{dit}");
}

//! **A full step writes what it found before it ends** (the measures C, E and F of 24 Nov 2026: past the step's bound,
//! half the names of the report were invented, the next step having only automatic lines; the model knows which names
//! it means to cite). A step_done of a full step that read code, with no text after its last tool, is refused once with
//! « write first the names, paths and lines you will use »; the note passes to the next step. By the host's own path.
//! A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_full_step_writes_its_findings_and_the_next_step_reads_them() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"] = json!({ "step_context_tokens": 1, "step_context_share": 0 });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "une petite borne"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let vus = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let vus = vus.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let vus = vus.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
                let step = step_of(turns);
                let results = tool_results(turns);
                if step >= 2 && results.is_empty() {
                    vus.lock().unwrap().push(last_user(turns));
                }
                let asked_to_write = results.iter().any(|r| r.contains("names, paths and lines"));
                match (step, results.len(), asked_to_write) {
                    (1, 0, _) => answer(turns, opts, sink, "Je lis le code.", calls(&[("read_file", json!({ "path": "src/lib.rs" }))])),
                    (1, _, false) => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "lu", "remaining": "écrire le rapport" }))])),
                    (1, _, true) => answer(turns, opts, sink, "Noté pour la suite : bonjour — src/lib.rs:1.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "lu", "remaining": "écrire le rapport" }))])),
                    _ => answer(turns, opts, sink, "Le rapport est écrit.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "écrit", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Lis le code, puis écris un rapport.", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(90);
    while !ended(&files.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let vus = vus.lock().unwrap().join("\n");
    assert!(vus.contains("bonjour — src/lib.rs:1"), "the full step's note of its findings reaches the next step: {vus}");
}

//! **The code teammate's output limit is 40 000 tokens** (Lucie, 23 Nov 2026, 10 h 3x: « ça ne va pas du tout ; il
//! faut au moins 40 000 » ; the chat's 4 096, thinking included on Vertex, gave empty replies). Declared in the code
//! template (`teammate.max_output_tokens`), and capped by the model's declared maximum (`models.llm[].max_output_tokens`)
//! instead of a provider's refusal. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

/// The `max_tokens` the first model call of a step receives, with `models` declared in the manifest when given.
fn sent_max_tokens(models: Option<Value>) -> usize {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    if let Some(models) = models {
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
        m["models"]["llm"] = models;
        std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        let decl = manifeste.ancestors().find(|p| p.join(".git").exists()).unwrap().to_path_buf();
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "the model"]] {
            assert!(std::process::Command::new("git").args(args).current_dir(&decl).status().unwrap().success());
        }
    }
    let sent = Arc::new(Mutex::new(None::<usize>));
    let model: ChatModel = {
        let sent = sent.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let sent = sent.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                sent.lock().unwrap().get_or_insert(opts.max_tokens);
                answer(turns, opts, sink, "", calls(&[("report", json!({ "text": "noté" })), ("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Fais-moi un site.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let end = Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(n) = *sent.lock().unwrap() {
            return n;
        }
        assert!(Instant::now() < end, "in 120 s, no model call");
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_code_teammate_asks_for_40000_output_tokens() {
    assert_eq!(sent_max_tokens(None), 40_000);
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_model_declared_at_8192_receives_8192() {
    let models = json!([{ "name": "petit", "provider": "compatible", "protocol": "vertex", "model": "gemini-x", "max_output_tokens": 8192 }]);
    assert_eq!(sent_max_tokens(Some(models)), 8192);
}

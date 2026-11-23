//! **The end-of-step rules are a file** (Lucie, 23 Nov 2026: « qu'on ne doive jamais compiler pour mettre des
//! classifiers ou autre là où ça nous plaît »). The rules that refuse a silent step_done live in the teammate's
//! `at_step_done.mmd` and its table; a user's table that maps them to `accept` lets a silent step_done through, without a
//! build. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_users_table_turns_the_silent_turn_rule_off_without_a_build() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let graphes = d.path().join("code-roots/config/templates/teammate/graphs");
    std::fs::create_dir_all(&graphes).unwrap();
    std::fs::write(graphes.join("at_step_done.json"), json!({ "silent_turn": { "action": "accept" }, "reply_after_tools": { "action": "accept" }, "accept": { "action": "accept" } }).to_string()).unwrap();
    let refus = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let refus = refus.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let refus = refus.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                refus.lock().unwrap().extend(tool_results(turns).into_iter().filter(|r| r.contains("\"ok\":false")));
                answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Regarde le projet.", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(60);
    while !ended(&files.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let refus = refus.lock().unwrap().clone();
    assert!(refus.is_empty(), "her table accepts the silent step_done, without a build: {refus:?}");
}

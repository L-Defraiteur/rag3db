//! **A teammate launched with only some tools** (banc, 23 Nov 2026: a witness agent with grep and reading only, the
//! same model as the agent measured; a manifest cut down to three tools still gave sixteen). `teammate.tools_only`, or
//! the request's `tools_only` at the launch: the backend's tools offered to the model are these and no other (the
//! step's own tools stay: report, step_done…), and a call to another is refused, saying why. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_launch_with_tools_only_offers_those_and_refuses_the_others() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let offered = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let results = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let (offered, results) = (offered.clone(), results.clone());
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (offered, results) = (offered.clone(), results.clone());
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                offered.lock().unwrap().push(opts.tools.iter().map(|t| t.name.clone()).collect());
                let got = tool_results(turns);
                results.lock().unwrap().extend(got.iter().cloned());
                if got.is_empty() {
                    return answer(turns, opts, sink, "", calls(&[("write", json!({ "path": "x.txt", "content": "x" }))]));
                }
                answer(turns, opts, sink, "", calls(&[("report", json!({ "text": "noté" })), ("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let only = json!(["read_file", "grep_files", "list_files"]);
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Lis le dépôt.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac", "tools_only": only })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let end = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < end, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let first = offered.lock().unwrap().first().cloned().unwrap_or_default();
    for t in ["read_file", "grep_files", "list_files", "report", "step_done"] {
        assert!(first.iter().any(|n| n == t), "{t} is offered: {first:?}");
    }
    for t in ["search_code", "usages", "write", "run_command", "fetch", "web_search"] {
        assert!(!first.iter().any(|n| n == t), "{t} is not offered: {first:?}");
    }
    let r = results.lock().unwrap().clone();
    assert!(r.iter().any(|x| x.contains("write") && x.contains("tools_only")), "a call to another tool is refused, saying why: {r:#?}");
}

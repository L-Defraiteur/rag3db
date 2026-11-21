//! **A teammate's memory writes are signed with its agent key** (M7, 21 November 2026): the host makes them for the
//! agent (`as_agent`), and `writes.author` signs the rows. A file of its own (c7's rule).

use super::*;

/// A memory that records, for each turn written, the agent the call is made for.
struct TurnsFor(Mutex<Vec<Option<String>>>);

impl crate::daemon_host::rangement::Rangeur for TurnsFor {
    fn put_turn(&self, _memoire: &std::path::Path, _line: &Value) -> Result<Value, String> {
        self.0.lock().unwrap().push(crate::backend::current_agent());
        Ok(json!({ "written": true }))
    }
    fn archive_slice(&self, _memoire: &std::path::Path, args: &Value) -> Result<Value, String> {
        Ok(json!({ "report": { "ok": true, "errors": [] }, "archived_through_turn": args["to_turn"] }))
    }
}

/// The turns of a teammate's steps go to the memory for its agent: each `put_turn` is made under the task's key.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_teammate_writes_its_turns_under_its_agent_key() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["bilans"] = json!({ "memoire": "memoire/backend.json" });
    m["teammate"] = json!({ "reply_wait_s": 2 });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "the memory of the session"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("signed", 8192, |turns, opts, sink| {
            answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "said hello", "remaining": "" }))]))
        })))
    });
    let memory = Arc::new(TurnsFor(Mutex::new(Vec::new())));
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_rangeur(memory.clone()));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the parent opens");
    let t = host.ouvrir_une_tache(&h.id, "doc").expect("the task opens");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append("lucie", "request", None, "Say hello").unwrap();
    let _ = host.run_teammate(&h.id, "doc");
    let agents = memory.0.lock().unwrap().clone();
    assert!(!agents.is_empty(), "the steps' turns went to the memory");
    let key = t.agent_key.clone().expect("the task has a key");
    assert!(agents.iter().all(|a| a.as_deref() == Some(key.as_str())), "each turn for {key}: {agents:?}");
}

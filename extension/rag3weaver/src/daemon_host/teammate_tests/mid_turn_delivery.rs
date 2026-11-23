//! **A message written during the turn reaches the agent at its next tool result** (Lucie's trial of 23 Nov 2026, 20 h
//! 50: « hésite pas à essayer l'index stp », written while the agent called its tools, was read one step later). What the
//! mailbox receives during a step comes at the head of the next tool result of that same step. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use crate::daemon_host::teammate::TeammateFiles;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_message_written_during_the_turn_comes_with_the_next_tool_result() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let boite: Arc<Mutex<Option<TeammateFiles>>> = Arc::new(Mutex::new(None));
    let vus_au_pas_1 = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let (boite, vus) = (boite.clone(), vus_au_pas_1.clone());
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (boite, vus) = (boite.clone(), vus.clone());
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if step_of(turns) <= 1 {
                    vus.lock().unwrap().extend(results.iter().cloned());
                }
                match results.len() {
                    0 => {
                        // She writes while the agent calls its first tool.
                        if let Some(f) = boite.lock().unwrap().as_ref() {
                            f.append_with("lucie", "instruction", None, "hésite pas à essayer l'index stp", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
                        }
                        answer(turns, opts, sink, "Je regarde le projet.", calls(&[("list_files", json!({}))]))
                    }
                    1 => answer(turns, opts, sink, "", calls(&[("list_files", json!({}))])),
                    _ => answer(turns, opts, sink, "J'ai regardé le projet.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "regardé", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.ouvrir_une_tache(&h.id, "doc").expect("the task opens");
    *boite.lock().unwrap() = Some(host.teammate_files(&h.id, "doc").unwrap());
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Regarde le projet.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(60);
    while !ended(&files.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let vus = vus_au_pas_1.lock().unwrap().join("\n");
    assert!(vus.contains("hésite pas à essayer l'index"), "her message reaches the agent within the step, at the head of a tool result: {vus}");
}

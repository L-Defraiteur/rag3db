//! **A report that the facts contradict is given back once** (c7, 23 Nov 2026: a report said « ajout d'un test dédié »
//! after seven commands and no write). At a turn's end, when the teammate's report says it wrote, added or created and
//! the turn changed no file, or says the tests pass and a command failed, the host gives that fact back to the agent,
//! in a sentence, once, before the turn ends. No judge of text: words declared in `teammate.facts_check`, with a
//! switch. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_report_of_an_addition_with_no_file_changed_is_given_back_once() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let heard = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let heard = heard.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let heard = heard.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                heard.lock().unwrap().push(turns.iter().map(|t| t.content.clone()).collect::<Vec<_>>().join("\n"));
                answer(turns, opts, sink, "", calls(&[("report", json!({ "text": "J'ai ajouté le test test_delete." })), ("step_done", json!({ "item": "", "succeeded": true, "done": "test ajouté", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Ajoute un test pour la suppression.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let end = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < end, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    let rendu = b.iter().find(|l| l["from"] == "host" && l["contradiction"] == true).unwrap_or_else(|| panic!("the fact is given back: {b:#?}"));
    assert!(rendu["text"].as_str().unwrap_or_default().contains("aucun fichier"), "{rendu}");
    assert_eq!(b.iter().filter(|l| l["from"] == "host" && l["contradiction"] == true).count(), 1, "once");
    assert!(heard.lock().unwrap().iter().any(|m| m.contains("aucun fichier")), "the agent hears it");
}

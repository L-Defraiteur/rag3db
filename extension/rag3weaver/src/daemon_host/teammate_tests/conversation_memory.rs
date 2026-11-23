//! **A turn receives the turns before it** (c7, 23 Nov 2026: in a folder without git, « ok corrige » answered « le
//! contexte n'est pas spécifié »; with git the agent had found its way by `git log`, by chance). A turn's message
//! carries the conversation's earlier requests and reports, the last ones in full, the older ones bounded. A module of
//! its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

/// A fake teammate that ends each step at once, and keeps what each call received.
fn listening(heard: Arc<Mutex<Vec<String>>>) -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let heard = heard.clone();
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            heard.lock().unwrap().push(turns.iter().map(|t| t.content.clone()).collect::<Vec<_>>().join("\n"));
            answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "trouvé : la suppression ne retire rien", "remaining": "" }))]))
        })))
    })
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_second_turn_receives_the_first() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let heard = Arc::new(Mutex::new(Vec::new()));
    let gate = d.path().join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d.path(), listening(heard.clone())).with_runs_root(d.path().join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let tour = |texte: &str| {
        host.post_to_teammate(&h.id, "doc", "lucie", "request", None, texte, &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
        let files = host.teammate_files(&h.id, "doc").unwrap();
        let fin = Instant::now() + Duration::from_secs(120);
        let n = files.mailbox().iter().filter(|l| l["kind"] == "ended").count();
        while files.mailbox().iter().filter(|l| l["kind"] == "ended").count() <= n {
            assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
            std::thread::sleep(Duration::from_millis(100));
        }
    };
    tour("Trouve le défaut de suppression dans notes_api.py, la route DELETE /notes/{id}.");
    let avant = heard.lock().unwrap().len();
    tour("ok corrige ça");
    let second: Vec<String> = heard.lock().unwrap()[avant..].to_vec();
    assert!(!second.is_empty());
    assert!(second.iter().any(|m| m.contains("notes_api.py") && m.contains("DELETE /notes/{id}")), "the second turn receives the first request: {second:#?}");
    assert!(second.iter().any(|m| m.contains("la suppression ne retire rien")), "and its report: {second:#?}");
}

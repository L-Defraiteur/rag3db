//! **A card's answer reaches the step in progress** (Lucie's trial of 23 Nov 2026, 20 h 50: her « oui » to a root came
//! with the host's grant, read one step later). A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use crate::daemon_host::teammate::TeammateFiles;
use crate::daemon_host::DaemonHost;
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

/// A host and a task whose fake teammate gets the task's mailbox, to write in it during its step.
fn banc(model: impl Fn(Arc<Mutex<Option<TeammateFiles>>>) -> ChatModel) -> (tempfile::TempDir, Arc<DaemonHost>, TeammateFiles) {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let boite: Arc<Mutex<Option<TeammateFiles>>> = Arc::new(Mutex::new(None));
    let host = Arc::new(hote(d.path(), model(boite.clone())).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.ouvrir_une_tache(&h.id, "doc").expect("the task opens");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    *boite.lock().unwrap() = Some(files.clone());
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Regarde le projet.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    (d, host, files)
}

fn attendre(files: &TeammateFiles, fini: &dyn Fn(&[Value]) -> bool) -> Vec<Value> {
    let fin = Instant::now() + Duration::from_secs(60);
    loop {
        let b = files.mailbox();
        if fini(&b) || Instant::now() >= fin {
            return b;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_cards_grant_comes_with_the_next_tool_result() {
    let vus = Arc::new(Mutex::new(Vec::<String>::new()));
    let (_d, _host, files) = banc(|boite| {
        let vus = vus.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (boite, vus) = (boite.clone(), vus.clone());
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let results = tool_results(turns);
                if step_of(turns) <= 1 {
                    vus.lock().unwrap().extend(results.iter().cloned());
                }
                match results.len() {
                    0 => {
                        if let Some(f) = boite.lock().unwrap().as_ref() {
                            f.append_with("lucie", "answer", None, "oui", &json!({ "by": { "kind": "human" }, "approved": true, "reply_to": "x:1", "granted": "racine accordée : /ailleurs (read) — relance ta commande mot pour mot : cat /ailleurs/notes.txt" })).unwrap();
                        }
                        answer(turns, opts, sink, "Je regarde le projet.", calls(&[("list_files", json!({}))]))
                    }
                    1 => answer(turns, opts, sink, "", calls(&[("list_files", json!({}))])),
                    _ => answer(turns, opts, sink, "J'ai regardé le projet.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "regardé", "remaining": "" }))])),
                }
            })))
        })
    });
    attendre(&files, &ended);
    let vus = vus.lock().unwrap().join("\n");
    assert!(vus.contains("racine accordée : /ailleurs"), "the grant her yes brought reaches the step in progress: {vus}");
}

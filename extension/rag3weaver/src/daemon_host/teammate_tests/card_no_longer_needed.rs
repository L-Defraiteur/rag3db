//! **A card the finished work still waits on says so, and « plus besoin » closes it** (Lucie, 23 Nov 2026, ~23 h 15, the
//! third of the three choices: the card stays; the waiting line says the work is done; « plus besoin » closes it without
//! replaying anything, and the turn ends without another step). A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use crate::daemon_host::teammate::TeammateFiles;
use super::mode_auto_v1::ended;
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

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
fn plus_besoin_closes_the_card_of_finished_work_and_ends_the_turn() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let boite: Arc<Mutex<Option<TeammateFiles>>> = Arc::new(Mutex::new(None));
    let appels = Arc::new(AtomicUsize::new(0));
    let model: ChatModel = {
        let (boite, appels) = (boite.clone(), appels.clone());
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (boite, appels) = (boite.clone(), appels.clone());
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                appels.fetch_add(1, Ordering::SeqCst);
                match tool_results(turns).len() {
                    0 => {
                        if let Some(f) = boite.lock().unwrap().as_ref() {
                            f.append_with("teammate", "approval", None, "commande en attente d'accord : python3 lire.py", &json!({ "command": "python3 lire.py" })).unwrap();
                        }
                        answer(turns, opts, sink, "Je regarde le projet.", calls(&[("list_files", json!({}))]))
                    }
                    _ => answer(turns, opts, sink, "C'est fait, sans cette commande.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
                }
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.ouvrir_une_tache(&h.id, "doc").expect("the task opens");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    *boite.lock().unwrap() = Some(files.clone());
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Regarde le projet.", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let b = attendre(&files, &|b| b.iter().any(|l| l["from"] == "host" && !l["awaiting"].is_null()) || ended(b));
    let attente = b.iter().find(|l| l["from"] == "host" && !l["awaiting"].is_null()).cloned().unwrap_or_default();
    assert!(attente["text"].as_str().is_some_and(|t| t.contains("travail est fini") && t.contains("plus besoin")), "the waiting line says the work is done and offers « plus besoin »: {b:#?}");
    let carte = attente["awaiting"].as_str().unwrap_or_default().to_string();
    let avant = appels.load(Ordering::SeqCst);
    host.post_to_teammate(&h.id, "doc", "lucie", "answer", None, "plus besoin", &json!({ "reply_to": carte, "approved": false, "dismissed": true, "by": { "kind": "human", "client": "tty" } })).unwrap();
    let b = attendre(&files, &|b| b.iter().filter(|l| l["kind"] == "ended").count() >= 1);
    assert!(ended(&b), "« plus besoin » ends the turn: {b:#?}");
    assert_eq!(appels.load(Ordering::SeqCst), avant, "no step after « plus besoin »: the work was done");
}

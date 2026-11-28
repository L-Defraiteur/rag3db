//! **The agent is told, at the folder's entry, whether it has its code index and which folders under it have theirs**
//! (Lucie, 24 Nov 2026: « juste un index à la demande, ou qu'il les liste s'il y en a déjà, qu'il soit notifié à
//! l'entrée dans le dossier »). By the host's own path: the first step of a turn in a registered folder that holds
//! another registered folder. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_first_step_says_the_folders_index_and_the_indexes_inside_it() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let outils = depot.join("outils");
    std::fs::create_dir_all(&outils).unwrap();
    std::fs::write(outils.join("lib.rs"), "pub fn outil() {}\n").unwrap();
    let roots = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    crate::code_tree::add_files(&roots, &outils, "code-lourd", Some("code-outils")).unwrap();
    let premiers = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let premiers = premiers.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let premiers = premiers.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
                if step_of(turns) <= 1 && tool_results(turns).is_empty() {
                    premiers.lock().unwrap().push(last_user(turns));
                }
                answer(turns, opts, sink, "Voici.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "répondu", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "t", "lucie", "request", None, "Où est défini outil ?", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let files = host.teammate_files(&h.id, "t").unwrap();
    let limite = Instant::now() + Duration::from_secs(60);
    while !ended(&files.mailbox()) && Instant::now() < limite {
        std::thread::sleep(Duration::from_millis(100));
    }
    let premier = premiers.lock().unwrap().first().cloned().unwrap_or_default();
    assert!(premier.contains("a son index de code"), "the folder's own index is said: {premier}");
    assert!(premier.contains("code-outils"), "and the folder inside with its index: {premier}");
}

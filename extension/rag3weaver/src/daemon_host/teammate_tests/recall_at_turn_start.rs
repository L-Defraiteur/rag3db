//! **A turn starts with what the long-term memory holds for its request, and with nothing when it holds nothing**
//! (the presentation's « Shared Long-Term Memory »; the hook `at_turn_start.mmd`, 24 Nov 2026). The code backend
//! declares its memory (`bilans.memoire`); an apport filed there before is recalled into the first step of a turn whose
//! request it answers, and an unrelated request gets no block. By the host's own path. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_turn_starts_with_what_the_memory_holds_for_its_request() {
    let d = tempfile::tempdir().unwrap();
    let m = d.path().join("memoire");
    std::fs::create_dir_all(&m).unwrap();
    let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/memory");
    let memoire = crate::gabarits::jouable::manifeste_jouable(&gabarit, &m).unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut c: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    c["bilans"] = json!({ "memoire": memoire.to_string_lossy() });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&c).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "une mémoire"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let premiers = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let premiers = premiers.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let premiers = premiers.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
                if step_of(turns) <= 1 && tool_results(turns).is_empty() {
                    premiers.lock().unwrap().push(last_user(turns));
                }
                answer(turns, opts, sink, "Voici ma réponse.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "répondu", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let mem = host.hosted(&memoire).unwrap();
    let ecrire = |outil: &str, record: Value| {
        let r = host.with_backend(&mem, |b| b.call(outil, json!({ "record": record }))).unwrap();
        assert!(r.is_ok(), "{outil} : {r:?}");
    };
    ecrire("put_fiche", json!({ "name": "contexte d'un pas", "about": "la borne de contexte d'un pas de l'équipier", "state": "current" }));
    ecrire("put_source", json!({ "chemin": "sources/2026-11-24-contexte.md", "titre": "contexte", "contenu": "la borne", "origine": "envoi" }));
    ecrire("put_apport", json!({ "fiche": "contexte d'un pas", "source": "sources/2026-11-24-contexte.md", "section": "borne", "texte": "La borne de contexte d'un pas vaut un quart de la fenêtre du modèle.", "niveau": "L1", "state": "current" }));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let tour = |tache: &str, demande: &str| {
        host.post_to_teammate(&h.id, tache, "lucie", "request", None, demande, &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
        let files = host.teammate_files(&h.id, tache).unwrap();
        let fin = Instant::now() + Duration::from_secs(60);
        while !ended(&files.mailbox()) && Instant::now() < fin {
            std::thread::sleep(Duration::from_millis(100));
        }
    };
    tour("doc", "Que vaut la borne de contexte d'un pas de l'équipier ?");
    tour("autre", "Quelle heure est-il à Tokyo ?");
    let premiers = premiers.lock().unwrap().clone();
    assert!(premiers.first().is_some_and(|m| m.contains("un quart de la fenêtre")), "the first step of the turn gets the apport that answers its request: {premiers:#?}");
    assert!(premiers.get(1).is_some_and(|m| !m.contains("un quart de la fenêtre")), "an unrelated request gets no block: {premiers:#?}");
}

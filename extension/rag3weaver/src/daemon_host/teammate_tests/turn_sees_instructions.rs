//! **A turn opened by what the user typed after her first message sees it, and the answer it replies to in full**
//! (24 Nov 2026: « oui je suis oki pour tes modifications » reached a turn that saw neither these words nor the proposal
//! they accepted — the terminal sends them as an `instruction`, and only `request` lines were counted; the last answer
//! was cut at 2000 characters). By the host's own path, two turns of one task. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_turn_opened_by_an_instruction_sees_it_and_the_proposal_in_full() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    // A proposal longer than 2000 characters, its end marked.
    let proposition = format!("Je propose de renommer ces fonctions. {} FIN-DE-LA-PROPOSITION. D'accord ?", "détail du renommage ; ".repeat(150));
    let premiers = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let (premiers, proposition) = (premiers.clone(), proposition.clone());
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (premiers, proposition) = (premiers.clone(), proposition.clone());
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
                if step_of(turns) <= 1 && tool_results(turns).is_empty() {
                    premiers.lock().unwrap().push(last_user(turns));
                }
                let first = premiers.lock().unwrap().len() <= 1;
                let text = if first { proposition.clone() } else { "C'est fait.".to_string() };
                answer(turns, opts, sink, &text, calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "répondu", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let tour = |kind: &str, texte: &str, fins: usize| {
        host.post_to_teammate(&h.id, "t", "lucie", kind, None, texte, &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
        let files = host.teammate_files(&h.id, "t").unwrap();
        let limite = Instant::now() + Duration::from_secs(60);
        while files.mailbox().iter().filter(|l| l["kind"] == "ended").count() < fins && Instant::now() < limite {
            std::thread::sleep(Duration::from_millis(100));
        }
    };
    tour("request", "Propose un renommage des fonctions de land.rs, sans rien écrire.", 1);
    tour("instruction", "oui, vas-y", 2);
    let premiers = premiers.lock().unwrap().clone();
    let second = premiers.get(1).cloned().unwrap_or_default();
    assert!(second.contains("oui, vas-y"), "the turn sees the words that opened it: {second}");
    assert!(second.contains("FIN-DE-LA-PROPOSITION"), "and the proposal they accept, in full: {second}");
}

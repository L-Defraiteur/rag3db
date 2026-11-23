//! **The teammate's prompt is a template, edited without restarting anything** (Lucie, 23 Nov 2026: « comment ça se
//! fait que son invite ne soit pas un gabarit modifiable sans tout relancer ? »). The prompt's texts are files shipped
//! under `templates/teammate/prompts/`, overridden by the user's own file under
//! `<config>/templates/teammate/prompts/`, read again at each step: an edit is in the next turn. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn an_edited_prompt_file_is_in_the_next_turn_without_a_restart() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let systems = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let systems = systems.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let systems = systems.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
                if let Some(t) = turns.iter().find(|t| t.role == "system") {
                    systems.lock().unwrap().push(t.content.clone());
                }
                answer(turns, opts, sink, "Bonjour !", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let roots = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let prompts = roots.config.join("templates/teammate/prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    std::fs::write(prompts.join("reply_first.md"), "MARK-ONE: write to {$user} first.\n").unwrap();
    let host = Arc::new(hote(d.path(), model).with_racines_de_code(roots).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "bonjour!", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let wait = |n: usize| {
        let fin = Instant::now() + Duration::from_secs(60);
        while files.mailbox().iter().filter(|l| l["kind"] == "ended").count() < n && Instant::now() < fin {
            std::thread::sleep(Duration::from_millis(100));
        }
    };
    wait(1);
    std::fs::write(prompts.join("reply_first.md"), "MARK-TWO: write to {$user} first.\n").unwrap();
    host.post_to_teammate(&h.id, "doc", "lucie", "instruction", None, "encore", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    wait(2);
    let s = systems.lock().unwrap().clone();
    assert!(s.first().is_some_and(|p| p.contains("MARK-ONE")), "the user's file is the prompt: {s:#?}");
    assert!(s.last().is_some_and(|p| p.contains("MARK-TWO") && !p.contains("MARK-ONE")), "the edit is in the next turn, without a restart: {s:#?}");
    assert!(s.iter().all(|p| !p.contains("{$user}")), "the variables are filled: {s:#?}");
}

/// **The step's message and the relaunches are template texts too** (the remaining prompt texts, 23 Nov 2026): the
/// user's `texts.json` overrides one key, and the next step's message carries it; the other keys stay as shipped.
#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_text_of_the_step_message_is_overridden_by_the_user_key_by_key() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let frontiers = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let frontiers = frontiers.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let frontiers = frontiers.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
                frontiers.lock().unwrap().push(last_user(turns));
                answer(turns, opts, sink, "Bonjour !", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let roots = crate::code_tree::Roots { config: d.path().join("config"), data: d.path().join("data") };
    let prompts = roots.config.join("templates/teammate/prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    std::fs::write(prompts.join("texts.json"), r#"{ "frontier.answer_or_act": "MARK-THREE: answer {$user}." }"#).unwrap();
    let host = Arc::new(hote(d.path(), model).with_racines_de_code(roots).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "bonjour!", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(60);
    while !files.mailbox().iter().any(|l| l["kind"] == "ended") && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let f = frontiers.lock().unwrap().clone();
    assert!(f.first().is_some_and(|m| m.contains("MARK-THREE: answer the user.")), "the user's key is in the step's message, filled: {f:#?}");
    assert!(f.first().is_some_and(|m| m.contains("Boîte aux lettres, depuis le pas précédent")), "the other keys stay as shipped: {f:#?}");
}

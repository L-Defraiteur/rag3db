//! **A request for code in a repository's chat opens the code task, and the conversation moves there** (Lucie's dry
//! run, 22 Nov 2026: the chat answered « open a task in the IDE »). The chat's agent calls `open_code_task`; the host
//! opens the task (its branch, its tree, its edit and command tools), writes the user's own message there as her request
//! (by human), starts the teammate, and returns `moved_to_task` with the link the IDE follows. A module of its own.

use super::chat::ChatModel;
use super::chat_qui_code::{depot_inscrit, hote};
use crate::llm::{CallbackLlm, Llm, MockLlm};
use serde_json::{json, Value};
use std::sync::Arc;

/// The chat's fake model: it calls `open_code_task` when offered and not yet called, then answers in words.
fn qui_ouvre() -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-modele", 8192, move |turns, opts, sink| {
            let offert = opts.tools.iter().any(|t| t.name == "open_code_task");
            if !offert || turns.last().is_some_and(|t| t.role == "tool") {
                return MockLlm::new("D'accord.").generate(turns, opts, sink);
            }
            MockLlm::new("").with_tool_calls(vec![("open_code_task", "{}")]).generate(turns, opts, sink)
        })))
    })
}

const DEMANDE: &str = "Corrige la fonction bonjour pour qu'elle dise salut.";

#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_request_for_code_in_a_repositorys_chat_opens_the_task_and_moves_there() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let host = Arc::new(hote(d.path(), qui_ouvre()));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.chat_turn(&h.id, DEMANDE).expect("the turn");
    let etat = host.chat_state(&h.id).unwrap();
    let resultat: Value = etat["turns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["role"] == "tool")
        .and_then(|t| serde_json::from_str(t["content"].as_str().unwrap_or_default()).ok())
        .unwrap_or_else(|| panic!("open_code_task was offered and answered: {etat:#}"));
    assert_eq!(resultat["kind"], json!("moved_to_task"), "{resultat}");
    let tache = resultat["task"].as_str().expect("the task's name").to_string();
    let t = host.tache(&h.id, &tache).expect("the task is open");
    assert_eq!(resultat["backend"], json!(t.id), "{resultat}");
    assert_eq!(resultat["branch"], json!(t.branche), "{resultat}");
    assert_eq!(resultat["link"], json!(format!("/ide/code/{}/taches/{tache}", h.id)), "{resultat}");
    assert_eq!(resultat["request"], json!(DEMANDE), "{resultat}");
    let boite = host.teammate_files(&h.id, &tache).unwrap().mailbox();
    let req = boite.iter().find(|l| l["kind"] == "request").expect("the request in the task's mailbox");
    assert_eq!(req["text"], json!(DEMANDE), "the user's own words: {req}");
    assert_eq!(req["by"]["kind"], json!("human"), "{req}");
}

#[test]
fn the_repositorys_chat_prompt_says_to_open_the_task_itself() {
    let p = super::chat::system_prompt(crate::propositions::ModeDeclarations::Approbation, false);
    assert!(p.contains("open_code_task"), "{p}");
    assert!(!p.contains("tell them to open a task"), "{p}");
}

//! **The task page shows what the agent does** (the trial of 23 November 2026, three real requests: minutes of « en
//! train de travailler » and nothing else on the page, while the agent listed, read, wrote and ran commands). Each call
//! of a tool shows on the page as it comes: its tool, its argument, then ✓ or ✗ and an excerpt of what it gave. The
//! witness in its own file.

use super::chat::ChatModel;
use super::chat_qui_code::{depot_inscrit, hote};
use crate::llm::{CallbackLlm, Llm, MockLlm};
use serde_json::json;
use std::sync::Arc;

/// The chat's agent opens the task; the teammate waits for the page, lists the files, waits again, then ends.
fn works_slowly() -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-modele", 16384, move |turns, opts, sink| {
            if opts.tools.iter().any(|t| t.name == "open_code_task") {
                if turns.last().is_some_and(|t| t.role == "tool") {
                    return MockLlm::new("C'est parti dans la tâche.").generate(turns, opts, sink);
                }
                return MockLlm::new("").with_tool_calls(vec![("open_code_task", "{}")]).generate(turns, opts, sink);
            }
            let since = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
            let results = turns[since..].iter().filter(|t| t.role == "tool").count();
            std::thread::sleep(std::time::Duration::from_secs(4));
            match results {
                0 => MockLlm::new("Je regarde le projet.").with_context_len(16384).with_tool_calls(vec![("list_files", "{}")]).generate(turns, opts, sink),
                1 => MockLlm::new("J'ai regardé : il tient en une bibliothèque.")
                    .with_context_len(16384)
                    .with_tool_calls(vec![("step_done", &json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }).to_string())])
                    .generate(turns, opts, sink),
                _ => MockLlm::new("Fini.").with_context_len(16384).generate(turns, opts, sink),
            }
        })))
    })
}

#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_task_page_shows_each_call_of_the_agent_as_it_comes() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_task_page_shows_each_call_of_the_agent_as_it_comes") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let (_repository, manifest) = depot_inscrit(d.path(), true);
    let host = Arc::new(hote(d.path(), works_slowly()));
    host.se_connaitre();
    let h = host.hosted(&manifest).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let id = h.id.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let served = host.clone();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 2).serve(served, Some("j".to_string())));
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{base}/ide/code/{id}/chat"), "document.querySelector('#send') && !document.querySelector('#send').disabled");
    n.evaluer("document.querySelector('#message').value = 'Regarde le projet.'; document.querySelector('#send').click()");
    n.attendre(&format!("location.pathname.startsWith('/ide/code/{id}/taches/')"), "the page goes to the task");
    n.attendre("!!document.querySelector('.activite [data-tool=\"list_files\"][data-ok=\"true\"]')", "the call of list_files on the task page, finished");
    let line = n.evaluer("document.querySelector('.activite [data-tool=\"list_files\"]').innerText").as_str().unwrap_or_default().to_string();
    assert!(line.contains("list_files") && line.contains("✓"), "the call says its tool and that it went well: {line}");
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}

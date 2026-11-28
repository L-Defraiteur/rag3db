//! **A message sent during a turn is never lost** (the user's decision, 28 November 2026). A human message sent to a
//! repository's chat while a turn runs is accepted as a note of that turn: it is said « received during the turn »,
//! kept in the chat's session, and the model reads it at the head of its next tool result — or, when the turn ends
//! without another tool, as the message of the next turn. The witnesses in their own file.

use super::chat::ChatModel;
use super::chat_qui_code::{depot_inscrit, hote};
use crate::llm::{CallbackLlm, Llm, MockLlm};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const NOTE: &str = "ajoute aussi un README";

/// The tools that read, in the order the fake model prefers them.
const READERS: [&str; 4] = ["read_file", "list_files", "outline", "grep_files"];

/// A fake model whose first call is slow: it raises `sleeping`, sleeps `nap`, then calls a reading tool. After a tool
/// result, it ends in text, quoting the note when a tool result of the turn carried it; a turn whose message is the
/// note quotes it too.
fn slow_reader(sleeping: Arc<AtomicBool>, nap: std::time::Duration) -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let sleeping = sleeping.clone();
        Ok(Arc::new(CallbackLlm::new("faux-modele", 8192, move |turns, opts, sink| {
            let last = turns.last();
            if last.is_some_and(|t| t.role == "tool") {
                let start = turns.iter().rposition(|t| t.role == "user").unwrap_or(0);
                let seen = turns[start..].iter().any(|t| t.role == "tool" && t.content.contains(NOTE));
                let text = if seen { format!("Lu. Consigne reçue en route : « {NOTE} ».") } else { "Lu, sans consigne.".to_string() };
                return MockLlm::new(&text).generate(turns, opts, sink);
            }
            if last.is_some_and(|t| t.role == "user" && t.content.contains(NOTE)) {
                return MockLlm::new(&format!("Consigne reçue au tour suivant : « {NOTE} ».")).generate(turns, opts, sink);
            }
            let Some(tool) = READERS.iter().find(|n| opts.tools.iter().any(|t| t.name == **n)) else {
                return MockLlm::new("aucun outil de lecture").generate(turns, opts, sink);
            };
            sleeping.store(true, Ordering::SeqCst);
            std::thread::sleep(nap);
            let args = match *tool {
                "read_file" | "outline" => json!({ "path": "README.md" }),
                "grep_files" => json!({ "pattern": "Petit" }),
                _ => json!({}),
            }
            .to_string();
            MockLlm::new("").with_tool_calls(vec![(*tool, args.as_str())]).generate(turns, opts, sink)
        })))
    })
}

/// Waits for `flag`, at most 30 s.
fn wait_for(flag: &AtomicBool, what: &str) {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !flag.load(Ordering::SeqCst) {
        assert!(std::time::Instant::now() < until, "{what}: still not after 30 s");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// **A human message during a turn enters the turn** (28 November 2026; before, « un tour est déjà en cours sur ce
/// backend », and a message sent right after an approval card's decision vanished). The second send is accepted as
/// received during the turn, the note is in the chat's session marked so, and the model quotes it at the end of the
/// same turn, having read it at the head of its tool result.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_message_sent_during_a_turn_enters_the_turn() {
    let d = tempfile::tempdir().unwrap();
    let (_repository, manifest) = depot_inscrit(d.path(), true);
    let sleeping = Arc::new(AtomicBool::new(false));
    let host = Arc::new(hote(d.path(), slow_reader(sleeping.clone(), std::time::Duration::from_secs(2))));
    let h = host.hosted(&manifest).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let id = h.id.clone();
    let first = {
        let (host, id) = (host.clone(), id.clone());
        std::thread::spawn(move || host.chat_turn(&id, "lis le README du dépôt"))
    };
    wait_for(&sleeping, "the first turn's model call");
    let running = host.run_du_chat_en_cours(&id).expect("a turn runs while the model sleeps");
    let second = host.chat_turn_by(&id, NOTE, &json!({ "kind": "human", "client": "ide" }));
    let first = first.join().unwrap();
    let second = second.unwrap_or_else(|refusal| panic!("a message sent during a turn is accepted, not refused: {refusal}"));
    assert_eq!(second["received_during_turn"], json!(true), "{second}");
    assert_eq!(second["run"], json!(running), "the note is of the running turn: {second}");
    let first = first.expect("the first turn");
    let text = first["text"].as_str().unwrap_or_default();
    assert!(text.contains(NOTE) && text.contains("en route"), "the model read the note in a tool result of the same turn: {first}");
    let state = host.chat_state(&id).unwrap();
    let turns = state["turns"].as_array().cloned().unwrap_or_default();
    let marked: Vec<&Value> = turns
        .iter()
        .flat_map(|t| t["metadata"]["received_during_turn"].as_array().cloned().unwrap_or_default().into_iter().map(move |n| (t, n)))
        .filter(|(_, n)| n["text"] == json!(NOTE))
        .map(|(t, _)| t)
        .collect();
    assert_eq!(marked.len(), 1, "the note is in the thread once, marked received during the turn: {turns:?}");
    assert_eq!(marked[0]["role"], json!("tool"), "where the model read it: {turns:?}");
    // Delivered in the turn: no second turn carries it again.
    let users: Vec<&Value> = turns.iter().filter(|t| t["role"] == "user").collect();
    assert_eq!(users.len(), 1, "no follow-up turn for a delivered note: {turns:?}");
}

/// **A note the turn could not deliver goes as the next turn's message** (28 November 2026): sent while the model
/// writes its last answer (no tool follows), it is not lost: once the turn ends, it starts the next turn, its user turn
/// marked received during the turn, and the model quotes it.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_note_left_at_the_end_of_a_turn_starts_the_next_one() {
    let d = tempfile::tempdir().unwrap();
    let (_repository, manifest) = depot_inscrit(d.path(), true);
    let answering = Arc::new(AtomicBool::new(false));
    // A model that answers in text after a nap, no tool at all.
    let model: ChatModel = {
        let answering = answering.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let answering = answering.clone();
            Ok(Arc::new(CallbackLlm::new("faux-modele", 8192, move |turns, opts, sink| {
                if turns.last().is_some_and(|t| t.role == "user" && t.content.contains(NOTE)) {
                    return MockLlm::new(&format!("Consigne reçue au tour suivant : « {NOTE} ».")).generate(turns, opts, sink);
                }
                answering.store(true, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_secs(2));
                MockLlm::new("Bonjour.").generate(turns, opts, sink)
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model));
    host.se_connaitre();
    let h = host.hosted(&manifest).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let id = h.id.clone();
    let first = {
        let (host, id) = (host.clone(), id.clone());
        std::thread::spawn(move || host.chat_turn(&id, "bonjour"))
    };
    wait_for(&answering, "the first turn's model call");
    let second = host.chat_turn_by(&id, NOTE, &json!({ "kind": "human", "client": "ide" }));
    let first = first.join().unwrap().expect("the first turn");
    let second = second.unwrap_or_else(|refusal| panic!("a message sent during a turn is accepted, not refused: {refusal}"));
    assert_eq!(second["received_during_turn"], json!(true), "{second}");
    // The next turn starts by itself once the first ends: wait for its end (two user turns, no turn running).
    let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let turns = loop {
        let state = host.chat_state(&id).unwrap();
        let turns = state["turns"].as_array().cloned().unwrap_or_default();
        let users = turns.iter().filter(|t| t["role"] == "user").count();
        if (users >= 2 && host.run_du_chat_en_cours(&id).is_none()) || std::time::Instant::now() > until {
            break turns;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let users: Vec<&Value> = turns.iter().filter(|t| t["role"] == "user").collect();
    assert_eq!(users.len(), 2, "the note started the next turn: {turns:?} (first: {first})");
    assert_eq!(users[1]["content"], json!(NOTE), "{turns:?}");
    assert_eq!(users[1]["metadata"]["received_during_turn"], json!(true), "the next turn's message says it came during a turn: {turns:?}");
    let last = turns.iter().rev().find(|t| t["role"] == "assistant").map(|t| t["content"].clone()).unwrap_or_default();
    assert!(last.as_str().unwrap_or_default().contains(NOTE), "the model read it: {turns:?}");
    assert!(
        !turns.iter().any(|t| t["role"] == "tool" && t["content"].as_str().unwrap_or_default().contains(NOTE)),
        "never twice: {turns:?}"
    );
}

/// **In the IDE, the send button stays on during a turn** (28 November 2026): while the fake model sleeps in its
/// first call, the page sends a second sentence; it shows at once in the thread marked « reçu pendant le tour », the
/// host takes it as a note of the turn, and the model quotes it at the end.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : une étape de passe à part, --features navigateur (scripts/temoins_navigateur.sh)")]
fn the_page_sends_during_a_turn_and_the_model_reads_it() {
    use crate::daemon_host::navigateur::{navigateur_ou_saut, Navigateur};
    let Some(binary) = navigateur_ou_saut("the_page_sends_during_a_turn_and_the_model_reads_it") else {
        return;
    };
    let d = tempfile::tempdir().unwrap();
    let (_repository, manifest) = depot_inscrit(d.path(), true);
    let sleeping = Arc::new(AtomicBool::new(false));
    let host = Arc::new(hote(d.path(), slow_reader(sleeping.clone(), std::time::Duration::from_secs(4))));
    host.se_connaitre();
    let h = host.hosted(&manifest).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let id = h.id.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tiny_http::Server::from_listener(listener, None).unwrap();
    let served = host.clone();
    let thread = std::thread::spawn(move || crate::daemon_host::DaemonServer::new(server, 4).serve(served, Some("j".to_string())));
    let mut n = Navigateur::lancer(&binary);
    n.aller(&format!("{base}/ide/entrer?jeton=j"), "location.pathname.startsWith('/ide/')");
    n.aller(&format!("{base}/ide/code/{id}/chat"), "document.querySelector('#send') && !document.querySelector('#send').disabled");
    n.evaluer("document.querySelector('#message').value = 'Lis le README du dépôt.'; document.querySelector('#send').click()");
    wait_for(&sleeping, "the first turn's model call");
    assert!(host.run_du_chat_en_cours(&id).is_some(), "a turn runs while the model sleeps");
    let disabled = n.evaluer("document.querySelector('#send').disabled");
    assert_eq!(disabled, json!(false), "the send button stays on during a turn");
    n.evaluer(&format!("document.querySelector('#message').value = '{NOTE}'; document.querySelector('#send').click()"));
    let shown = format!(
        "[...document.querySelectorAll('article.message.user')].some((a) => a.innerText.includes('{NOTE}') && a.innerText.includes('reçu pendant le tour'))"
    );
    n.attendre(&shown, "the second sentence in the thread, marked received during the turn");
    n.attendre(
        &format!("[...document.querySelectorAll('article.message.assistant')].some((a) => !a.classList.contains('en-cours') && a.innerText.includes('en route') && a.innerText.includes('{NOTE}'))"),
        "the model quotes the sentence at the end of the turn",
    );
    // The end of the turn rereads the thread: the sentence is still there, marked.
    n.attendre(&shown, "the sentence, marked, after the end of the turn");
    drop(n);
    let (status, _) = crate::daemon_host::tests::demander("POST", &format!("{base}/arret"), Some("j"));
    assert_eq!(status, 200);
    thread.join().unwrap();
}

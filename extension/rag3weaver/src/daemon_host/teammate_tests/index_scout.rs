//! **The scout of the index: at the start of a turn, the searches done for the agent** (ticket of 24 Nov 2026, « l'éclaireur
//! de l'index »). In a folder whose index is ready, the first step of a turn opened by a request about the code gets a
//! block of what the index found for it: a structured call scores what the agent already knows and gives the searches,
//! the searches run on the index, a second structured call relates the results to the request, and a line whose
//! `path:line` is not in the results is dropped. A greeting, or a request the agent already knows, runs no search and
//! adds nothing. The score and the decision are in the turn's trace (a host line `scout` of the mailbox). The scout's
//! two calls go to the model the user's table names (`at_turn_start.json`), here the local fake server of
//! `tests/common/fake_sse.rs`. By the host's own path. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[path = "../../../tests/common/fake_sse.rs"]
mod fake_sse;

/// A reply of the fake server: one chunk whose content is `json`, then the end.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
fn replies(json: &Value) -> fake_sse::Reply {
    let content = json!({ "choices": [{ "index": 0, "delta": { "role": "assistant", "content": json.to_string() }, "finish_reason": null }] });
    let stop = json!({ "choices": [{ "index": 0, "delta": {}, "finish_reason": "stop" }] });
    fake_sse::Reply::Sse { frames: vec![content.to_string(), stop.to_string()] }
}

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn a_turn_on_the_code_starts_with_what_the_index_found_and_a_greeting_with_nothing() {
    let d = tempfile::tempdir().unwrap();
    let server = fake_sse::FakeServer::start_sequence(vec![
        // The request on the code: the agent knows little, three searches and a name to follow.
        replies(&json!({ "knows": 0.1, "queries": ["bonjour", "the greeting function", "where bonjour is defined"], "usages": "bonjour" })),
        // The synthesis: one line from the results, one invented.
        replies(&json!({
            "orientation": "Le petit dépôt tient sa seule fonction, bonjour, dans src/lib.rs.",
            "relevant": [
                { "n": 1, "at": "src/lib.rs:1", "symbol": "bonjour", "why": "la définition demandée" },
                { "n": 9, "at": "src/fantome.rs:42", "symbol": "fantome", "why": "une ligne qui n'est dans aucun résultat" }
            ]
        })),
        // The greeting: nothing to search.
        replies(&json!({ "knows": 0.95, "queries": [], "usages": "" })),
    ]);
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut c: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    // The index in full text alone: this build has no token counter for the template's embedding model, and the scout
    // reads the text index; the scout's model, by its name.
    c.as_object_mut().unwrap().remove("embeddings");
    c["models"] = json!({});
    c["models"]["llm"] = json!([
        { "name": "equipier", "access": "compatible", "provider": "openai", "model": "faux-equipier", "address": "http://127.0.0.1:9/v1", "default": true },
        { "name": "eclaireur", "access": "compatible", "provider": "openai", "model": "faux-eclaireur", "address": server.url }
    ]);
    std::fs::write(&manifeste, serde_json::to_string_pretty(&c).unwrap()).unwrap();
    let declarations = manifeste.ancestors().find(|p| p.join(".git").exists()).unwrap().to_path_buf();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "le modèle de l'éclaireur"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&declarations).status().unwrap().success(), "git {args:?}");
    }
    // The user's table, read at each turn: blocking, the threshold, the bound, and the model of the two calls.
    let graphes = d.path().join("code-roots/config/templates/teammate/graphs");
    std::fs::create_dir_all(&graphes).unwrap();
    std::fs::write(
        graphes.join("at_turn_start.json"),
        json!({ "scout": { "mode": "blocking", "knows_threshold": 0.5, "max_ms": 120000, "models": ["eclaireur"] } }).to_string(),
    )
    .unwrap();
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
    let h = host.hosted(&manifeste).unwrap();
    // The folder's index, done before the turns, to its last line.
    let index = host.with_backend(&h, |b| b.call("index", json!({ "confirm": true }))).unwrap().expect("the index starts");
    let texte = index["result"].as_str().or_else(|| index.as_str()).unwrap_or_default().to_string();
    let journal = std::path::PathBuf::from(texte.lines().find_map(|l| l.strip_prefix("journal : ")).unwrap_or_else(|| panic!("the index says its journal: {index}")).trim());
    let fin = Instant::now() + Duration::from_secs(600);
    loop {
        let derniere = std::fs::read_to_string(&journal).ok().and_then(|j| j.lines().last().map(str::to_string)).unwrap_or_default();
        if derniere.starts_with(crate::dataflow::index_nodes::DONE_LINE) {
            break;
        }
        assert!(!derniere.starts_with(crate::dataflow::index_nodes::FAILED_PREFIX), "the index failed: {derniere}");
        assert!(Instant::now() < fin, "the index did not end: {derniere}");
        std::thread::sleep(Duration::from_millis(200));
    }
    let tour = |tache: &str, demande: &str| {
        host.post_to_teammate(&h.id, tache, "lucie", "request", None, demande, &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
        let files = host.teammate_files(&h.id, tache).unwrap();
        let fin = Instant::now() + Duration::from_secs(120);
        while !ended(&files.mailbox()) && Instant::now() < fin {
            std::thread::sleep(Duration::from_millis(100));
        }
        files.mailbox()
    };
    let code = tour("code", "Où est définie la fonction bonjour, et qui l'appelle ?");
    let salut = tour("salut", "bonjour !");
    let premiers = premiers.lock().unwrap().clone();
    let bodies: Vec<String> = server.request.try_iter().collect();
    eprintln!("first steps: {premiers:#?}\nscout calls: {}", bodies.len());
    let premier = premiers.first().cloned().unwrap_or_default();
    assert!(premier.contains("What the code index found for this request"), "the first step of a turn on the code gets the scout's block: {premiers:#?}");
    assert!(premier.contains("src/lib.rs:1") && premier.contains("Le petit dépôt tient sa seule fonction"), "with its orientation and its checked path:line: {premier}");
    assert!(!premier.contains("src/fantome.rs:42"), "a path:line that no result holds is dropped: {premier}");
    assert!(premiers.get(1).is_some_and(|m| !m.contains("What the code index found")), "a greeting gets no block: {premiers:#?}");
    assert_eq!(bodies.len(), 3, "two calls for the request on the code, one for the greeting, none more: {bodies:#?}");
    assert!(bodies[0].contains("Où est définie la fonction bonjour") && bodies[0].contains("src"), "the first call reads the request and the project's plan: {}", bodies[0]);
    assert!(bodies[1].contains("src/lib.rs:1"), "the synthesis reads the numbered results: {}", bodies[1]);
    assert!(bodies[2].contains("bonjour !"), "{}", bodies[2]);
    let trace = |boite: &[Value]| boite.iter().find(|l| l["from"] == "host" && l["kind"] == "scout").cloned();
    let t = trace(&code).unwrap_or_else(|| panic!("the turn's trace holds the scout's score and decision: {code:#?}"));
    assert_eq!((t["knows"].as_f64(), t["decision"].as_str()), (Some(0.1), Some("search")), "{t}");
    assert_eq!(t["dropped"], json!(["src/fantome.rs:42"]), "{t}");
    let t = trace(&salut).unwrap_or_else(|| panic!("{salut:#?}"));
    assert_eq!((t["knows"].as_f64(), t["decision"].as_str()), (Some(0.95), Some("known")), "{t}");
}

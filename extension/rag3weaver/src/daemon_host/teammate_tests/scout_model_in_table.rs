//! **The scout's light model is declared by its table, and the folder's manifest wins at an equal name** (28 Nov 2026:
//! the shipped table named `gemini-3.1-flash-lite`, which no manifest declares, so that the scout's two calls went to
//! the teammate's own model; declaring it in the code template would make it every new folder's teammate). An entry
//! of `scout.models` is a name, looked up in the manifest's `models.llm`, or a whole model source, used as it is
//! unless the manifest declares its name. By the host's own path, a thanks in an indexed folder (one scout call).
//! A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[path = "../../../tests/common/fake_sse.rs"]
mod fake_sse;

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
fn knows_it() -> fake_sse::Reply {
    let content = json!({ "choices": [{ "index": 0, "delta": { "role": "assistant", "content": json!({ "knows": 0.95, "queries": [], "usages": "" }).to_string() }, "finish_reason": null }] });
    let stop = json!({ "choices": [{ "index": 0, "delta": {}, "finish_reason": "stop" }] });
    fake_sse::Reply::Sse { frames: vec![content.to_string(), stop.to_string()] }
}

/// One thanks in an indexed folder whose manifest declares `llm`, the user's table naming `table_models`.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
fn a_thanks(llm: Value, table_models: Value) {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut c: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    c["embeddings"] = json!({ "model": "faux-embarqueur", "dimensions": 8 });
    c["models"] = json!({ "llm": llm });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&c).unwrap()).unwrap();
    let declarations = manifeste.ancestors().find(|p| p.join(".git").exists()).unwrap().to_path_buf();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "les modèles"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&declarations).status().unwrap().success(), "git {args:?}");
    }
    let graphes = d.path().join("code-roots/config/templates/teammate/graphs");
    std::fs::create_dir_all(&graphes).unwrap();
    std::fs::write(graphes.join("at_turn_start.json"), json!({ "scout": { "mode": "blocking", "knows_threshold": 0.5, "max_ms": 120000, "models": table_models } }).to_string()).unwrap();
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
            answer(turns, opts, sink, "Avec plaisir.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "répondu", "remaining": "" }))]))
        })))
    });
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
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
    host.post_to_teammate(&h.id, "code", "lucie", "request", None, "Merci, c'est tout !", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let files = host.teammate_files(&h.id, "code").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !files.mailbox().iter().any(|l| l["kind"] == "ended") && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn the_scout_calls_the_model_its_table_declares() {
    let table = fake_sse::FakeServer::start_sequence(vec![knows_it()]);
    a_thanks(
        json!([{ "name": "equipier", "access": "compatible", "provider": "openai", "model": "faux-equipier", "address": "http://127.0.0.1:9/v1", "default": true }]),
        json!([{ "name": "leger", "access": "compatible", "provider": "openai", "model": "faux-leger", "address": table.url }]),
    );
    let calls: Vec<String> = table.request.try_iter().collect();
    assert_eq!(calls.len(), 1, "the scout's call goes to the model its table declares, the manifest declaring one model only: {calls:#?}");
}

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn the_manifests_model_wins_at_an_equal_name() {
    let table = fake_sse::FakeServer::start_sequence(vec![knows_it()]);
    let own = fake_sse::FakeServer::start_sequence(vec![knows_it()]);
    a_thanks(
        json!([
            { "name": "equipier", "access": "compatible", "provider": "openai", "model": "faux-equipier", "address": "http://127.0.0.1:9/v1", "default": true },
            { "name": "leger", "access": "compatible", "provider": "openai", "model": "faux-leger-a-elle", "address": own.url }
        ]),
        json!([{ "name": "leger", "access": "compatible", "provider": "openai", "model": "faux-leger", "address": table.url }]),
    );
    assert_eq!(own.request.try_iter().count(), 1, "the manifest's own `leger` is called");
    assert_eq!(table.request.try_iter().count(), 0, "not the table's");
}

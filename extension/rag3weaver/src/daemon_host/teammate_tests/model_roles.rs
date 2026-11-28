//! **A template names a role, the user's configuration says which model plays it** (Lucie, 28 Nov 2026: role aliases,
//! « large → gemini-3.8-flash, small → qwen… », so that the templates speak in technical identifiers without imposing
//! a stack of providers). The shipped `templates/models/roles.json` gives each role an ordered list of model names,
//! the first one declared winning; `templates/models/catalog.json` holds whole model sources by name; the user's own
//! files of the same name under her configuration take their place, key by key; the folder's manifest wins at an
//! equal name. A role with no model declared falls back to `large`, the teammate's own, said in the task's mailbox.
//! By the host's own path: the scout's table names `small`, a thanks in an indexed folder (one scout call). A module
//! of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[path = "../../../tests/common/fake_sse.rs"]
pub(super) mod fake_sse;

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
pub(super) fn knows_it() -> fake_sse::Reply {
    let content = json!({ "choices": [{ "index": 0, "delta": { "role": "assistant", "content": json!({ "knows": 0.95, "queries": [], "usages": "" }).to_string() }, "finish_reason": null }] });
    let stop = json!({ "choices": [{ "index": 0, "delta": {}, "finish_reason": "stop" }] });
    fake_sse::Reply::Sse { frames: vec![content.to_string(), stop.to_string()] }
}

/// One thanks in an indexed folder whose manifest declares `llm`, the user's roles and catalog being `roles` and
/// `catalog`; the mailbox of the task.
#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
pub(super) fn a_thanks(llm: Value, roles: Value, catalog: Value, manifest_roles: Value) -> Vec<Value> {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut c: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    c["embeddings"] = json!({ "model": "faux-embarqueur", "dimensions": 8 });
    c["models"] = json!({ "llm": llm });
    if !manifest_roles.is_null() {
        c["roles"] = manifest_roles;
    }
    std::fs::write(&manifeste, serde_json::to_string_pretty(&c).unwrap()).unwrap();
    let declarations = manifeste.ancestors().find(|p| p.join(".git").exists()).unwrap().to_path_buf();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "les modèles"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&declarations).status().unwrap().success(), "git {args:?}");
    }
    let graphes = d.path().join("code-roots/config/templates/teammate/graphs");
    std::fs::create_dir_all(&graphes).unwrap();
    std::fs::write(graphes.join("at_turn_start.json"), json!({ "scout": { "mode": "blocking", "knows_threshold": 0.5, "max_ms": 120000, "models": ["small"] } }).to_string()).unwrap();
    let modeles = d.path().join("code-roots/config/templates/models");
    std::fs::create_dir_all(&modeles).unwrap();
    std::fs::write(modeles.join("roles.json"), json!({ "roles": roles }).to_string()).unwrap();
    std::fs::write(modeles.join("catalog.json"), json!({ "models": catalog }).to_string()).unwrap();
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
    files.mailbox()
}

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
pub(super) fn equipier() -> Value {
    json!({ "name": "equipier", "access": "compatible", "provider": "openai", "model": "faux-equipier", "address": "http://127.0.0.1:9/v1", "default": true })
}

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn the_scout_calls_the_model_the_users_configuration_gives_its_role() {
    let x = fake_sse::FakeServer::start_sequence(vec![knows_it()]);
    a_thanks(
        json!([equipier()]),
        json!({ "small": ["x"] }),
        json!([{ "name": "x", "access": "compatible", "provider": "openai", "model": "faux-x", "address": x.url }]),
        Value::Null,
    );
    let calls: Vec<String> = x.request.try_iter().collect();
    assert_eq!(calls.len(), 1, "the role small is played by x, from the user's catalog: {calls:#?}");
}

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn the_manifests_model_wins_at_an_equal_name() {
    let catalog = fake_sse::FakeServer::start_sequence(vec![knows_it()]);
    let own = fake_sse::FakeServer::start_sequence(vec![knows_it()]);
    a_thanks(
        json!([equipier(), { "name": "x", "access": "compatible", "provider": "openai", "model": "faux-x-a-elle", "address": own.url }]),
        json!({ "small": ["x"] }),
        json!([{ "name": "x", "access": "compatible", "provider": "openai", "model": "faux-x", "address": catalog.url }]),
        Value::Null,
    );
    assert_eq!(own.request.try_iter().count(), 1, "the manifest's own x is called");
    assert_eq!(catalog.request.try_iter().count(), 0, "not the catalog's");
}

#[cfg(all(feature = "code", feature = "rag3db-native", feature = "openai-llm", target_os = "linux"))]
#[test]
fn a_role_with_no_model_falls_back_to_large_and_says_so() {
    let b = a_thanks(json!([equipier()]), json!({ "small": ["absent"] }), json!([]), Value::Null);
    let said = b.iter().find(|l| l["from"] == "host" && l["kind"] == "role").unwrap_or_else(|| panic!("the fallback is said: {b:#?}"));
    let text = said["text"].as_str().unwrap_or_default();
    assert!(text.contains("small") && text.contains("large"), "{said}");
}

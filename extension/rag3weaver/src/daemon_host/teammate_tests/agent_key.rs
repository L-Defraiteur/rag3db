//! **The agent's key** (Lucie, 25 October 2026; decided with c7 and archi, 21 November 2026): an agent is its stable
//! key `agent:<key>`, and a teammate's task is a context it runs in. The task registry holds the key
//! (`Tache.agent_key`): declared by `teammate.agent` to continue an existing agent, else struck when the task opens
//! (`agent:<task>-<6 hex>`). It survives the host rereading its registry, and it signs what the host writes for the
//! agent: its identity in `compose_restart`. A file of its own (c7's rule, 21 November 2026).

use super::*;

fn registre(d: &std::path::Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(d.join("taches/registre.json")).unwrap()).unwrap()
}

fn cle_au_registre(d: &std::path::Path, nom: &str) -> Value {
    registre(d)["ouvertes"].as_array().into_iter().flatten().find(|t| t["nom"] == nom).map(|t| t["agent_key"].clone()).unwrap_or(Value::Null)
}

fn sans_modele() -> ChatModel {
    Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> { Err("no model in this witness".into()) })
}

/// A task opened without `teammate.agent` gets a key struck by the host, `agent:<task>-<6 hex>`, written in the
/// registry; a host that rereads the registry (a restart) finds the same key.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_task_gets_an_agent_key_that_survives_the_registry_being_reread() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let host = hote(d.path(), sans_modele());
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the parent opens");
    let t = host.ouvrir_une_tache(&h.id, "doc").expect("the task opens");
    let cle = t.agent_key.clone().expect("the task has a key");
    let suffixe = cle.strip_prefix("agent:doc-").unwrap_or_default();
    assert!(suffixe.len() == 6 && suffixe.chars().all(|c| c.is_ascii_hexdigit()), "agent:<task>-<6 hex>: {cle}");
    assert_eq!(cle_au_registre(d.path(), "doc"), json!(cle), "the registry holds it: {}", registre(d.path()));
    drop(host);
    let host = hote(d.path(), sans_modele());
    host.relire_les_taches();
    assert_eq!(cle_au_registre(d.path(), "doc"), json!(cle), "the reread registry keeps it: {}", registre(d.path()));
    let autre = host.ouvrir_une_tache(&h.id, "tests").expect("a second task opens");
    assert_ne!(autre.agent_key.as_deref(), Some(cle.as_str()), "another task, another agent");
}

/// `teammate.agent` declared: the task continues that agent, with its key (the reviewer who takes up its work on the
/// next lot).
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_task_that_declares_teammate_agent_continues_that_agent() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"] = json!({ "agent": "agent:relecteur" });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "the reviewer"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let host = hote(d.path(), sans_modele());
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the parent opens");
    let t = host.ouvrir_une_tache(&h.id, "lot-2").expect("the task opens");
    assert_eq!(t.agent_key.as_deref(), Some("agent:relecteur"));
    assert_eq!(cle_au_registre(d.path(), "lot-2"), json!("agent:relecteur"));
}

/// A memory that records what `compose_restart` is asked, and files each slice under a Source.
struct Restarts(Mutex<Vec<Value>>);

impl crate::daemon_host::rangement::Rangeur for Restarts {
    fn put_turn(&self, _memoire: &std::path::Path, _line: &Value) -> Result<Value, String> {
        Ok(json!({ "written": true }))
    }
    fn archive_slice(&self, _memoire: &std::path::Path, args: &Value) -> Result<Value, String> {
        Ok(json!({ "report": { "ok": true, "errors": [] }, "archived_through_turn": args["to_turn"], "source": format!("sources/slice-{}.md", args["to_turn"]) }))
    }
    fn compose_restart(&self, _memoire: &std::path::Path, args: &Value) -> Result<Value, String> {
        self.0.lock().unwrap().push(args.clone());
        Ok(json!({ "text": "restart", "tokens": 1 }))
    }
}

/// The compaction of a teammate's session restarts it as its agent: `compose_restart` gets `identity = agent:<key>`.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_compaction_of_a_teammate_restarts_it_under_its_agent_key() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["bilans"] = json!({ "memoire": "memoire/backend.json" });
    m["teammate"] = json!({ "reply_wait_s": 2 });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "the memory of the session"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("keyed", 8192, |turns, opts, sink| {
            answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "said hello", "remaining": "" }))]))
        })))
    });
    let memory = Arc::new(Restarts(Mutex::new(Vec::new())));
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_rangeur(memory.clone()));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the parent opens");
    let t = host.ouvrir_une_tache(&h.id, "doc").expect("the task opens");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append("lucie", "request", None, "Say hello").unwrap();
    let _ = host.run_teammate(&h.id, "doc");
    host.compacter_le_chat(&t.id).expect("the teammate's session compacts");
    let asked = memory.0.lock().unwrap().clone();
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert_eq!(asked[0]["identity"], json!(t.agent_key.clone().expect("the task has a key")), "{asked:?}");
}

//! **Background commands that tell the agent when they end** (Lucie, 21 November 2026, for her trial of the mode auto:
//! « s'il oublie, l'installation finit dans le vide »). A command the teammate runs in the background outlives its
//! step; when it ends, the host wakes the teammate and its next step starts with the end said (handle, code, last
//! lines), without the teammate asking.
use super::super::chat::ChatModel;
use super::super::chat_qui_code::{depot_inscrit, hote};
use crate::llm::{CallbackLlm, Llm, Turn};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The user text of the turn (the step's boundary message).
fn boundary(turns: &[Turn]) -> String {
    turns.iter().rev().find(|t| t.role == "user").map(|t| t.content.clone()).unwrap_or_default()
}

/// A background command ends after its step: the teammate is woken, and its next step is told the end with its
/// last line, without polling.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn a_background_command_that_ends_is_told_at_the_next_step() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    super::en_approbation(&manifeste);
    std::fs::write(depot.join("fond.sh"), "sleep 2\necho fini-de-fond\n").unwrap();
    for args in [&["add", "fond.sh"][..], &["commit", "-q", "-m", "fond"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&depot).status().unwrap().success(), "git {args:?}");
    }
    let told = Arc::new(Mutex::new(None::<String>));
    let launched = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let results = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let (told, launched, results) = (told.clone(), launched.clone(), results.clone());
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (told, launched, results) = (told.clone(), launched.clone(), results.clone());
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let user = boundary(turns);
                if user.contains("fini-de-fond") {
                    *told.lock().unwrap() = Some(user);
                    return super::answer(turns, opts, sink, "", super::calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fin lue", "remaining": "" }))]));
                }
                // The command is asked for until it has run once (the gate may ask the supervisor first).
                results.lock().unwrap().extend(super::tool_results(turns));
                let ran = super::tool_results(turns).iter().any(|r| r.contains("fond.sh") && !r.contains(crate::dataflow::run_nodes::EN_ATTENTE));
                if ran {
                    launched.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                if !launched.load(std::sync::atomic::Ordering::SeqCst) && super::tool_results(turns).is_empty() {
                    return super::answer(turns, opts, sink, "", super::calls(&[("run_command", json!({ "command": "sh fond.sh", "background": true }))]));
                }
                // Launched: the step ends; nothing asks after the command.
                super::answer(turns, opts, sink, "", super::calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "lancée en fond", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Lance fond.sh en fond, et dis-moi quand il a fini.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let end = Instant::now() + Duration::from_secs(90);
    let mut answered = std::collections::HashSet::new();
    while told.lock().unwrap().is_none() {
        // The gate asks for the command: the supervisor grants it.
        for l in files.mailbox() {
            if l["kind"] == "approval" && answered.insert(l["id"].as_str().unwrap_or_default().to_string()) {
                host.post_to_teammate(&h.id, "doc", "c7", "answer", None, "oui", &json!({ "reply_to": l["id"], "approved": true, "by": { "kind": "human", "client": "tty" } })).expect("le oui");
            }
        }
        assert!(Instant::now() < end, "in 90 s, no step was told the end of the background command (launched: {}); tool results: {:#?}\nmailbox: {:#?}", launched.load(std::sync::atomic::Ordering::SeqCst), results.lock().unwrap(), files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(launched.load(std::sync::atomic::Ordering::SeqCst), "the command ran");
    let said = told.lock().unwrap().clone().unwrap_or_default();
    assert!(said.contains("a fini") && said.contains("code 0"), "the end is said with its code: {said}");
}

/// **The bounds of background commands** (Lucie, 22 November 2026): with `teammate.background {max: 1,
/// max_timeout_s: 5}`, a second command while one runs is refused naming the bound, a `timeout_s` above the cap is
/// refused naming it, « never » (`timeout_s: 0`) is refused (only a declared server may), and the command that runs
/// with `timeout_s: 3` is stopped at its time: the teammate is told « arrêtée … (délai atteint) » with its last lines.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn background_commands_are_bounded_in_number_and_time() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    super::en_approbation(&manifeste);
    std::fs::write(depot.join("dort.sh"), "echo je-dors\nsleep 30\n").unwrap();
    for args in [&["add", "dort.sh"][..], &["commit", "-q", "-m", "dort"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&depot).status().unwrap().success(), "git {args:?}");
    }
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"] = json!({ "background": { "max": 1, "max_timeout_s": 5 }, "reply_wait_s": 2 });
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let declarations = manifeste.parent().unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "bornes"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(declarations).status().unwrap().success(), "git {args:?}");
    }
    let told = Arc::new(Mutex::new(None::<String>));
    let launched = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let results = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let (told, launched, results) = (told.clone(), launched.clone(), results.clone());
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let (told, launched, results) = (told.clone(), launched.clone(), results.clone());
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let user = boundary(turns);
                if user.contains("délai atteint") {
                    *told.lock().unwrap() = Some(user);
                    return super::answer(turns, opts, sink, "", super::calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fin lue", "remaining": "" }))]));
                }
                let got = super::tool_results(turns);
                if got.iter().any(|r| r.contains("**En fond**")) {
                    launched.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                results.lock().unwrap().extend(got.iter().cloned());
                if !launched.load(std::sync::atomic::Ordering::SeqCst) && got.is_empty() {
                    let fond = |extra: Value| {
                        let mut a = json!({ "command": "sh dort.sh", "background": true });
                        for (k, v) in extra.as_object().cloned().unwrap_or_default() {
                            a[k] = v;
                        }
                        ("run_command", a)
                    };
                    return super::answer(turns, opts, sink, "", super::calls(&[
                        fond(json!({ "timeout_s": 3 })),
                        fond(json!({})),
                        fond(json!({ "timeout_s": 10 })),
                        fond(json!({ "timeout_s": 0 })),
                    ]));
                }
                super::answer(turns, opts, sink, "", super::calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "lancées", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    host.post_to_teammate(&h.id, "doc", "c7", "request", None, "Lance dort.sh en fond, plusieurs fois.", &Value::Null).expect("la request");
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let end = Instant::now() + Duration::from_secs(90);
    let mut answered = std::collections::HashSet::new();
    while told.lock().unwrap().is_none() {
        for l in files.mailbox() {
            if l["kind"] == "approval" && answered.insert(l["id"].as_str().unwrap_or_default().to_string()) {
                // A loaded machine, made deterministic: the yes comes 4 s late (the first command's own time is 3 s).
                std::thread::sleep(Duration::from_secs(4));
                host.post_to_teammate(&h.id, "doc", "c7", "answer", None, "oui", &json!({ "reply_to": l["id"], "approved": true, "by": { "kind": "human", "client": "tty" } })).expect("le oui");
            }
        }
        assert!(Instant::now() < end, "in 90 s, no step was told the stop at the time (launched: {}); tool results: {:#?}", launched.load(std::sync::atomic::Ordering::SeqCst), results.lock().unwrap());
        std::thread::sleep(Duration::from_millis(100));
    }
    let r = results.lock().unwrap().clone();
    assert!(r.iter().any(|x| x.contains("teammate.background.max") && !x.contains("max_timeout_s")), "a second one while one runs is refused, naming the bound: {r:#?}");
    assert!(r.iter().any(|x| x.contains("teammate.background.max_timeout_s")), "a time above the cap is refused, naming it: {r:#?}");
    assert!(r.iter().any(|x| x.contains("jamais")), "« never » is refused: {r:#?}");
    let said = told.lock().unwrap().clone().unwrap_or_default();
    assert!(said.contains("arrêtée après") && said.contains("délai atteint") && said.contains("je-dors"), "stopped at its time, said with its last lines: {said}");
}

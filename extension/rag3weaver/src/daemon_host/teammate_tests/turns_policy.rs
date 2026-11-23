//! **The memory of the conversation is a declared policy** (Lucie, 23 Nov 2026, 08 h 28 and 08 h 34: « il faut que
//! ce soit configurable et qu'on puisse les enlever » ; « les tours, c'est une autre politique, si on trim les vieux
//! ou autre, désactivable entièrement »). `teammate.conversation` says whether a turn receives the turns before it
//! (`earlier_turns`) and how old turns are cut (`trim`, which can be switched off: then they stay whole); a launch
//! overrides it with the request's `conversation`. Apart from it, the protection bound (`teammate.input_max_chars`)
//! cuts a request too long, and never goes away. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

fn listening(heard: Arc<Mutex<Vec<String>>>) -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let heard = heard.clone();
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            heard.lock().unwrap().push(turns.iter().map(|t| t.content.clone()).collect::<Vec<_>>().join("\n"));
            answer(turns, opts, sink, "", calls(&[("report", json!({ "text": "noté" })), ("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
        })))
    })
}

/// Three turns of one thread, the third with `conversation` (none: the declared policy); what the third turn heard.
fn fil(teammate: Value, troisieme: Value, textes: [&str; 3]) -> (String, Vec<Value>) {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    if !teammate.is_null() {
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
        m["teammate"] = teammate;
        std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
        let decl = manifeste.ancestors().find(|p| p.join(".git").exists()).unwrap().to_path_buf();
        for args in [&["add", "-A"][..], &["commit", "-q", "-m", "la politique"]] {
            assert!(std::process::Command::new("git").args(args).current_dir(&decl).status().unwrap().success());
        }
    }
    let heard = Arc::new(Mutex::new(Vec::new()));
    let host = Arc::new(hote(d.path(), listening(heard.clone())).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let mut avant = 0;
    for (i, texte) in textes.iter().enumerate() {
        let mut extra = json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" });
        if i == 2 && !troisieme.is_null() {
            extra["conversation"] = troisieme.clone();
        }
        host.post_to_teammate(&h.id, "doc", "lucie", "request", None, texte, &extra).unwrap();
        let files = host.teammate_files(&h.id, "doc").unwrap();
        let fin = Instant::now() + Duration::from_secs(120);
        while files.mailbox().iter().filter(|l| l["kind"] == "ended").count() <= i {
            assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
            std::thread::sleep(Duration::from_millis(100));
        }
        if i == 1 {
            avant = heard.lock().unwrap().len();
        }
    }
    let boite = host.teammate_files(&h.id, "doc").unwrap().mailbox();
    let h = heard.lock().unwrap();
    (h[avant..].join("\n"), boite)
}

/// **Only the context is shaped** (Lucie, 23 Nov 2026: « ça n'arrive qu'à l'agent, le trim, pas dans les données »):
/// the mailbox gives the first request back whole, byte for byte.
fn premier_entier(boite: &[Value], premier: &str) {
    let l = boite.iter().find(|l| l["kind"] == "request" && l["from"] == "lucie").expect("the first request");
    assert_eq!(l["text"].as_str(), Some(premier), "the mailbox keeps the first turn whole");
}

const UN: &str = "Premier tour : alpha-un.";
const DEUX: &str = "Second tour : beta-deux.";
const TROIS: &str = "Troisième tour : et donc ?";

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_declared_default_keeps_the_earlier_turns() {
    let (entendu, _) = fil(Value::Null, Value::Null, [UN, DEUX, TROIS]);
    assert!(entendu.contains("alpha-un") && entendu.contains("beta-deux"), "{entendu}");
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_trim_of_two_keeps_only_the_last_two_lines() {
    let (entendu, boite) = fil(json!({ "conversation": { "trim": { "max": 2 } } }), Value::Null, [UN, DEUX, TROIS]);
    assert!(entendu.contains("beta-deux") && !entendu.contains("alpha-un"), "the last two lines only: {entendu}");
    premier_entier(&boite, UN);
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_launch_can_turn_the_earlier_turns_off() {
    let (entendu, boite) = fil(Value::Null, json!({ "earlier_turns": false }), [UN, DEUX, TROIS]);
    assert!(!entendu.contains("alpha-un") && !entendu.contains("beta-deux"), "no earlier turn: {entendu}");
    premier_entier(&boite, UN);
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn without_trim_an_old_turn_stays_whole() {
    let long = format!("Premier tour : {}fin-du-premier.", "x".repeat(3000));
    let (entendu, _) = fil(json!({ "conversation": { "trim": { "enabled": false } } }), Value::Null, [&long, DEUX, TROIS]);
    assert!(entendu.contains("fin-du-premier"), "a turn of 3 000 characters stays whole without trim");
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_protection_bound_cuts_a_request_too_long() {
    let long = format!("Premier tour : {}fin-cachee.", "y".repeat(5000));
    let (entendu, boite) = fil(json!({ "input_max_chars": 1000 }), Value::Null, [&long, DEUX, TROIS]);
    assert!(!entendu.contains("fin-cachee") && entendu.contains("coupé"), "cut at the bound, and said: {} characters", entendu.len());
    premier_entier(&boite, &long);
}

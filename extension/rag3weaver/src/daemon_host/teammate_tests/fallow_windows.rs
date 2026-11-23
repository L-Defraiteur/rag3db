//! **The fallow windows** (Lucie, 23 Nov 2026, 08 h 30 – 09 h 30: « max 2 fenêtres de tokens actives à la fois » ; c7: the
//! whole thread while it fits a declared budget, then windows). `teammate.conversation.fallow`: past `budget_tokens`,
//! the earlier lines are grouped in windows of `window_tokens`, and only the last `windows` windows enter in clear; a
//! window that leaves is said set aside (its summary comes with the next lot). Under the budget, the thread enters
//! whole. Only the context is shaped: the mailbox keeps every turn. A module of its own.

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

/// Five turns of one thread under `teammate`; what the fifth turn heard, and the mailbox.
fn five_turns(teammate: Value, texts: &[String; 5]) -> (String, Vec<Value>) {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&manifeste).unwrap()).unwrap();
    m["teammate"] = teammate;
    std::fs::write(&manifeste, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let decl = manifeste.ancestors().find(|p| p.join(".git").exists()).unwrap().to_path_buf();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "the policy"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&decl).status().unwrap().success());
    }
    let heard = Arc::new(Mutex::new(Vec::new()));
    let host = Arc::new(hote(d.path(), listening(heard.clone())).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let mut before = 0;
    for (i, text) in texts.iter().enumerate() {
        host.post_to_teammate(&h.id, "doc", "lucie", "request", None, text, &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
        let files = host.teammate_files(&h.id, "doc").unwrap();
        let end = Instant::now() + Duration::from_secs(120);
        while files.mailbox().iter().filter(|l| l["kind"] == "ended").count() <= i {
            assert!(Instant::now() < end, "in 120 s: {:#?}", files.mailbox());
            std::thread::sleep(Duration::from_millis(100));
        }
        if i == 3 {
            before = heard.lock().unwrap().len();
        }
    }
    let mailbox = host.teammate_files(&h.id, "doc").unwrap().mailbox();
    let h = heard.lock().unwrap();
    (h[before..].join("\n"), mailbox)
}

/// Four turns of about 250 tokens each (a thousand characters), then the question.
fn texts() -> [String; 5] {
    let t = |mark: &str| format!("Tour {mark} : {} fin-{mark}.", "z".repeat(1000));
    [t("un"), t("deux"), t("trois"), t("quatre"), "Cinquième tour : et donc ?".to_string()]
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn past_the_budget_only_the_last_windows_enter_in_clear() {
    let policy = json!({ "conversation": { "kinds": ["request"], "trim": { "enabled": false }, "fallow": { "enabled": true, "budget_tokens": 600, "window_tokens": 300, "windows": 2 } } });
    let (heard, mailbox) = five_turns(policy, &texts());
    assert!(heard.contains("fin-trois") && heard.contains("fin-quatre"), "the last two windows in clear: {} characters", heard.len());
    assert!(!heard.contains("fin-un") && !heard.contains("fin-deux"), "the older windows are out of the context");
    assert!(heard.contains("mises en jachère") || heard.contains("en jachère"), "and said set aside");
    let first = mailbox.iter().find(|l| l["kind"] == "request" && l["from"] == "lucie").expect("the first request");
    assert_eq!(first["text"].as_str(), Some(texts()[0].as_str()), "the mailbox keeps the first turn whole");
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn under_the_budget_the_whole_thread_enters() {
    let policy = json!({ "conversation": { "kinds": ["request"], "trim": { "enabled": false }, "fallow": { "enabled": true, "budget_tokens": 100000, "window_tokens": 300, "windows": 2 } } });
    let (heard, _) = five_turns(policy, &texts());
    assert!(["fin-un", "fin-deux", "fin-trois", "fin-quatre"].iter().all(|m| heard.contains(m)), "under the budget, the whole thread");
}

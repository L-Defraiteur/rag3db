//! **The user is told something, in her folder's words** (Lucie's trial of 23 Nov 2026, `~/essai-2`: the agent wrote
//! its answers in step_done's `done` twice, the relaunch included, and she read nothing; then it told her to `cd` into
//! its own tree). The relaunch names the channel (the text of the reply; `done` is never shown); a turn still silent
//! after it shows her its last `done`; each step tells the agent its tree is not her folder; and a path of its tree in
//! what she reads becomes her folder's. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_silent_turn_shows_its_done_in_her_folder_words_and_the_relaunch_names_the_channel() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    crate::daemon_host::chat_qui_code::en_copie(&manifeste);
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let seen = seen.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let seen = seen.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
                let frontier = last_user(turns);
                let results = tool_results(turns).join("\n");
                seen.lock().unwrap().push(format!("{frontier}\n{results}"));
                // The agent names its tree as the frontier gives it, if it does; silent until its step_done is refused.
                let tree = frontier.split("your own tree (").nth(1).and_then(|r| r.split(')').next()).unwrap_or("/nowhere").to_string();
                let refused = results.contains("\"ok\":false");
                let text = if refused { format!("Lance npm run dev dans {tree}") } else { String::new() };
                answer(turns, opts, sink, &text, calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            })))
        })
    };
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(d.path().join("gate")));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Comment je lance la page ?", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    // Without the guard, the turn waits for the supervisor and never ends: the witness waits a bounded time, then judges.
    let fin = Instant::now() + Duration::from_secs(60);
    while !files.mailbox().iter().any(|l| l["kind"] == "ended") && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    let seen = seen.lock().unwrap().clone();
    let folder = depot.canonicalize().unwrap().display().to_string();
    assert!(seen.iter().any(|f| f.contains("never shown")), "the refusal names the channel: {seen:#?}");
    assert!(seen.iter().all(|f| f.contains(&folder)), "each step names her folder: {seen:#?}");
    let shown = b.iter().find(|l| l["from"] == "teammate" && l["kind"] == "report").unwrap_or_else(|| panic!("what the agent wrote reaches her: {b:#?}"));
    let text = shown["text"].as_str().unwrap_or_default();
    assert!(text.contains(&format!("Lance npm run dev dans {folder}")), "in her folder's words: {text}");
}

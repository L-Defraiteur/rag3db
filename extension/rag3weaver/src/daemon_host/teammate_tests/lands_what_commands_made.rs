//! **Everything a turn created lands in the user's folder** (Lucie's trial of 23 Nov 2026, `~/essai-2`: `npm create
//! vite` wrote `package.json`, `index.html`, `src/main.jsx` and `vite.config.js`, which stayed in the agent's tree, and
//! her project did not start). The host's commit takes every file the turn created, the project's `.gitignore` kept;
//! only a folder that brings hundreds of files at once (dependencies, a build) stays out, said in one line that asks
//! her nothing. A second turn that changes nothing says no change. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_generated_project_lands_whole_and_a_mass_of_dependencies_stays_out() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    crate::daemon_host::chat_qui_code::en_copie(&manifeste);
    let model: ChatModel = Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            let n = tool_results(turns).len();
            let second_turn = last_user(turns).contains("Et alors");
            match (second_turn, n) {
                (false, 0) => answer(turns, opts, sink, "", calls(&[("run_command", json!({ "command": "sh -c 'mkdir -p app/src app/node_modules/dep && echo {} > app/package.json && echo x > app/src/main.jsx && for i in $(seq 1 400); do echo $i > app/node_modules/dep/f$i.js; done'" }))])),
                (false, 1) => answer(turns, opts, sink, "", calls(&[("edit_file", json!({ "path": "app/src/App.jsx", "content": "export default 1;\n" }))])),
                _ => answer(turns, opts, sink, "C'est fait.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
            }
        })))
    });
    let gate = d.path().join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d.path(), model).with_runs_root(d.path().join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    let by = json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" });
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Crée l'app.", &by).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    assert!(depot.join("app/package.json").is_file(), "what the generator wrote lands: {b:#?}");
    assert!(depot.join("app/src/main.jsx").is_file(), "the generated sources land");
    assert!(depot.join("app/src/App.jsx").is_file(), "what the agent wrote lands");
    assert!(!depot.join("app/node_modules").exists(), "a mass of dependencies stays out of her folder");
    assert!(!b.iter().any(|l| l["text"].as_str().is_some_and(|t| t.contains("ajoute un .gitignore"))), "no gesture asked of her: {b:#?}");
    let dit = b.iter().find(|l| l["from"] == "host" && l["uncommitted"].is_number()).unwrap_or_else(|| panic!("the mass left out is said: {b:#?}"));
    assert!(dit["text"].as_str().unwrap_or_default().contains("app/node_modules/dep"), "{dit}");

    // A second turn that changes nothing says no change.
    let avant = b.len();
    host.post_to_teammate(&h.id, "doc", "lucie", "instruction", None, "Et alors ?", &json!({ "by": { "kind": "human", "client": "tty" } })).unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while files.mailbox().iter().filter(|l| l["kind"] == "ended").count() < 2 {
        assert!(Instant::now() < fin, "the second turn: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    let changes: Vec<&Value> = b[avant..].iter().filter(|l| l["kind"] == "turn_changes").collect();
    assert!(changes.iter().all(|l| l["files"].as_array().is_none_or(|f| f.is_empty())), "a turn that changed nothing says no change: {changes:#?}");
}

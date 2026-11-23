//! **The host commits what the turn made, not a mass of dependencies** (c7, 23 Nov 2026, 04:14: the nestjs request
//! landed 18 971 files, both `node_modules` and `dist/`, the agent having written no `.gitignore`). At the end of a
//! step, the host's commit takes what the turn created and changed; a folder that brings hundreds of files at once stays
//! out, and a line says how much and where, asking nothing (since Lucie's trial of 23 Nov 2026). A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

fn scripted(steps: Vec<(&'static str, Value)>, seen: Arc<Mutex<Vec<String>>>) -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        let steps = steps.clone();
        let seen = seen.clone();
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            let results = tool_results(turns);
            if let Some(r) = results.last() {
                seen.lock().unwrap().push(r.clone());
            }
            match steps.get(results.len()) {
                Some((tool, args)) => answer(turns, opts, sink, "", calls(&[(tool, args.clone())])),
                None => answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))])),
            }
        })))
    })
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_host_commits_what_the_agent_wrote_and_says_what_commands_left() {
    let d = tempfile::tempdir().unwrap();
    let (depot, manifeste) = depot_inscrit(d.path(), true);
    let steps = vec![
        ("edit_file", json!({ "path": "src/app.ts", "content": "export const app = 1;\n" })),
        ("run_command", json!({ "command": "sh -c 'mkdir -p backend/node_modules && for i in $(seq 1 400); do echo $i > backend/node_modules/f$i; done'" })),
        ("run_command", json!({ "command": "sh -c 'echo suivi >> README.md'" })),
    ];
    let gate = d.path().join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d.path(), scripted(steps, Arc::new(Mutex::new(Vec::new())))).with_runs_root(d.path().join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Crée le serveur.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    assert!(depot.join("src/app.ts").is_file(), "what the agent wrote lands: {b:#?}");
    assert!(std::fs::read_to_string(depot.join("README.md")).unwrap_or_default().contains("suivi"), "a tracked file's change lands");
    assert!(!depot.join("backend/node_modules").exists(), "a mass a command produced stays out of her folder");
    let dit = b.iter().find(|l| l["from"] == "host" && l["uncommitted"].is_number()).unwrap_or_else(|| panic!("the line says it: {b:#?}"));
    let texte = dit["text"].as_str().unwrap_or_default();
    assert!(texte.contains("400") && texte.contains("backend/node_modules") && !texte.contains(".gitignore"), "{texte}");
}

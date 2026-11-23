//! **The turn's facts come out when the turn waits too** (c7, 23 Nov 2026, 06:11: a turn suspended « en attente du
//! superviseur » had landed its work, and no `turn_changes` line said it). A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use crate::daemon_host::teammate::TeammateFiles;
use crate::daemon_host::DaemonHost;
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

fn lancer(d: &std::path::Path, steps: Vec<(&'static str, Value)>, demande: &str) -> (Arc<DaemonHost>, TeammateFiles) {
    let (_depot, manifeste) = depot_inscrit(d, true);
    let gate = d.join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d, scripted(steps, Arc::new(Mutex::new(Vec::new())))).with_runs_root(d.join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, demande, &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    (host, files)
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_turn_that_waits_says_its_facts() {
    let d = tempfile::tempdir().unwrap();
    // A request that asks for a finding, a step whose command writes and that ends without a report: the turn waits.
    let steps = vec![("run_command", json!({ "command": "sh -c 'echo pendu > pendu.py'" }))];
    let (_host, files) = lancer(d.path(), steps, "Trouve le défaut du pendu.");
    let fin = Instant::now() + Duration::from_secs(120);
    loop {
        let b = files.mailbox();
        if b.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("j'attends le superviseur") || t.contains("en attente du superviseur"))) {
            let faits = b.iter().find(|l| l["from"] == "host" && l["kind"] == "turn_changes").unwrap_or_else(|| panic!("the turn's facts before it waits: {b:#?}"));
            assert!(faits["text"].as_str().unwrap_or_default().contains("pendu.py"), "{faits}");
            break;
        }
        assert!(Instant::now() < fin, "the turn waits: {b:#?}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

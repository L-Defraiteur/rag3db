//! **At `bac`, a line the parser cannot reduce runs through `sh -c`** (c7, the terminal's night, 23 Nov 2026: « un agent
//! de code écrit `cmd > log 2>&1` dix fois par tâche »). The shell parser tells the truth and refuses a redirection,
//! a substitution, an expansion or a glob; at the `bac` level the sandbox bounds what the line reaches, so the host
//! runs it as `sh -c '<line>'`, which the gate lets through, and says so. A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

/// A fake teammate that calls the given tools in order, then ends its step.
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
fn at_bac_a_redirection_and_a_glob_run_through_the_shell() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let steps = vec![
        ("run_command", json!({ "command": "echo fizz > fizz.txt 2>&1" })),
        ("run_command", json!({ "command": "ls *.txt" })),
    ];
    let gate = d.path().join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d.path(), scripted(steps, seen.clone())).with_runs_root(d.path().join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Écris fizz.txt.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    let boite = loop {
        let b = files.mailbox();
        if ended(&b) || b.iter().any(|l| l["kind"] == "approval") {
            break b;
        }
        assert!(Instant::now() < fin, "in 120 s: {b:#?}");
        std::thread::sleep(Duration::from_millis(100));
    };
    let vus = seen.lock().unwrap().clone();
    assert!(boite.iter().all(|l| l["kind"] != "approval"), "no card: {boite:#?}");
    let t = host.tache(&h.id, "doc").expect("the task");
    let arbre = t.workspace.as_ref().map(|w| w.racine.clone()).unwrap_or(t.arbre);
    assert_eq!(std::fs::read_to_string(arbre.join("fizz.txt")).ok().as_deref(), Some("fizz\n"), "the redirection wrote in the tree: {vus:#?}");
    assert!(vus.get(1).is_some_and(|r| r.contains("fizz.txt") && !r.contains("refusée")), "the glob is expanded by the shell: {vus:#?}");
}

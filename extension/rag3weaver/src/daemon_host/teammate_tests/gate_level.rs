//! **The gate's level, chosen at the launch** (Lucie, 22 Nov 2026, 23 h 20 and 23 h 22, by c7): `bac` (every command
//! the sandbox runs passes), `judge` (the judge decides, the card when it says no), `cards` (every command the guard
//! holds asks). The level a request carries (`gate`) beats the project's declaration: a project declared in approval,
//! launched with `bac`, writes, runs and commits without a card. At every level, a write by the code tools in the task's
//! tree never asks. The level is said in the mailbox when the task starts (`gate_level`). A module of its own.

use crate::daemon_host::chat_qui_code::{depot_inscrit, hote};
use crate::daemon_host::teammate::TeammateFiles;
use crate::daemon_host::DaemonHost;
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

/// A project declared in approval, launched at `level`, until its end or its first card.
fn lancer(d: &std::path::Path, level: &str, steps: Vec<(&'static str, Value)>) -> (Arc<DaemonHost>, String, TeammateFiles, Vec<Value>, Arc<Mutex<Vec<String>>>) {
    let (_depot, manifeste) = depot_inscrit(d, true);
    super::en_approbation(&manifeste);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let gate = d.join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d, scripted(steps, seen.clone())).with_runs_root(d.join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the repository opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Écris fizz.txt et commite-le.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": level })).unwrap();
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
    (host, h.id.clone(), files, boite, seen)
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_launch_at_bac_beats_a_project_declared_in_approval() {
    let d = tempfile::tempdir().unwrap();
    let steps = vec![
        ("run_command", json!({ "command": "sh -c 'echo fizz > fizz.txt'" })),
        ("run_command", json!({ "command": "git add fizz.txt" })),
        ("run_command", json!({ "command": "git commit -q -m fizz" })),
    ];
    let (host, id, _files, boite, seen) = lancer(d.path(), "bac", steps);
    let cartes: Vec<&Value> = boite.iter().filter(|l| l["kind"] == "approval").collect();
    assert!(cartes.is_empty(), "the launch's level beats the project's declaration: {cartes:#?}; results {:?}", seen.lock().unwrap());
    let dit = boite.iter().find(|l| l["from"] == "host" && l["kind"] == "gate_level").unwrap_or_else(|| panic!("the level is said: {boite:#?}"));
    assert_eq!(dit["level"], "bac", "{dit}");
    let t = host.tache(&id, "doc").expect("the task");
    let arbre = t.workspace.as_ref().map(|w| w.racine.clone()).unwrap_or(t.arbre);
    let log = std::process::Command::new("git").arg("-C").arg(&arbre).args(["log", "--format=%s", "-1"]).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&log.stdout).trim(), "fizz", "the commit is on the task's branch");
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn under_cards_the_code_tools_write_and_edit_without_a_card_and_a_command_asks() {
    let d = tempfile::tempdir().unwrap();
    let steps = vec![
        ("edit_file", json!({ "path": "fizz.txt", "content": "fizz\n" })),
        ("edit_file", json!({ "path": "fizz.txt", "old": "fizz", "new": "buzz" })),
        ("run_command", json!({ "command": "git add fizz.txt" })),
    ];
    let (host, id, _files, boite, seen) = lancer(d.path(), "cards", steps);
    let t = host.tache(&id, "doc").expect("the task");
    let arbre = t.workspace.as_ref().map(|w| w.racine.clone()).unwrap_or(t.arbre);
    assert_eq!(std::fs::read_to_string(arbre.join("fizz.txt")).ok().as_deref(), Some("buzz\n"), "written then edited by the code tools: {:?}", seen.lock().unwrap());
    let cartes: Vec<&Value> = boite.iter().filter(|l| l["kind"] == "approval").collect();
    assert_eq!(cartes.len(), 1, "one card, for the command: {cartes:#?}");
    assert!(cartes[0].to_string().contains("git add"), "the card is the command's: {}", cartes[0]);
}

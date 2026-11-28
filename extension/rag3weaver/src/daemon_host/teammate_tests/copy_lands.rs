//! **A folder without git receives the work too** (c7, 23 Nov 2026: a full folder, not versioned, never received
//! anything: the task works in a copy). At the end of each step, the host copies into the user's folder what the agent
//! wrote by its code tools, the files that existed and changed, and what commands produced (since Lucie's trial of
//! 23 Nov 2026: a project made by a generator lands whole); only a mass of files at once stays in the copy, said. The line says there is no git there. And the turn's facts come from the folder's diff. A module of its own.

use crate::daemon_host::chat_qui_code::hote;
use crate::daemon_host::DaemonHost;
use super::mode_auto_v1::ended;
use super::*;
use std::path::{Path, PathBuf};
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

/// A code backend whose manifest and root are in no git repository (the root read-only, writes allowed to tasks).
fn sans_git(d: &Path, model: ChatModel) -> (Arc<DaemonHost>, String, PathBuf) {
    let config = d.join("config").join("code");
    let racine = d.join("racine");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::create_dir_all(racine.join("src")).unwrap();
    std::fs::write(racine.join("src").join("main.py"), "def supprime(): pass\n").unwrap();
    std::fs::write(racine.join("notes.md"), "à garder\n").unwrap();
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &config).unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(racine.to_string_lossy());
    m["workspace"]["read_only"] = json!(true);
    m["workspace"]["task_writes"] = json!(true);
    // The copy is asked for: a folder without git is worked in place otherwise (24 Nov 2026).
    m["teammate"]["workspace"] = json!("copy");
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let gate = d.join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d, model).with_runs_root(d.join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the backend opens");
    (host, h.id.clone(), racine.canonicalize().unwrap())
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_folder_without_git_receives_what_the_agent_wrote() {
    let d = tempfile::tempdir().unwrap();
    let steps = vec![
        ("edit_file", json!({ "path": "src/main.py", "content": "def supprime(): return True\n" })),
        ("edit_file", json!({ "path": "tests/test_main.py", "content": "def test_x():\n    assert True\n" })),
        ("run_command", json!({ "command": "sh -c 'mkdir -p build && echo bin > build/out.bin'" })),
    ];
    let (host, id, racine) = sans_git(d.path(), scripted(steps, Arc::new(Mutex::new(Vec::new()))));
    host.post_to_teammate(&id, "doc", "lucie", "request", None, "Corrige la suppression.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    assert_eq!(std::fs::read_to_string(racine.join("src/main.py")).unwrap(), "def supprime(): return True\n", "the changed file is in her folder: {b:#?}");
    assert!(racine.join("tests/test_main.py").is_file(), "the file the agent wrote is in her folder");
    assert!(racine.join("build/out.bin").is_file(), "what a command produced lands too");
    let dit = b.iter().find(|l| l["from"] == "host" && l["kind"] == "merged").unwrap_or_else(|| panic!("the line: {b:#?}"));
    assert!(dit["text"].as_str().unwrap_or_default().contains("pas de git"), "{dit}");
    let faits = b.iter().find(|l| l["from"] == "host" && l["kind"] == "turn_changes").unwrap_or_else(|| panic!("the turn's facts, by the folder's diff: {b:#?}"));
    assert!(faits["text"].as_str().unwrap_or_default().contains("src/main.py"), "{faits}");
}

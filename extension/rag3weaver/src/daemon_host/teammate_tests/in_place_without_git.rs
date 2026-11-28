//! **A folder without git is worked in place** (Lucie, 24 Nov 2026: « faut arrêter ça les copies de travail
//! maintenant, on travaille direct dans le dossier » — that day, a task copied 106 GB of a parent folder). The task
//! writes in the user's folder itself: no copy of it under the task, nothing to land after the step. A copy stays
//! possible when `teammate.workspace` asks for it. By the host's own path. A module of its own.

use crate::daemon_host::chat_qui_code::hote;
use super::mode_auto_v1::ended;
use super::*;
use std::path::Path;
use std::time::{Duration, Instant};

fn writes_one_file() -> ChatModel {
    Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            if tool_results(turns).is_empty() {
                answer(turns, opts, sink, "", calls(&[("edit_file", json!({ "path": "src/main.py", "content": "def supprime(): return True\n" }))]))
            } else {
                answer(turns, opts, sink, "Corrigé.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
            }
        })))
    })
}

fn dirs_named(root: &Path, name: &str, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(root).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() && !p.is_symlink() {
            if p.file_name().is_some_and(|n| n == name) {
                out.push(p.clone());
            }
            dirs_named(&p, name, out);
        }
    }
}

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_folder_without_git_is_worked_in_place() {
    let d = tempfile::tempdir().unwrap();
    let config = d.path().join("config").join("code");
    let racine = d.path().join("racine");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::create_dir_all(racine.join("src")).unwrap();
    std::fs::write(racine.join("src").join("main.py"), "def supprime(): pass\n").unwrap();
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let manifeste = crate::gabarits::jouable::manifeste_jouable(&gabarit, &config).unwrap();
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["workspace"]["root"] = json!(racine.to_string_lossy());
    m["workspace"]["read_only"] = json!(true);
    m["workspace"]["task_writes"] = json!(true);
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let gate = d.path().join("gate");
    std::fs::create_dir_all(&gate).unwrap();
    let host = Arc::new(hote(d.path(), writes_one_file()).with_runs_root(d.path().join("runs")).with_gate_dir(gate));
    host.se_connaitre();
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("the backend opens");
    host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Corrige la suppression.", &json!({ "by": { "kind": "human", "client": "tty" }, "gate": "bac" })).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) {
        assert!(Instant::now() < fin, "in 120 s: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(std::fs::read_to_string(racine.join("src/main.py")).unwrap(), "def supprime(): return True\n", "the change is in her folder");
    let mut copies = Vec::new();
    dirs_named(d.path(), "espace", &mut copies);
    assert!(copies.is_empty(), "no working copy of her folder under the task: {copies:#?}");
    let b = files.mailbox();
    assert!(!b.iter().any(|l| l["from"] == "host" && l["kind"] == "merged"), "nothing to land after the step: {b:#?}");
}

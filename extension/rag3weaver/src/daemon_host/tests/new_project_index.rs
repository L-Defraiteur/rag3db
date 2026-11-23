//! **A new project gets its index without anyone asking** (Lucie, 23 Nov 2026: « crée-moi une app » left a project
//! with neither a search by meaning nor `usages`): a folder the host created when it was opened is indexed by the host
//! itself, in the background, at the first landing of a turn that brings it code, and the mailbox says it in one line.
//! A module of its own.

use super::*;
use crate::daemon_host::atelier_fichiers::tests::{atelier_avec, attendre_l_index};
use crate::llm::{CallbackLlm, Llm, MockLlm};
use std::time::{Duration, Instant};

#[test]
fn a_folder_the_host_created_is_indexed_at_its_first_landing() {
    let d = tempfile::tempdir().unwrap();
    let n = Arc::new(std::sync::Mutex::new(0usize));
    let model: crate::daemon_host::chat::ChatModel = {
        let n = n.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let n = n.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
                let k = { let mut g = n.lock().unwrap(); *g += 1; *g };
                let (text, calls) = if k == 1 {
                    ("", vec![("edit_file".to_string(), json!({ "path": "src/main.rs", "content": "fn main() {\n    println!(\"un\");\n}\n" }).to_string())])
                } else {
                    ("J'ai écrit src/main.rs.", vec![("step_done".to_string(), json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }).to_string())])
                };
                MockLlm::new(text).with_context_len(16384).with_tool_calls(calls).generate(turns, opts, sink)
            })))
        })
    };
    let a = atelier_avec(d.path(), model);
    let espace = d.path().join("git_workspaces");
    std::fs::create_dir_all(&espace).unwrap();
    let table = d.path().join("dossiers-des-sessions.json");
    std::fs::write(&table, "{}").unwrap();
    a.host.set_espace(espace.clone(), table);
    let corps = json!({"client": "ide", "claude": {"session_id": "S1", "name": "le témoin"}}).to_string();
    let (_, r) = a.host.handle("POST", "/session", None, corps.as_bytes());
    let s = r["session"].as_str().unwrap().to_string();
    let neuf = espace.join("mon-app");
    let r = a.host.ouvrir_un_dossier(Some(&s), &neuf);
    assert_eq!(r["ok"], json!(true), "{r}");
    let racines = a.host.racines_de_code().unwrap();
    let nom = crate::code_tree::name_of_tree(&neuf.canonicalize().unwrap()).unwrap();
    let manifeste = racines.manifest(&nom).canonicalize().unwrap();
    let note = manifeste.ancestors().find(|p| p.file_name().is_some_and(|n| n == "templates")).unwrap().parent().unwrap().join("no-index.json");
    assert!(note.is_file(), "the witness starts from a new folder without an index");

    let h = a.host.hosted(&manifeste).unwrap();
    a.host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Crée-moi une petite app.", &json!({ "by": { "kind": "human", "client": "ide" }, "gate": "bac" })).unwrap();
    let files = a.host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !files.mailbox().iter().any(|l| l["kind"] == "ended") {
        assert!(Instant::now() < fin, "the turn ends: {:#?}", files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
    let b = files.mailbox();
    assert!(b.iter().any(|l| l["kind"] == "merged"), "the turn lands its code in the folder: {b:#?}");
    assert!(neuf.join("src/main.rs").is_file(), "main.rs is in the folder");
    assert!(
        b.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("index lancé"))),
        "the host says it indexes the new project: {b:#?}"
    );
    assert!(!note.exists(), "the folder is indexed without anyone asking");
    attendre_l_index(&a, &neuf);
}

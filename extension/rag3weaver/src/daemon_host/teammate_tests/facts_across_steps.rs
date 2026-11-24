//! **The facts of every file mapped survive the steps that follow** (the measure C of 24 Nov 2026, 4 h 29: the block
//! kept the first 40 scopes of the first `outline` only, half of them `file_scope` headers, and only the step before
//! passed; the files mapped next, and those of the first step, were written from memory). By the host's own path: a step
//! maps a large file and a small one, a step without the index follows, and the third step still sees a symbol of each
//! file, and no `file_scope`. A module of its own.

use crate::daemon_host::atelier_fichiers::tests::{atelier_avec, attendre_l_index, petit_depot};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_third_step_still_sees_a_symbol_of_each_file_the_first_mapped() {
    let d = tempfile::tempdir().unwrap();
    let vus = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let vus = vus.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let vus = vus.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
                let step = step_of(turns);
                let results = tool_results(turns);
                if step >= 3 && results.is_empty() {
                    vus.lock().unwrap().push(last_user(turns));
                }
                match (step, results.len()) {
                    (1, 0) => answer(turns, opts, sink, "Je cartographie.", calls(&[("outline", json!({ "path": "src/grand.rs" })), ("outline", json!({ "path": "src/lib.rs" }))])),
                    (1, _) => answer(turns, opts, sink, "J'ai les plans.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "plans lus", "remaining": "lire" }))])),
                    (2, 0) => answer(turns, opts, sink, "Je liste.", calls(&[("list_files", json!({}))])),
                    (2, _) => answer(turns, opts, sink, "Listé.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "listé", "remaining": "écrire" }))])),
                    _ => answer(turns, opts, sink, "Le rapport est écrit.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "écrit", "remaining": "" }))])),
                }
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
    let depot = petit_depot(&espace);
    let grand: String = (0..200).map(|i| format!("/// La fonction {i}.\npub fn fonction_{i}() -> u32 {{\n    {i}\n}}\n\n")).collect();
    std::fs::write(depot.join("src/grand.rs"), grand).unwrap();
    for args in [&["add", "-A"][..], &["commit", "-q", "-m", "un grand fichier"]] {
        assert!(std::process::Command::new("git").args(args).current_dir(&depot).status().unwrap().success());
    }
    assert_eq!(a.host.ouvrir_un_dossier(Some(&s), &depot)["ok"], json!(true));
    a.host.index_folder(&depot).expect("the folder is indexed");
    attendre_l_index(&a, &depot);
    let racines = a.host.racines_de_code().unwrap();
    let manifeste = racines.manifest(&crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap()).canonicalize().unwrap();
    let h = a.host.hosted(&manifeste).unwrap();
    a.host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Cartographie le code, puis écris un rapport.", &json!({ "by": { "kind": "human", "client": "ide" } })).unwrap();
    let files = a.host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let vus = vus.lock().unwrap().join("\n");
    assert!(vus.contains("src/grand.rs:") && vus.contains("src/lib.rs:"), "the third step sees a symbol of each file the first mapped: {vus}");
    assert!(!vus.contains("file_scope"), "no file_scope header among the facts: {vus}");
}

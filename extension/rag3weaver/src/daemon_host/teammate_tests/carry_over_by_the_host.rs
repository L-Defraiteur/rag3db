//! **The next step sees what the index showed the step before, by the host's own path** (the measure of 24 Nov 2026,
//! 3 h: `carry-over-facts` landed green while the shipped manifest named no `carry_over_node`, so the product never
//! called it). A folder opened by the host from the shipped templates, a step that only mapped by `outline`, and the
//! next step's message carries « Found by the code index » with its places. A module of its own.

use crate::daemon_host::atelier_fichiers::tests::{atelier_avec, attendre_l_index, petit_depot};
use super::mode_auto_v1::ended;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn the_next_step_sees_the_places_the_index_gave() {
    let d = tempfile::tempdir().unwrap();
    let vus = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let vus = vus.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let vus = vus.clone();
            Ok(Arc::new(CallbackLlm::new("faux-equipier", 16384, move |turns, opts, sink| {
                let step = step_of(turns);
                let results = tool_results(turns);
                if step >= 2 && results.is_empty() {
                    vus.lock().unwrap().push(last_user(turns));
                }
                match (step, results.len()) {
                    (1, 0) => answer(turns, opts, sink, "Je cartographie le fichier.", calls(&[("outline", json!({ "path": "src/lib.rs" }))])),
                    (1, _) => answer(turns, opts, sink, "J'ai le plan du fichier.", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "plan lu", "remaining": "écrire le rapport" }))])),
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
    assert_eq!(a.host.ouvrir_un_dossier(Some(&s), &depot)["ok"], json!(true));
    a.host.index_folder(&depot).expect("the folder is indexed");
    attendre_l_index(&a, &depot);
    let racines = a.host.racines_de_code().unwrap();
    let manifeste = racines.manifest(&crate::code_tree::name_of_tree(&depot.canonicalize().unwrap()).unwrap()).canonicalize().unwrap();
    let h = a.host.hosted(&manifeste).unwrap();
    a.host.post_to_teammate(&h.id, "doc", "lucie", "request", None, "Cartographie src/lib.rs, puis écris un rapport.", &json!({ "by": { "kind": "human", "client": "ide" } })).unwrap();
    let files = a.host.teammate_files(&h.id, "doc").unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while !ended(&files.mailbox()) && Instant::now() < fin {
        std::thread::sleep(Duration::from_millis(100));
    }
    let vus = vus.lock().unwrap().join("\n");
    assert!(vus.contains("Found by the code index") && vus.contains("src/lib.rs:"), "the second step's message carries the places the index gave: {vus}");
}

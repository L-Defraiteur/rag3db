//! **A tool of another backend, imported by the manifest, is offered to the agent and reaches that backend** (23 Nov
//! 2026: the agent of a code backend did not reach the long-term memory its manifest declares; the sessions did, by the
//! host's MCP, not the product's agent). A module of its own.

use super::*;

#[test]
fn an_imported_memory_tool_reaches_the_memory_the_manifest_names() {
    let m = tempfile::tempdir().unwrap();
    let gabarit = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/memory");
    let memoire = crate::gabarits::jouable::manifeste_jouable(&gabarit, m.path()).unwrap();
    let memoire_txt = memoire.to_string_lossy().to_string();
    let vus = Arc::new(Mutex::new(Vec::<String>::new()));
    let model: ChatModel = {
        let vus = vus.clone();
        Arc::new(move |_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
            let vus = vus.clone();
            Ok(Arc::new(CallbackLlm::new("faux-lecteur", 8192, move |turns, opts, sink| {
                let resultats: Vec<String> = turns.iter().filter(|t| t.role == "tool").map(|t| t.content.clone()).collect();
                vus.lock().unwrap().extend(resultats.iter().cloned());
                let modele = if resultats.is_empty() {
                    MockLlm::new("Je regarde la mémoire.").with_tool_calls(vec![("get_fiche", r#"{"record":{"name":"la fiche du témoin"}}"#)])
                } else {
                    MockLlm::new("Fini.")
                };
                modele.generate(turns, opts, sink)
            })))
        })
    };
    let (_d, host, _manifeste, id) = banc_avec(model, move |c| {
        c["imports"] = json!([{ "backend": memoire_txt, "tools": ["get_fiche", "recall_fiches"] }]);
    });
    let mem = host.hosted(&memoire).unwrap();
    let r = host.with_backend(&mem, |b| b.call("put_fiche", json!({ "record": { "name": "la fiche du témoin", "about": "une note posée avant le tour", "state": "current" } }))).unwrap();
    assert!(r.is_ok(), "the fiche is put: {r:?}");
    host.chat_turn(&id, "que dit la mémoire ?").expect("the turn");
    let vus = vus.lock().unwrap().join("\n");
    assert!(vus.contains("une note posée avant le tour"), "the imported get_fiche reached the memory: {vus}");
}

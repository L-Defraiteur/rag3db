//! **The run of a step names its task and its number** (the agent page of the IDE, 21 Nov 2026:
//! the page finds a task's current step by its runs). A module of its own.

use super::*;

/// A teammate's step turn says, in its `RunStarted`, the task and the step's number; a run that
/// is not a step's turn says none.
#[cfg(all(feature = "code", feature = "rag3db-native"))]
#[test]
fn the_run_of_a_step_names_its_task_and_number() {
    let d = tempfile::tempdir().unwrap();
    let (_depot, manifeste) = depot_inscrit(d.path(), true);
    let model: ChatModel = Arc::new(|_b: &crate::backend::PreparedBackend| -> Result<Arc<dyn Llm>, String> {
        Ok(Arc::new(CallbackLlm::new("faux-equipier", 8192, move |turns, opts, sink| {
            answer(turns, opts, sink, "", calls(&[("step_done", json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" }))]))
        })))
    });
    let host = hote(d.path(), model).with_runs_root(d.path().join("runs"));
    let h = host.hosted(&manifeste).unwrap();
    host.with_backend(&h, |_| ()).expect("le principal s'ouvre");
    let tache = host.ouvrir_une_tache(&h.id, "doc").expect("la tâche s'ouvre");
    let t = host.hote_par_id(&tache.id).unwrap();
    let mut agent = host.with_backend(&t, |b| b.bus_de_publication().subscribe(crate::events::topic::AGENT)).unwrap();
    let files = host.teammate_files(&h.id, "doc").unwrap();
    files.append_with("supervisor", "request", None, "Fais-le.", &json!({ "expects": "none" })).unwrap();
    host.run_teammate(&h.id, "doc").unwrap();
    let mut debuts = Vec::new();
    while let Ok(e) = agent.try_recv() {
        let e = e.to_json();
        if e["kind"] == "RunStarted" && e["run_kind"] == "agent" {
            debuts.push(e);
        }
    }
    assert!(!debuts.is_empty(), "the step's turn started a run");
    assert_eq!(debuts[0]["step"], json!({ "task": "doc", "number": 1 }), "the run of step 1 of « doc » says so: {debuts:#?}");
}

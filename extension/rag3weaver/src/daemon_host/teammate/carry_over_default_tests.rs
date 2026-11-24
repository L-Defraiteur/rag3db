//! **The shipped code backend declares its `Previous step` node** (the measure of 24 Nov 2026, 3 h: the shipped
//! manifest named no `carry_over_node`, so the Rust block served, which keeps no `outline` nor index result; a step
//! that mapped by `outline` left nothing to the next one, which wrote from memory). A module of its own.

use super::*;
use serde_json::json;

#[test]
fn the_shipped_code_backend_keeps_the_index_facts_for_the_next_step() {
    let gabarit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/code");
    let m: Value = serde_json::from_str(&std::fs::read_to_string(gabarit.join("backend.json")).unwrap()).unwrap();
    let noeud = m["teammate"]["carry_over_node"].as_str().map(str::to_string);
    assert_eq!(noeud.as_deref(), Some("PreviousStep"), "the shipped manifest declares the node that keeps the facts: {}", m["teammate"]);
    let w = StepWork {
        read: Vec::new(),
        wrote: Vec::new(),
        command: None,
        results: vec![("outline".into(), "- enum `Status` — extension/rag3weaver/src/daemon_host/teammate.rs:54-60\n".into())],
    };
    let bloc = carry_over_by_node(&gabarit.join("nodes"), noeud.as_deref().unwrap_or_default(), &json!({}), w.to_json(1, "", 8_000)).unwrap();
    assert!(bloc.contains("teammate.rs:54"), "a step that only mapped by outline leaves its places to the next one: {bloc}");
}

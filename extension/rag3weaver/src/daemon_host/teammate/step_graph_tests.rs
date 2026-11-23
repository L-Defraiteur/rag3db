//! The witness of the lot `step-hooks` (23 November 2026): `step_done` asks a declared graph, which refuses or accepts.
use super::*;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

const SILENT_REFUSED: &str = "%% tool: at_step_done_witness
%% param: step json! -- the facts of the step
%% result: chain.verdict
graph LR
    facts[\"RhaiNode(script='input', value=$step)\"]
    after[\"TextAfterLastTool()\"]
    silent[\"Not()\"]
    chain[\"FirstMatch(rules='silent', otherwise='accept')\"]
    facts -->|result:step| after
    after -->|holds:holds| silent
    silent -->|holds:silent| chain
";

const TABLE: &str = r#"{ "silent": { "action": "refuse", "text_key": "witness_write_first", "max_refusals": 2 } }"#;

fn step(d: &std::path::Path) -> Step {
    let graph = super::super::step_graphs::StepGraph::parse(SILENT_REFUSED, TABLE).expect("the witness's graph binds");
    Step::new(TeammateFiles::new(d.join("box"), "t"), "t", "r", None, 3)
        .with_delivery(0, super::super::prompts::Texts::shipped())
        .with_at_step_done(Some(std::sync::Arc::new(graph)))
}

fn closing() -> Value {
    json!({ "item": "", "succeeded": true, "done": "fait", "remaining": "" })
}

/// **A declared rule refuses `step_done`, numbered, then accepts it said answered nothing**: the graph reads the step's
/// facts (no text after the last tool) and its table says refuse twice; a step that wrote after its last tool is
/// accepted at once. No rule of this witness is in the host's code: it lives in the graph and its table.
#[test]
fn step_done_asks_its_declared_graph_which_refuses_then_accepts() {
    let d = tempfile::tempdir().unwrap();
    let s = step(d.path());
    s.used_tools.store(true, Ordering::SeqCst);
    let facts = s.step_facts(Some(&closing()));
    assert_eq!((facts["text_since_last_tool"].clone(), facts["used_tools"].clone()), (json!(false), json!(true)), "{facts:#}");
    assert_eq!(facts["step_done"]["succeeded"], json!(true));
    for n in 1..=2 {
        let e = s.step_done(&closing()).unwrap_err();
        assert_eq!(e, format!("witness_write_first ({n}/2)"), "refusal {n}");
    }
    s.step_done(&closing()).expect("accepted after its two refusals");
    assert!(s.answered_nothing(), "and said answered nothing");

    let d = tempfile::tempdir().unwrap();
    let s = step(d.path());
    s.used_tools.store(true, Ordering::SeqCst);
    s.wrote_text.store(true, Ordering::SeqCst);
    s.step_done(&closing()).expect("a step that wrote after its last tool is accepted at once");
    assert!(!s.answered_nothing());
}

/// **The shipped graph accepts** (its rules move into it one by one), and a table that refuses without its text is
/// refused at reading.
#[test]
fn the_shipped_graph_accepts_and_a_table_without_its_text_is_refused() {
    let shipped = super::super::step_graphs::StepGraph::parse(
        include_str!("../../../templates/teammate/graphs/at_step_done.mmd"),
        include_str!("../../../templates/teammate/graphs/at_step_done.json"),
    )
    .expect("the shipped graph binds");
    let (verdict, _, a) = shipped.decide(&json!({})).unwrap();
    assert_eq!((verdict.as_str(), a.action.as_str()), ("accept", "accept"));
    let e = super::super::step_graphs::StepGraph::parse(SILENT_REFUSED, r#"{ "silent": { "action": "refuse" } }"#).err().unwrap();
    assert!(e.contains("text_key"), "{e}");
}

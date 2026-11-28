//! **One call that marks the last item done and closes the plan does both** (24 Nov 2026: in three runs of the measure,
//! `{"close": …, "item": {"status": "done"}}` was refused — the close ran first, then the item found no active plan —,
//! while the closed plan was already filed; the model spent three more calls, `resume` refused « a plan is already
//! active »). Through `ours`, the door every tool call of the model goes through. A module of its own.

use super::*;
use serde_json::json;

fn call(args: Value) -> ToolCall {
    ToolCall::new("c1", "edit_plan", args.to_string())
}

fn step_with_a_plan(d: &std::path::Path) -> Step {
    let s = Step::new(TeammateFiles::new(d.join("boite"), "t"), "t", "r", None, 1);
    let r = s.ours(&call(json!({ "plan": { "request": "r", "items": [
        { "id": "lire", "text": "lire", "status": "done", "alone": true },
        { "id": "ecrire", "text": "écrire", "status": "doing", "alone": true },
    ] } })));
    assert!(r.unwrap().content.contains("\"ok\":true"));
    s
}

#[test]
fn one_call_marks_the_last_item_done_and_closes_the_plan() {
    let d = tempfile::tempdir().unwrap();
    let s = step_with_a_plan(d.path());
    let r = s.ours(&call(json!({ "item": { "id": "ecrire", "status": "done" }, "close": { "outcome": "accomplished", "reason": "écrit" } }))).unwrap();
    assert!(r.content.contains("\"ok\":true"), "item and close in one call: {}", r.content);
    let files = TeammateFiles::new(d.path().join("boite"), "t");
    assert!(files.read_plans().active().is_none(), "no plan is active after it");
    let closed = files.closed();
    assert_eq!(closed.len(), 1, "{closed:#?}");
    assert!(closed[0].items.iter().all(|i| i.status == Status::Done), "the closed plan has its last item done: {closed:#?}");
    assert_eq!(s.closed_plans().len(), 1, "filed once");
}

#[test]
fn a_refused_call_closes_nothing() {
    let d = tempfile::tempdir().unwrap();
    let s = step_with_a_plan(d.path());
    let r = s.ours(&call(json!({ "item": { "id": "ecrire", "status": "fini" }, "close": { "outcome": "accomplished", "reason": "écrit" } }))).unwrap();
    assert!(r.content.contains("\"ok\":false"), "an unknown status refuses the call: {}", r.content);
    let files = TeammateFiles::new(d.path().join("boite"), "t");
    assert!(files.read_plans().active().is_some(), "the plan is still active");
    assert!(files.closed().is_empty(), "nothing is filed by a refused call: {:#?}", files.closed());
    assert!(s.closed_plans().is_empty(), "nor kept for the memory");
}

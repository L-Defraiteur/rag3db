//! The witness of the end-of-step conditions (23 November 2026): each answers its one question from the step's facts.
use super::*;
use serde_json::json;

fn ask(node_type: &str, config: Value, step: Value) -> (bool, String) {
    let c = conditions().into_iter().find(|c| c.node_type == node_type).unwrap_or_else(|| panic!("{node_type} is a condition"));
    let mut node = ConditionFactory(c).create("n", &config).unwrap();
    let mut ctx = NodeContext::new();
    ctx.set_input(STEP, crate::dataflow::port::PortValue::new(step));
    node.execute(&mut ctx).unwrap();
    let v = ctx.drain_outputs().remove(HOLDS).and_then(|v| v.downcast::<Value>().cloned()).expect("a verdict");
    (v["holds"].as_bool().unwrap(), v["why"].as_str().unwrap_or_default().to_string())
}

fn humaine(texte: &str) -> Value {
    json!({ "from": "lucie", "kind": "request", "by": { "kind": "human" }, "text": texte, "id": "t:1" })
}

#[test]
fn the_steps_own_flags_are_read_as_they_are() {
    assert!(ask("TextAfterLastTool", json!({}), json!({ "text_since_last_tool": true })).0);
    assert!(!ask("TextAfterLastTool", json!({}), json!({ "text_since_last_tool": false })).0);
    assert!(ask("WroteInStep", json!({}), json!({ "text_in_step": true })).0);
    assert!(!ask("WroteInStep", json!({}), json!({})).0, "a fact absent is false");
    assert!(ask("UsedTools", json!({}), json!({ "used_tools": true })).0);
}

#[test]
fn whose_request_opened_the_turn() {
    let agent = json!({ "from": "superviseur", "kind": "instruction", "by": { "kind": "agent" }, "text": "x" });
    let hote = json!({ "from": "host", "kind": "request", "text": "relance" });
    let s = json!({ "turn": [humaine("Regarde.")] });
    assert!(ask("TurnAskedBy", json!({ "who": "human" }), s.clone()).0);
    assert!(!ask("TurnAskedBy", json!({ "who": "agent" }), s.clone()).0);
    assert!(ask("TurnAskedBy", json!({ "who": "any" }), s).0);
    let s = json!({ "turn": [agent, hote] });
    assert!(ask("TurnAskedBy", json!({ "who": "agent" }), s.clone()).0, "the host's relaunch is nobody's request");
    assert!(!ask("TurnAskedBy", json!({ "who": "human" }), s).0);
    assert!(!ask("TurnAskedBy", json!({ "who": "any" }), json!({ "turn": [] })).0);
    assert!(ConditionFactory(conditions().into_iter().find(|c| c.node_type == "TurnAskedBy").unwrap()).create("n", &json!({ "who": "robot" })).is_err());
}

#[test]
fn a_teammate_line_since_the_request() {
    let rapport = json!({ "from": "teammate", "kind": "report", "text": "Je regarde." });
    let fait = json!({ "from": "teammate", "kind": "step_done", "text": "fait" });
    assert!(ask("TurnHasTeammateLine", json!({}), json!({ "turn": [humaine("x"), rapport.clone()] })).0);
    assert!(!ask("TurnHasTeammateLine", json!({}), json!({ "turn": [humaine("x"), fait] })).0, "a step_done is no line to the asker");
    assert!(!ask("TurnHasTeammateLine", json!({}), json!({ "turn": [rapport, humaine("x")] })).0, "a line before the request does not count");
}

#[test]
fn a_step_done_that_closes_the_turn() {
    let fini = json!({ "item": "", "succeeded": true, "remaining": "" });
    assert!(ask("ClosingStepDone", json!({}), json!({ "step_done": fini.clone(), "plan": null })).0);
    assert!(!ask("ClosingStepDone", json!({}), json!({ "step_done": { "item": "", "succeeded": true, "remaining": "le CHANGELOG" } })).0, "something left");
    assert!(!ask("ClosingStepDone", json!({}), json!({ "step_done": { "item": "", "succeeded": false, "remaining": "" } })).0, "not succeeded");
    let plan = json!({ "items": [{ "id": "a", "status": "done" }, { "id": "b", "status": "todo" }] });
    assert!(!ask("ClosingStepDone", json!({}), json!({ "step_done": { "item": "a", "succeeded": true, "remaining": "" }, "plan": plan.clone() })).0, "b is still to do");
    assert!(ask("ClosingStepDone", json!({}), json!({ "step_done": { "item": "b", "succeeded": true, "remaining": "" }, "plan": plan })).0, "b is the one it closes");
    assert!(!ask("ClosingStepDone", json!({}), json!({})).0, "no step_done");
}

#[test]
fn a_card_still_waiting_in_the_turn() {
    let carte = json!({ "from": "teammate", "kind": "approval", "id": "t:2", "text": "commande en attente" });
    let question = json!({ "from": "teammate", "kind": "question", "id": "t:3", "text": "Quel titre ?" });
    let reponse = json!({ "from": "lucie", "kind": "answer", "reply_to": "t:2", "text": "oui" });
    assert!(ask("OpenCard", json!({ "kind": "approval" }), json!({ "turn": [humaine("x"), carte.clone()] })).0);
    assert!(!ask("OpenCard", json!({ "kind": "approval" }), json!({ "turn": [humaine("x"), carte.clone(), reponse] })).0, "answered");
    assert!(!ask("OpenCard", json!({ "kind": "approval" }), json!({ "turn": [humaine("x"), question.clone()] })).0, "a question is not a command's card");
    assert!(ask("OpenCard", json!({ "kind": "any" }), json!({ "turn": [humaine("x"), question] })).0);
}

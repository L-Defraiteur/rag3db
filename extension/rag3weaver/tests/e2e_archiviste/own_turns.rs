//! **The automatic recall brings back the calling agent's own turns from its other conversations** (24 November 2026,
//! Lucie: « mes tours d'abord, ceux de mon swarm ensuite, la mémoire générale après »). A fourth source of
//! `recall_for_request`, `own_turns`: the turns signed by the agent that calls, in runs other than the one given; the
//! current run stays with the summarised-window source, so nothing comes twice and the active window never. A file of its own.

use super::*;

const ME: &str = "agent:reviewer-7f3a";
const OTHER: &str = "agent:builder-0c21";

/// Yesterday's conversation of ME holds the fact; OTHER said a near sentence in its own conversation; today's
/// conversation of ME is the current run, its active window opening at 10:30.
fn three_conversations(b: &Backend) {
    b.call_as(Some(ME), "put_turn", json!({"run": "task:yesterday", "turn": 3, "seq": 0, "at": "2026-11-22T09:00:00Z", "role": "user",
        "text": "quokkaword is the codename of the night build", "line_id": "box-7"})).expect("put_turn");
    b.call_as(Some(OTHER), "put_turn", json!({"run": "task:elsewhere", "turn": 1, "seq": 0, "at": "2026-11-22T09:30:00Z", "role": "user",
        "text": "quokkaword is the codename of the morning build"})).expect("put_turn");
    b.call_as(Some(ME), "put_turn", json!({"run": "task:today", "turn": 9, "seq": 0, "at": "2026-11-23T11:00:00Z", "role": "user",
        "text": "quokkaword again, in the active window"})).expect("put_turn");
}

fn recall(b: &Backend, agent: Option<&str>, args: Value) -> Value {
    b.call_as(agent, "recall_for_request", args).unwrap_or_else(|e| panic!("recall_for_request: {e}"))["result"].clone()
}

/// The fact said in another conversation of the same agent comes back; another agent's turn and the active window never.
#[test]
#[ignore]
fn a_fact_from_another_conversation_of_the_same_agent_comes_back() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    three_conversations(&b);
    let r = recall(&b, Some(ME), json!({"request": "what is the codename of the night build", "run": "task:today", "active_since": "2026-11-23T10:30:00Z", "top": 3}));
    let block = r["block"].as_str().unwrap_or_else(|| panic!("a block: {r}"));
    assert!(block.contains("codename of the night build"), "{block}");
    assert!(!block.contains("morning build"), "another agent's turn is not mine: {block}");
    assert!(!block.contains("in the active window"), "{block}");
    assert!(r["sources"].as_array().is_some_and(|s| s.contains(&json!("turn:box-7"))), "{r}");
    assert!(r["passages"].as_array().is_some_and(|p| p.iter().any(|p| p["tier"] == json!("own_turns"))), "{r}");
}

/// `own_turns: false` turns the source off; with no agent behind the call it is empty, without an error.
#[test]
#[ignore]
fn the_own_turns_source_can_be_turned_off_and_is_empty_without_an_agent() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    three_conversations(&b);
    let off = recall(&b, Some(ME), json!({"request": "what is the codename of the night build", "run": "task:today", "active_since": "2026-11-23T10:30:00Z", "own_turns": false}));
    assert!(!off.to_string().contains("box-7"), "{off}");
    let nobody = recall(&b, None, json!({"request": "what is the codename of the night build", "run": "task:today", "active_since": "2026-11-23T10:30:00Z"}));
    assert!(!nobody.to_string().contains("box-7"), "{nobody}");
}

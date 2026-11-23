//! **A read filters by its author, « me » by default being the agent that calls** (23 November 2026, Lucie: « je veux
//! les tours avec moi comme auteur, je veux les tours de mon swarm »). `search_turns` takes `author` (me, a key, or *)
//! and `swarm` (mine, a name, or *); « me » is the key of the agent the call is made for, given by the host, never by
//! the caller. A file of its own.

use super::*;

const ME: &str = "agent:reviewer-7f3a";
const OTHER: &str = "agent:builder-0c21";

fn two_authors(b: &Backend) {
    b.call_as(Some(ME), "put_turn", json!({"run": "task:t", "turn": 1, "seq": 0, "at": "2026-11-23T10:00:00Z", "role": "user",
        "text": "the quokkaword plan of the reviewer"})).expect("put_turn");
    b.call_as(Some(OTHER), "put_turn", json!({"run": "task:t", "turn": 2, "seq": 0, "at": "2026-11-23T10:01:00Z", "role": "user",
        "text": "the quokkaword plan of the builder"})).expect("put_turn");
}

/// `author: "me"` keeps the turns the calling agent wrote; without a filter, both.
#[test]
#[ignore]
fn author_me_keeps_the_turns_of_the_agent_that_calls() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    two_authors(&b);
    let mine = b.call_as(Some(ME), "search_turns", json!({"query": "quokkaword", "author": "me", "options": {"limit": 5}})).expect("search");
    let s = mine.to_string();
    assert!(s.contains("of the reviewer") && !s.contains("of the builder"), "{mine}");
    let all = b.call_as(Some(ME), "search_turns", json!({"query": "quokkaword", "author": "*", "options": {"limit": 5}})).expect("search");
    assert!(all.to_string().contains("of the reviewer") && all.to_string().contains("of the builder"), "{all}");
    let theirs = b.call_as(Some(ME), "search_turns", json!({"query": "quokkaword", "author": OTHER, "options": {"limit": 5}})).expect("search");
    assert!(theirs.to_string().contains("of the builder") && !theirs.to_string().contains("of the reviewer"), "{theirs}");
}

/// `author: "me"` with no agent behind the call is refused, saying why — never every turn.
#[test]
#[ignore]
fn author_me_without_an_agent_is_refused() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    two_authors(&b);
    let e = b.call_as(None, "search_turns", json!({"query": "quokkaword", "author": "me"})).err().expect("refused");
    assert!(e.contains("me") && e.contains("agent"), "{e}");
}

/// `swarm` is accepted before the swarms exist, and the answer says it filtered nothing (`swarm: null`).
#[test]
#[ignore]
fn swarm_is_accepted_and_said_null_before_the_swarms_exist() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    two_authors(&b);
    let r = b.call_as(Some(ME), "search_turns", json!({"query": "quokkaword", "swarm": "mine", "options": {"limit": 5}})).expect("search");
    assert_eq!(r["result"]["swarm"], Value::Null, "{r}");
    assert!(r["result"].as_object().is_some_and(|o| o.contains_key("swarm")), "the answer says it: {r}");
    assert!(r.to_string().contains("of the reviewer") && r.to_string().contains("of the builder"), "{r}");
}

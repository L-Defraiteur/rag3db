//! **The recall of the cards and the apports filters by its author, « me » being the agent that calls** (23 November
//! 2026, Lucie: « je veux les fiches mémoire écrites avec moi comme auteur, et je veux les fiches mémoire de mon
//! swarm »). The structured search shipped to every backend (`search_structured`) takes `author` (me, a key, or *) and
//! `swarm` (mine, a name, or *); « me » is the agent the call is made for, given by the host. A file of its own.

use super::*;

const ME: &str = "agent:reviewer-7f3a";
const OTHER: &str = "agent:builder-0c21";

fn two_cards(b: &Backend) {
    b.call_as(Some(ME), "put_fiche", json!({"record": {"name": "reviewer plan", "about": "the quokkaword plan of the reviewer", "state": "current"}})).expect("put_fiche");
    b.call_as(Some(OTHER), "put_fiche", json!({"record": {"name": "builder plan", "about": "the quokkaword plan of the builder", "state": "current"}})).expect("put_fiche");
}

/// `author: "me"` keeps the cards the calling agent wrote; an explicit key keeps that agent's; `*`, every card.
#[test]
#[ignore]
fn recall_fiches_by_author_me_keeps_the_cards_of_the_agent_that_calls() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    two_cards(&b);
    let mine = b.call_as(Some(ME), "recall_fiches", json!({"query": "quokkaword plan", "author": "me", "options": {"limit": 5}})).expect("recall");
    assert!(mine.to_string().contains("reviewer plan") && !mine.to_string().contains("builder plan"), "{mine}");
    let theirs = b.call_as(Some(ME), "recall_fiches", json!({"query": "quokkaword plan", "author": OTHER, "options": {"limit": 5}})).expect("recall");
    assert!(theirs.to_string().contains("builder plan") && !theirs.to_string().contains("reviewer plan"), "{theirs}");
    let all = b.call_as(Some(ME), "recall_fiches", json!({"query": "quokkaword plan", "author": "*", "options": {"limit": 5}})).expect("recall");
    assert!(all.to_string().contains("builder plan") && all.to_string().contains("reviewer plan"), "{all}");
}

/// The author filter adds to the caller's own filter, it does not replace it.
#[test]
#[ignore]
fn the_author_filter_adds_to_the_callers_filter() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    two_cards(&b);
    b.call_as(Some(ME), "put_fiche", json!({"record": {"name": "reviewer draft", "about": "a quokkaword draft of the reviewer", "state": "proposed"}})).expect("put_fiche");
    let r = b.call_as(Some(ME), "recall_fiches", json!({"query": "quokkaword", "author": "me",
        "options": {"limit": 5, "filter_condition": {"must": [{"field": {"key": "state", "value": "current"}}]}}})).expect("recall");
    let s = r.to_string();
    assert!(s.contains("reviewer plan") && !s.contains("reviewer draft") && !s.contains("builder plan"), "{r}");
}

/// `author: "me"` with no agent behind the call is refused, saying why — never every card.
#[test]
#[ignore]
fn recall_fiches_by_author_me_without_an_agent_is_refused() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    two_cards(&b);
    let e = b.call_as(None, "recall_fiches", json!({"query": "quokkaword", "author": "me"})).err().expect("refused");
    assert!(e.contains("me") && e.contains("agent"), "{e}");
}

/// `swarm` is accepted before the swarms exist, and filters nothing.
#[test]
#[ignore]
fn recall_apports_accepts_swarm_before_the_swarms_exist() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    two_cards(&b);
    let r = b.call_as(Some(ME), "recall_fiches", json!({"query": "quokkaword plan", "swarm": "mine", "options": {"limit": 5}})).expect("recall");
    assert!(r.to_string().contains("builder plan") && r.to_string().contains("reviewer plan"), "{r}");
    b.call_as(Some(ME), "recall_apports", json!({"query": "quokkaword", "swarm": "mine", "author": "me"})).expect("recall_apports takes them too");
}

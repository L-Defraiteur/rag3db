//! **The automatic recall of a turn** (23 November 2026, the context policy of the code agent): `recall_for_request`
//! searches, for the request of the turn, the turns of the windows already summarised, then the L1 apports, then the
//! cards, fuses them by weighted rank, and renders the best passages in full as a block labelled « automatic recall »,
//! or nothing under its threshold. `prepare: direct`: no model call. A file of its own.

use super::*;

const ME: &str = "agent:reviewer-7f3a";
const OTHER: &str = "agent:builder-0c21";

/// Turn 1 (summarised, before the active window) holds the fact; turn 5 (active, in clear) mentions the same word.
fn a_thread(b: &Backend) {
    b.call_as(Some(ME), "put_turn", json!({"run": "task:t", "turn": 1, "seq": 0, "at": "2026-11-23T10:00:00Z", "role": "user",
        "text": "quokkaword is the codename of the night build", "line_id": "box-1"})).expect("put_turn");
    b.call_as(Some(ME), "put_turn", json!({"run": "task:t", "turn": 5, "seq": 0, "at": "2026-11-23T11:00:00Z", "role": "user",
        "text": "quokkaword again, in the active window"})).expect("put_turn");
}

fn recall(b: &Backend, agent: Option<&str>, args: Value) -> Value {
    b.call_as(agent, "recall_for_request", args).unwrap_or_else(|e| panic!("recall_for_request: {e}"))["result"].clone()
}

/// The summarised turn comes back in full, labelled; the active window is never recalled; the source points to the line.
#[test]
#[ignore]
fn the_summarised_turn_comes_back_in_full_and_the_active_window_never() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    a_thread(&b);
    let r = recall(&b, Some(ME), json!({"request": "what is quokkaword", "run": "task:t", "active_since": "2026-11-23T10:30:00Z"}));
    let block = r["block"].as_str().unwrap_or_else(|| panic!("a block: {r}"));
    assert!(block.starts_with("Rappel automatique"), "{block}");
    assert!(block.contains("quokkaword is the codename of the night build"), "the turn in full: {block}");
    assert!(!block.contains("in the active window"), "a passage in clear is never recalled: {block}");
    assert_eq!(r["sources"][0], json!("turn:box-1"), "{r}");
    assert_eq!(r["mode"], json!("recent"), "{r}");
    assert_eq!(r["degraded"], Value::Null, "{r}");
    assert!(r.as_object().is_some_and(|o| o.contains_key("swarm")) && r["swarm"].is_null(), "{r}");
    // What each signal said of the passage, for the bench (and the threshold it sets).
    assert!(r["passages"][0]["signals"].as_array().is_some_and(|s| !s.is_empty()), "{r}");
    eprintln!("recall evidence: {}", r["passages"]);
}

/// Nothing found: no block, and it is not a fallback (`degraded` stays null).
#[test]
#[ignore]
fn nothing_found_gives_no_block_and_is_not_a_fallback() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    a_thread(&b);
    let r = recall(&b, Some(ME), json!({"request": "zebracornword", "run": "task:t", "active_since": "2026-11-23T10:30:00Z"}));
    assert_eq!(r["block"], Value::Null, "a nearest neighbour is not evidence: {r}");
    assert_eq!(r["degraded"], Value::Null, "{r}");
    assert_eq!(r["sources"], json!([]), "{r}");
}

/// The dense similarity of an unrelated request, measured (threshold 0): printed for the bench, never asserted.
#[test]
#[ignore]
fn the_similarity_of_an_unrelated_request_is_printed() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    a_thread(&b);
    for q in ["zebracornword", "how is the weather in Lisbon", "what is quokkaword", "the codename of the night build"] {
        let r = recall(&b, Some(ME), json!({"request": q, "run": "task:t", "active_since": "2026-11-23T10:30:00Z", "min_similarity": 0.0}));
        eprintln!("similarity probe « {q} »: {}", r["passages"]);
    }
}

/// Without `active_since` (no window summarised yet), no turn is searched: the cards only.
#[test]
#[ignore]
fn without_a_summarised_window_only_the_cards_are_searched() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    a_thread(&b);
    b.call_as(Some(ME), "put_fiche", json!({"record": {"name": "night build", "about": "quokkaword names the night build of the engine", "state": "current"}})).expect("put_fiche");
    let r = recall(&b, Some(ME), json!({"request": "what is quokkaword", "run": "task:t"}));
    let sources = r["sources"].as_array().unwrap_or_else(|| panic!("{r}"));
    assert!(!sources.is_empty() && sources.iter().all(|s| !s.as_str().unwrap_or("").starts_with("turn:")), "{r}");
    assert!(sources.contains(&json!("card:night build")), "{r}");
}

/// `author: me` keeps what the calling agent wrote.
#[test]
#[ignore]
fn author_me_recalls_only_what_the_calling_agent_wrote() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    a_thread(&b);
    let r = recall(&b, Some(OTHER), json!({"request": "what is quokkaword", "run": "task:t", "active_since": "2026-11-23T10:30:00Z", "author": "me"}));
    assert_eq!(r["block"], Value::Null, "the other agent wrote nothing: {r}");
}

/// `prepare: model` before the preparation exists: the recall runs on the request as it is, and says so.
#[test]
#[ignore]
fn prepare_model_before_it_exists_runs_direct_and_says_so() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    a_thread(&b);
    let r = recall(&b, Some(ME), json!({"request": "what is quokkaword", "run": "task:t", "active_since": "2026-11-23T10:30:00Z", "prepare": "model"}));
    assert!(r["block"].is_string(), "{r}");
    assert!(r["degraded"].as_str().is_some_and(|d| d.contains("model")), "{r}");
}

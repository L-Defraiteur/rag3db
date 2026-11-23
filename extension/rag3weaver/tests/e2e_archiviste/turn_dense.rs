//! **The turns are found by their meaning too, by default, and keep the mailbox line they came from** (23 November 2026:
//! the automatic recall searches the turns of the summarised windows by words and by meaning; a recalled turn points back
//! to its line). A file of its own.

use super::*;

/// The Turn entity declares the vector signal: search_turns finds a turn by the vector signal alone.
#[test]
#[ignore]
fn a_turn_is_found_by_the_vector_signal_alone() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    appel(&b, "put_turn", json!({"run": "task:t", "turn": 1, "seq": 0, "at": "2026-11-23T10:00:00Z", "role": "user",
        "text": "the compositor keeps its duty cycle at seventy percent"}));
    appel(&b, "put_turn", json!({"run": "task:t", "turn": 1, "seq": 1, "at": "2026-11-23T10:00:30Z", "role": "assistant",
        "text": "noted"}));
    let found = appel(&b, "search_turns", json!({"query": "the compositor keeps its duty cycle at seventy percent",
        "run": "task:t", "options": {"limit": 3, "signals": ["vector"]}}));
    let turns = found["result"]["turns"].as_array().unwrap_or_else(|| panic!("{found}"));
    assert!(!turns.is_empty() && turns[0].to_string().contains("duty cycle"), "{found}");
}

/// put_turn takes the id of the mailbox line the message came from, and the row gives it back.
#[test]
#[ignore]
fn a_turn_keeps_the_mailbox_line_it_came_from() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |_| {});
    appel(&b, "put_turn", json!({"run": "task:t", "turn": 1, "seq": 0, "at": "2026-11-23T10:00:00Z", "role": "user",
        "text": "a line of the mailbox", "line_id": "box-42"}));
    let listed = appel(&b, "list_turns", json!({"run": "task:t"}));
    assert!(listed.to_string().contains("\"line_id\":\"box-42\""), "{listed}");
}

/// A memory whose turns were written when Turn declared only bm25 reopens under the template that declares the vector
/// signal: it loads, and a turn written before is found by the vector signal alone (the long memory of a user moving
/// to the shipped template).
#[test]
#[ignore]
fn a_turn_written_before_the_vector_signal_is_found_by_it_after_a_reopening() {
    let dir = tempfile::tempdir().expect("dossier");
    {
        let b = ouvrir(dir.path(), |m| {
            m["entities"]["Turn"]["config"]["signals"] = json!(["bm25"]);
        });
        appel(&b, "put_turn", json!({"run": "task:t", "turn": 1, "seq": 0, "at": "2026-11-23T10:00:00Z", "role": "user",
            "text": "the compositor keeps its duty cycle at seventy percent"}));
    }
    let b = ouvrir(dir.path(), |_| {});
    let found = appel(&b, "search_turns", json!({"query": "the compositor keeps its duty cycle at seventy percent",
        "run": "task:t", "options": {"limit": 3, "signals": ["vector"]}}));
    let turns = found["result"]["turns"].as_array().unwrap_or_else(|| panic!("{found}"));
    assert!(!turns.is_empty() && turns[0].to_string().contains("duty cycle"), "{found}");
}

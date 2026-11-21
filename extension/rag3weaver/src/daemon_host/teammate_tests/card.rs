//! **The approval card** (page 03 of rag3weaver-archi): one card, its states, one closing. A module
//! of its own.

use super::*;
use super::super::card::{card_closing, card_state, close_card, open_card};
use super::super::teammate::TeammateFiles;

/// A card opens with its tool and arguments, goes through its states, closes once by the recall;
/// the user's answer after it is kept without effect and said; a state after the closing is
/// refused.
#[test]
fn a_card_opens_changes_state_and_closes_once() {
    let d = tempfile::tempdir().unwrap();
    let files = TeammateFiles::new(d.path().to_path_buf(), "p/t");
    let id = open_card(&files, "run_command", &json!({ "command": "git push origin lot" }), "outbound", "once").unwrap();
    let ouverture = files.mailbox().into_iter().find(|l| l["id"] == id.as_str()).expect("the card's line");
    assert_eq!(ouverture["from"], json!("teammate"));
    assert_eq!(ouverture["kind"], json!("approval"));
    assert_eq!(ouverture["tool"], json!("run_command"));
    assert_eq!(ouverture["args"], json!({ "command": "git push origin lot" }));
    assert_eq!(ouverture["class"], json!("outbound"));
    assert_eq!(ouverture["scope"], json!("once"));
    assert_eq!(ouverture["command"], json!("git push origin lot"), "a run_command card keeps its command");
    card_state(&files, &id, "recall", "looking for an authorization…", None).unwrap();
    card_state(&files, &id, "waiting_survivor", "the survivor decides in 10 min", Some(1_795_000_000_000)).unwrap();
    let etats: Vec<String> = files.mailbox().iter().filter(|l| l["kind"] == "approval_state" && l["reply_to"] == id.as_str())
        .map(|l| l["stage"].as_str().unwrap_or_default().to_string()).collect();
    assert_eq!(etats, ["recall", "waiting_survivor"]);
    assert!(card_closing(&files, &id).is_none(), "still open");
    let auth = json!({ "authorization": { "quote": "pousse quand c'est vert", "at": 1795 } });
    assert!(close_card(&files, &id, "recall", true, &auth).unwrap(), "the first closing is the one");
    let fermeture = card_closing(&files, &id).expect("closed");
    assert_eq!(fermeture["from"], json!("recall"));
    assert_eq!(fermeture["approved"], json!(true));
    assert_eq!(fermeture["authorization"]["quote"], json!("pousse quand c'est vert"));
    // The user answers after: kept without effect, and said.
    assert!(!close_card(&files, &id, "user", false, &json!({})).unwrap(), "a second closing changes nothing");
    assert_eq!(card_closing(&files, &id).unwrap()["from"], json!("recall"), "the closing stays the recall's");
    assert!(files.mailbox().iter().any(|l| l["from"] == "host" && l["reply_to"] == id.as_str() && l["text"].as_str().is_some_and(|t| t.contains("already closed") && t.contains("recall"))), "{:#?}", files.mailbox());
    assert!(card_state(&files, &id, "appeal", "too late", None).is_err(), "no state after the closing");
}

//! **Only a human's yes grants** (c7's reading of 22 Nov 2026, 22 h 45: an answer approved by a non-human entry granted
//! the card all the same; the answer's branch never read `by.kind`). A yes signed by an agent, or by no one, is kept in the
//! mailbox but grants nothing, and says why; the same card, answered by the human, runs. A module of its own.

use super::mode_auto_v1::v1_with;
use super::*;
use std::time::{Duration, Instant};

#[cfg(all(feature = "code", feature = "rag3db-native", target_os = "linux"))]
#[test]
fn a_yes_that_is_not_human_grants_nothing_and_says_why() {
    let v = v1_with(|_| (vec!["rm vieux.txt".to_string()], "Lis le README.".to_string()), &["supprime vieux.txt"]);
    let b = v.wait("the card", &|b| b.iter().any(|l| l["kind"] == "approval"));
    let card = b.iter().find(|l| l["kind"] == "approval").unwrap().clone();
    v.wait("the step waits for the supervisor", &|b| b.iter().any(|l| l["from"] == "host" && l["text"].as_str().is_some_and(|t| t.contains("en attente du superviseur"))));
    v.host
        .post_to_teammate(&v.id, "doc", "agent", "answer", None, "oui", &json!({ "reply_to": card["id"], "approved": true, "by": { "kind": "agent", "client": "mcp" } }))
        .unwrap();
    let b = v.wait("the answer", &|b| b.iter().any(|l| l["kind"] == "answer"));
    let reponse = b.iter().find(|l| l["kind"] == "answer").unwrap();
    let note = reponse["granted"].as_str().unwrap_or_default();
    assert!(note.contains("pas accordé") && note.contains("humain"), "the answer says it grants nothing: {reponse}");
    std::thread::sleep(Duration::from_secs(2));
    assert!(v.tree().join("vieux.txt").exists(), "a non-human yes ran nothing");
    v.host
        .post_to_teammate(&v.id, "doc", "lucie", "answer", None, "oui", &json!({ "reply_to": card["id"], "approved": true, "by": { "kind": "human", "client": "tty" } }))
        .unwrap();
    let fin = Instant::now() + Duration::from_secs(120);
    while v.tree().join("vieux.txt").exists() {
        assert!(Instant::now() < fin, "the human's yes runs it: {:#?}", v.files.mailbox());
        std::thread::sleep(Duration::from_millis(100));
    }
}

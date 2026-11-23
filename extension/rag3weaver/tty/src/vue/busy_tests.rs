//! **The view stays at work until the turn's `ended`** (a trial on 23 November 2026: a first « bonjour! » opened the
//! task, its run's start passed before the terminal knew the task, the model took 54 s, then the host's review 42 s;
//! the view showed the resting animation all along, read as a crash). From the request to the `ended` (or the stop),
//! the view plays the work animation, says what runs and how long it has run.
use super::*;
use serde_json::json;

fn vivante(v: &Vue, t: Instant) -> String {
    v.ligne_vivante(160, t).spans.iter().map(|s| s.content.to_string()).collect()
}

#[test]
fn a_first_turn_without_run_events_stays_at_work() {
    let mut v = Vue::new(160, true);
    v.ligne_de_boite(&json!({ "id": "b/t:0", "from": "supervisor", "kind": "request", "text": "bonjour!" }));
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "host", "kind": "report", "text": "tâche ouverte : branche agent/x" }));
    assert_ne!(v.quelle(), Some(Quelle::Repos), "the turn runs: not the resting animation");
    let l = vivante(&v, v.attente_depuis + Duration::from_secs(40));
    assert!(l.contains("le modèle réfléchit") && l.contains("40 s"), "{l}");
}

#[test]
fn after_its_run_the_turn_is_reviewed_until_ended() {
    let mut v = Vue::new(160, true);
    v.ligne_de_boite(&json!({ "id": "b/t:0", "from": "supervisor", "kind": "request", "text": "bonjour!" }));
    v.trame(&json!({ "evenement": { "kind": "RunStarted", "run": "r1" } }));
    v.trame(&json!({ "evenement": { "kind": "RunFinished", "run": "r1", "ok": true } }));
    v.tick(Instant::now() + Duration::from_secs(5));
    assert_ne!(v.quelle(), Some(Quelle::Repos), "ended not yet come: still at work");
    let l = vivante(&v, v.attente_depuis + Duration::from_secs(12));
    assert!(l.contains("relecture du tour") && l.contains("12 s"), "{l}");
    v.ligne_de_boite(&json!({ "id": "b/t:9", "from": "host", "kind": "ended", "text": "fait sans plan" }));
    assert_eq!(v.quelle(), Some(Quelle::Repos), "at the ended, back to rest");
}

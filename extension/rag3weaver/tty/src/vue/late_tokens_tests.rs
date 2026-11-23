//! **The agent's text is not cut in two when its last words come after its run's end** (an end-to-end witness on
//! 23 November 2026: 132 to 151 of 300 words reached the terminal after the `RunFinished` of their run, each subject
//! being relayed by its own thread). The view keeps the text open at the run's end and closes it after a silence.
use super::*;
use serde_json::json;

fn tok(run: &str, text: &str) -> serde_json::Value {
    json!({ "evenement": { "kind": "Token", "run": run, "node": "chat", "text": text } })
}

#[test]
fn late_tokens_continue_the_same_paragraph() {
    let mut v = Vue::new(200, true);
    v.trame(&json!({ "evenement": { "kind": "RunStarted", "run": "r1" } }));
    v.trame(&tok("r1", "Je vais lancer "));
    v.trame(&json!({ "evenement": { "kind": "RunFinished", "run": "r1", "ok": true } }));
    v.trame(&tok("r1", "le serveur avec "));
    v.trame(&tok("r1", "npm run dev."));
    v.tick(Instant::now() + Duration::from_secs(5));
    let lignes: Vec<String> = v.prendre_lignes().iter().map(|l| l.texte.clone()).collect();
    assert!(lignes.iter().any(|l| l.contains("Je vais lancer le serveur avec npm run dev.")), "one paragraph, not two: {lignes:#?}");
}

#[test]
fn the_text_closes_after_a_silence() {
    let mut v = Vue::new(200, true);
    v.trame(&json!({ "evenement": { "kind": "RunStarted", "run": "r1" } }));
    v.trame(&tok("r1", "Fini."));
    v.trame(&json!({ "evenement": { "kind": "RunFinished", "run": "r1", "ok": true } }));
    let t0 = Instant::now();
    v.tick(t0);
    assert!(v.prendre_lignes().iter().all(|l| !l.texte.contains("Fini.")), "still open right after the end");
    v.tick(t0 + Duration::from_secs(5));
    assert!(v.prendre_lignes().iter().any(|l| l.texte.contains("Fini.")), "closed after a silence");
}

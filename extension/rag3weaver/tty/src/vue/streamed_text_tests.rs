//! **The agent's text is seen as it is written** (23 November 2026: « on ne le voit rien dire en streaming »): a
//! teammate run's `Token` frames, on the `tokens` subject of the task's backend, pass the task's filter, grow in the
//! living zone while the model writes, and are written once in the history: the host's copy at the end of the step
//! (`origin: host_copy`) does not write them again.
use super::*;
use serde_json::json;

#[test]
fn a_teammate_runs_tokens_grow_live_and_are_written_once() {
    let task = "tache-abc";
    let frame = |seq: u64, text: &str| json!({ "sujet": "tokens", "backend": task, "evenement": { "kind": "Token", "event": "token", "run": "agent-1", "node": "chat", "seq": seq, "text": text, "count": 1 } });
    let f1 = frame(1, "Je vais ");
    assert!(crate::tui::frame_is_the_tasks(&f1, Some(task), "depot"), "a token of the task's run passes the task's filter");
    let mut v = Vue::new(120, true);
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "explique" }));
    v.trame(&json!({ "evenement": { "kind": "RunStarted", "run": "agent-1" } }));
    v.trame(&json!({ "evenement": { "kind": "LlmCall" } }));
    v.trame(&f1);
    assert!(v.en_cours.contains("Je vais"), "the text grows in the living zone: {:?}", v.en_cours);
    v.trame(&frame(2, "lancer le serveur avec npm run dev."));
    assert!(v.en_cours.contains("Je vais lancer le serveur"), "{:?}", v.en_cours);
    v.trame(&json!({ "evenement": { "kind": "RunFinished", "run": "agent-1", "ok": true } }));
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "teammate", "kind": "report", "origin": "host_copy", "text": "Je vais lancer le serveur avec npm run dev." }));
    v.ligne_de_boite(&json!({ "id": "b/t:3", "from": "host", "kind": "ended", "text": "fait" }));
    let h: Vec<String> = v.prendre_lignes().iter().map(|l| l.texte.clone()).collect();
    assert_eq!(h.iter().filter(|l| l.contains("npm run dev")).count(), 1, "written once: {h:#?}");
}

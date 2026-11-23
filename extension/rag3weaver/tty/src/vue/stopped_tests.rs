//! **A stopped turn says so** (a trial of the stop on 23 November 2026: after Échap, the view said « il a fini sans
//! répondre »). The host's `ended` carries `stopped: true`, `by` and what landed; the view says « tour arrêté » with
//! what is already in the folder, never « sans répondre », whatever the sentence of `text`.
use super::*;
use serde_json::json;

fn texts(v: &mut Vue) -> String {
    v.prendre_lignes().iter().map(|l| l.texte.clone()).collect::<Vec<_>>().join("\n")
}

#[test]
fn a_stopped_turn_says_what_landed_and_is_not_unanswered() {
    let mut v = Vue::new(160, true);
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "écris notes.md puis attends" }));
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "host", "kind": "ended", "text": "n'importe quelle phrase", "stopped": true, "by": "human", "landed": ["notes.md"] }));
    let t = texts(&mut v);
    assert!(t.contains("tour arrêté : 1 fichier(s) déjà dans ton dossier"), "{t}");
    assert!(!v.ligne_d_etat().contains("sans répondre"), "{}", v.ligne_d_etat());
}

#[test]
fn a_stopped_turn_with_nothing_landed_says_so() {
    let mut v = Vue::new(160, true);
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "x" }));
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "host", "kind": "ended", "text": "", "stopped": true, "by": "human", "landed": [] }));
    let t = texts(&mut v);
    assert!(t.contains("tour arrêté : rien n'a encore changé dans ton dossier"), "{t}");
    assert!(!v.ligne_d_etat().contains("sans répondre"), "{}", v.ligne_d_etat());
}

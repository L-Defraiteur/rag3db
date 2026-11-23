//! **Échap stops the turn, never the session** (23 November 2026): during a turn, from the request sent to its end,
//! Échap (and Ctrl-G) stop it and the view stays with its history; at rest they send nothing, since a stop sent with
//! no turn running would stop the next one before it starts.
use super::*;
use serde_json::json;

#[test]
fn escape_stops_a_turn_and_does_nothing_at_rest() {
    let mut v = Vue::new(120, true);
    assert_eq!(v.touche(Touche::Echap), None, "at rest, Échap sends no stop");
    assert_eq!(v.touche(Touche::CtrlG), None, "nor does Ctrl-G");
    for c in "fais ça".chars() {
        v.touche(Touche::Car(c));
    }
    assert!(matches!(v.touche(Touche::Entree), Some(Action::Envoyer(..))));
    assert_eq!(v.touche(Touche::Echap), Some(Action::Arreter), "sent, not started yet: Échap stops it");
    v.trame(&json!({ "evenement": { "kind": "LlmCall" } }));
    assert_eq!(v.touche(Touche::Echap), Some(Action::Arreter), "while it thinks");
    v.ligne_de_boite(&json!({ "id": "b/t:9", "from": "host", "kind": "ended", "text": "", "stopped": true, "by": "human", "landed": [] }));
    assert_eq!(v.touche(Touche::Echap), None, "the turn ended: Échap is at rest again");
    assert!(v.prendre_lignes().iter().any(|l| l.texte.contains("fais ça")), "the history stays");
}

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

/// **Échap empties the input first, and one press is one action** (23 Nov. 2026): with text typed and a turn
/// running, Échap held down empties the input and the turn goes on, whatever the keyboard's repeats; a second
/// press, after the key was released, stops the turn.
#[test]
fn escape_empties_the_input_first_and_a_held_key_is_one_press() {
    let mut v = Vue::new(120, true);
    for c in "go".chars() {
        v.touche(Touche::Car(c));
    }
    assert!(matches!(v.touche(Touche::Entree), Some(Action::Envoyer(..))));
    v.trame(&json!({ "evenement": { "kind": "LlmCall" } }));
    for c in "et puis".chars() {
        v.touche(Touche::Car(c));
    }
    let t0 = Instant::now();
    assert_eq!(v.touche_a(Touche::Echap, t0), None, "the first press empties the input");
    assert!(v.saisie.est_vide());
    // Held: the terminal repeats after ~600 ms, then every ~30 ms; none of it stops the turn.
    let mut t = t0 + Duration::from_millis(600);
    while t < t0 + Duration::from_millis(1500) {
        assert_eq!(v.touche_a(Touche::Echap, t), None, "a repeat of the held key at {:?}", t - t0);
        t += Duration::from_millis(30);
    }
    // Released, then pressed again.
    assert_eq!(v.touche_a(Touche::Echap, t + Duration::from_millis(800)), Some(Action::Arreter), "a new press stops the turn");
}

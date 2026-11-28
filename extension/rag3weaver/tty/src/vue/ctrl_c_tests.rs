//! **Ctrl-C stops the turn, not the session** (a trial of the view, 23 November 2026: Ctrl-C during a turn
//! closed the whole session). While the teammate works, Ctrl-C stops it; at rest, a first Ctrl-C says so and a second
//! within two seconds quits; a typed input is emptied first, as before.
use super::*;
use serde_json::json;

#[test]
fn ctrl_c_during_a_turn_stops_the_turn() {
    let mut v = Vue::new(120, true);
    v.changer(Etat::Reflechit);
    assert_eq!(v.touche(Touche::CtrlC), Some(Action::Arreter), "while it thinks");
    v.changer(Etat::Agit("read_file".into()));
    assert_eq!(v.touche(Touche::CtrlC), Some(Action::Arreter), "while it acts");
}

#[test]
fn ctrl_c_at_rest_warns_then_quits_on_a_second_press() {
    let mut v = Vue::new(120, true);
    v.changer(Etat::Attend);
    assert_eq!(v.touche(Touche::CtrlC), None, "a first Ctrl-C at rest does not quit");
    assert!(v.prendre_lignes().iter().any(|l| l.texte.contains("Ctrl-C encore pour quitter")), "it says how to quit");
    assert_eq!(v.touche(Touche::CtrlC), Some(Action::Quitter), "a second one quits");
    // A typed input is emptied first, as before.
    let mut w = Vue::new(120, true);
    w.changer(Etat::Reflechit);
    for c in "abc".chars() {
        w.touche(Touche::Car(c));
    }
    assert_eq!(w.touche(Touche::CtrlC), None);
    assert!(w.saisie.est_vide());
}

/// **A turn whose work landed is not « fini sans répondre »** (same trial: the work landed in the folder, the agent
/// wrote no text, and the view said « il a fini sans répondre »).
#[test]
fn a_turn_whose_work_landed_is_not_said_unanswered() {
    let mut v = Vue::new(160, true);
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "ajoute --lignes" }));
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "host", "kind": "merged", "text": "2 fichier(s) dans ton dossier /d (1 commit(s) de la branche agent/t)" }));
    v.ligne_de_boite(&json!({ "id": "b/t:3", "from": "host", "kind": "ended", "text": "fait sans plan" }));
    let s = v.ligne_d_etat();
    assert!(!s.contains("sans répondre"), "{s}");
}

/// **Quitting always works** (Lucie, 24 November 2026: after a restart of the host, the view never saw its turn end,
/// and « arrêt demandé » came at each Ctrl-C, five times, without quitting). During a turn, the first Ctrl-C stops it,
/// the second says how to quit, the third within a few seconds quits; Ctrl-D and /quit quit whatever the state.
#[test]
fn quitting_always_works_even_during_a_turn_that_never_ends() {
    let mut v = Vue::new(120, true);
    v.changer(Etat::Reflechit);
    assert_eq!(v.touche(Touche::CtrlC), Some(Action::Arreter), "the first stops the turn");
    assert_eq!(v.touche(Touche::CtrlC), None, "the turn did not end: the second says how to quit");
    assert_eq!(v.touche(Touche::CtrlC), Some(Action::Quitter), "the third quits");
    let mut d = Vue::new(120, true);
    d.changer(Etat::Agit("run_command".into()));
    assert_eq!(d.touche(Touche::CtrlD), Some(Action::Quitter), "Ctrl-D quits during a turn");
    let mut q = Vue::new(120, true);
    q.changer(Etat::Reflechit);
    for c in "/quit".chars() {
        q.touche(Touche::Car(c));
    }
    // `/quit` is the command that quits (`tui::executer` ends the session on it), whatever the state.
    assert_eq!(q.touche(Touche::Entree), Some(Action::Commande(crate::commandes::Commande::Quitter)), "/quit quits during a turn");
}

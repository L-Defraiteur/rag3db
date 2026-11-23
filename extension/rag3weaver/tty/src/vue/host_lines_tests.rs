//! **What changed is read in the view as in `--plain`** (a trial of the view, 23 November 2026): the
//! host's `turn_changes` and the line on what commands produced are shown without `--steps`; a turn whose teammate
//! wrote no prose answers with its done step.
use super::*;
use serde_json::json;

fn texts(v: &mut Vue) -> String {
    v.prendre_lignes().iter().map(|l| l.texte.clone()).collect::<Vec<_>>().join("\n")
}

#[test]
fn the_turn_changes_and_what_commands_produced_are_shown_without_steps() {
    let mut v = Vue::new(160, true);
    assert!(!v.montrer_etapes);
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "host", "kind": "turn_changes", "text": "2 fichier(s) modifié(s) dans ce tour : a.py, b.py", "files": ["a.py", "b.py"] }));
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "host", "kind": "report", "text": "1 produit de commande laissé hors du commit : __pycache__/", "uncommitted": 1 }));
    // The host's other words stay muted.
    v.ligne_de_boite(&json!({ "id": "b/t:3", "from": "host", "kind": "report", "text": "tâche ouverte : x" }));
    let t = texts(&mut v);
    assert!(t.contains("modifié(s) dans ce tour : a.py, b.py"), "{t}");
    assert!(t.contains("hors du commit : __pycache__/"), "{t}");
    assert!(!t.contains("tâche ouverte"), "{t}");
}

#[test]
fn a_turn_without_prose_answers_with_its_done_step() {
    let mut v = Vue::new(160, true);
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "ajoute --lignes" }));
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "teammate", "kind": "step_done", "text": "fait : Ajout de l'option --lignes dans word_count.py" }));
    v.ligne_de_boite(&json!({ "id": "b/t:3", "from": "host", "kind": "merged", "text": "1 fichier(s) dans ton dossier /d" }));
    v.ligne_de_boite(&json!({ "id": "b/t:4", "from": "host", "kind": "ended", "text": "fait" }));
    let t = texts(&mut v);
    assert!(t.contains("Ajout de l'option --lignes"), "{t}");
    // A turn that spoke does not repeat its step.
    let mut w = Vue::new(160, true);
    w.ligne_de_boite(&json!({ "id": "c/t:1", "from": "supervisor", "kind": "request", "text": "x" }));
    w.ligne_de_boite(&json!({ "id": "c/t:2", "from": "teammate", "kind": "report", "text": "J'ai fait x." }));
    w.ligne_de_boite(&json!({ "id": "c/t:3", "from": "teammate", "kind": "step_done", "text": "fait : x fait" }));
    w.ligne_de_boite(&json!({ "id": "c/t:4", "from": "host", "kind": "ended", "text": "fait" }));
    let t = texts(&mut w);
    assert!(t.contains("J'ai fait x.") && !t.contains("x fait"), "{t}");
}

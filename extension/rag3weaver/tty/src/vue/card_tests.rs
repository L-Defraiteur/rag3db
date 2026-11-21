//! **The approval card in the terminal** (the mode auto's gate, archi's page 07 of 21 November 2026; Lucie, 10 h 0x:
//! one card, never hidden, its state changing, and one closing). The card opens with an `approval` (tool, arguments,
//! class, scope); each `approval_state` line of the host replaces its state on screen, written nowhere; one `answer`
//! closes it and leaves one history line that says who decided.
use super::*;
use serde_json::{json, Value};

fn line(n: usize, from: &str, kind: &str, extra: Value) -> Value {
    let mut l = json!({ "id": format!("a1/doc:{n}"), "at": n, "from": from, "kind": kind, "text": format!("ligne {n}") });
    for (k, v) in extra.as_object().cloned().unwrap_or_default() {
        l[k] = v;
    }
    l
}

fn written(v: &mut Vue) -> Vec<String> {
    v.prendre_lignes().into_iter().map(|l| l.texte).collect()
}

fn opened(v: &mut Vue) {
    v.ligne_de_boite(&line(1, "teammate", "approval", json!({ "tool": "browser_click", "args": { "tab_id": "7", "ref": "e4" }, "class": "outbound", "scope": "once" })));
}

/// The card names the tool and its arguments (not only a command), and its scope.
#[test]
fn the_card_names_the_tool_its_arguments_and_its_scope() {
    let mut v = Vue::new(160, true);
    opened(&mut v);
    let s = v.ligne_d_etat();
    assert!(s.contains("attend ton accord : browser_click tab_id=7 ref=e4"), "{s}");
    assert!(s.contains("pour cet envoi seulement"), "{s}");
    assert!(s.contains("[ Oui ]"), "{s}");
}

/// Each state line of the host replaces the card's state, and the card stays, answerable, with nothing written.
#[test]
fn a_state_line_changes_the_card_without_hiding_it_or_writing_anything() {
    let mut v = Vue::new(160, true);
    opened(&mut v);
    written(&mut v);
    v.ligne_de_boite(&line(2, "host", "approval_state", json!({ "reply_to": "a1/doc:1", "stage": "recall", "text": "cherche une autorisation…" })));
    assert!(v.ligne_d_etat().contains("cherche une autorisation…"), "{}", v.ligne_d_etat());
    v.ligne_de_boite(&line(3, "host", "approval_state", json!({ "reply_to": "a1/doc:1", "stage": "appeal", "text": "l'agent fait valoir : tu l'as demandé à 10 h 02" })));
    let s = v.ligne_d_etat();
    assert!(s.contains("l'agent fait valoir") && !s.contains("cherche une autorisation"), "the state is replaced: {s}");
    assert!(s.contains("browser_click") && s.contains("[ Oui ]"), "the card stays, answerable: {s}");
    assert_eq!(written(&mut v), Vec::<String>::new(), "a state line writes nothing to the history");
    assert!(matches!(v.touche(Touche::Car('o')), Some(Action::Repondre { accord: Some(true), .. })), "the user answers at any time");
}

/// The survivor's wait is counted down from `until`.
#[test]
fn the_survivor_wait_is_counted_down() {
    assert_eq!(crate::boite::countdown(1_000_000 + 540_000, 1_000_000), "9 min");
    assert_eq!(crate::boite::countdown(1_000_000 + 45_000, 1_000_000), "45 s");
    assert_eq!(crate::boite::countdown(1_000_000, 1_000_500), "maintenant");
}

/// The one closing line says who decided: the user, an authorization recalled (its quote), the agent's appeal, or
/// the verifier (its model, its reasons, its alternative).
#[test]
fn the_closing_line_says_who_decided() {
    let close = |answer: Value| {
        let mut v = Vue::new(200, true);
        opened(&mut v);
        written(&mut v);
        v.ligne_de_boite(&line(9, "host", "answer", answer));
        let w = written(&mut v);
        assert_eq!(w.len(), 1, "one closing line: {w:?}");
        assert!(!v.ligne_d_etat().contains("attend ton accord"), "closed, the card leaves: {}", v.ligne_d_etat());
        w[0].clone()
    };
    let by_user = close(json!({ "reply_to": "a1/doc:1", "approved": true, "from": "user" }));
    assert!(by_user.ends_with("accordé par toi : browser_click tab_id=7 ref=e4"), "{by_user}");
    let by_recall = close(json!({ "reply_to": "a1/doc:1", "approved": true, "from": "recall", "authorization": { "quote": "clique où tu veux sur la doc", "at": 1_795_273_500_000u64 } }));
    assert!(by_recall.contains("validé par ton autorisation, le ") && by_recall.contains(" : « clique où tu veux sur la doc »"), "{by_recall}");
    let by_appeal = close(json!({ "reply_to": "a1/doc:1", "approved": true, "from": "appeal" }));
    assert!(by_appeal.contains("accordé sur l'appel de l'agent"), "{by_appeal}");
    let by_verifier = close(json!({ "reply_to": "a1/doc:1", "approved": false, "from": "verifier", "model": "gemini-3.8-flash", "reasons": "la page n'est pas dans la demande", "alternative": { "text": "lis la page d'abord", "class": "read" } }));
    assert!(by_verifier.contains("refusé par le vérificateur (gemini-3.8-flash) : la page n'est pas dans la demande"), "{by_verifier}");
    assert!(by_verifier.contains("il propose : lis la page d'abord"), "{by_verifier}");
}

/// **The date of an authorization** (archi: ms since the epoch, said in local time): « le 21 nov. à 16 h 05 ».
#[test]
fn an_authorization_date_is_said_in_local_words() {
    assert_eq!(crate::boite::said_date(21, 10, 16, 5), "le 21 nov. à 16 h 05");
    assert_eq!(crate::boite::said_date(1, 0, 9, 0), "le 1 janv. à 9 h 00");
}

/// **An answer after the card closed is said without effect** (archi: the host keeps it and writes « card … already
/// closed by … »): the recall closed the card, the user's late answer leaves a line that says so.
#[test]
fn a_late_answer_is_said_without_effect() {
    let mut v = Vue::new(200, true);
    opened(&mut v);
    v.ligne_de_boite(&line(2, "recall", "answer", json!({ "reply_to": "a1/doc:1", "approved": true, "authorization": { "quote": "vas-y" } })));
    written(&mut v);
    v.ligne_de_boite(&line(3, "user", "answer", json!({ "reply_to": "a1/doc:1", "approved": false, "text": "non" })));
    v.ligne_de_boite(&line(4, "host", "report", json!({ "reply_to": "a1/doc:1", "late_from": "user", "late_approved": false, "text": "card a1/doc:1 already closed by recall: the closing of user is kept without effect" })));
    let w = written(&mut v);
    assert!(w.iter().any(|l| l.contains("sans effet") && l.contains("déjà close par ton autorisation rappelée")), "{w:?}");
}

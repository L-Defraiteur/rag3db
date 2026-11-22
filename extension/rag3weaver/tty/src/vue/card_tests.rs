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
    assert!(s.contains("attend ton accord : browser_click ref=e4 tab_id=7"), "the arguments in the order of their names: {s}");
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
    assert!(by_user.ends_with("accordé par toi : browser_click ref=e4 tab_id=7"), "the scope is said on the open card only: {by_user}");
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

/// **The card says its class in plain words** (archi, 21 November 2026: so that Lucie sees why it opens); without a
/// class, nothing changes.
#[test]
fn the_card_says_its_class_in_plain_words() {
    let said = |class: &str| {
        let mut v = Vue::new(200, true);
        v.ligne_de_boite(&line(1, "teammate", "approval", json!({ "tool": "run_command", "args": { "command": "rm build.log" }, "command": "rm build.log", "class": class, "scope": "once" })));
        v.ligne_d_etat()
    };
    assert!(said("destroy_local").contains("supprime un fichier qui n'est sur aucun distant"), "{}", said("destroy_local"));
    assert!(said("outbound").contains("envoie hors de la machine ou installe des paquets"), "{}", said("outbound"));
    assert!(said("database").contains("touche une base"), "{}", said("database"));
    assert!(said("system").contains("touche le système"), "{}", said("system"));
    let mut v = Vue::new(200, true);
    v.ligne_de_boite(&line(1, "teammate", "approval", json!({ "command": "cargo build" })));
    assert!(v.ligne_d_etat().contains("attend ton accord : cargo build  [ Oui ]"), "without a class, nothing changes: {}", v.ligne_d_etat());
}

fn root_card(v: &mut Vue, args: Value) {
    v.ligne_de_boite(&line(1, "teammate", "approval", json!({ "tool": "run_command", "args": args, "command": "ls /home/lucied/projet", "class": "open_root", "scope": "once" })));
}

/// **A root asked for is said with its mode** (archi's judge-all, 22 November 2026): the card names the root and
/// whether it is read or written.
#[test]
fn an_open_root_card_says_the_root_and_its_mode() {
    let mut v = Vue::new(220, true);
    root_card(&mut v, json!({ "command": "ls /home/lucied/projet", "root": "/home/lucied/projet", "mode": "write" }));
    let s = v.ligne_d_etat();
    assert!(s.contains("demande la racine /home/lucied/projet en écriture"), "{s}");
    let mut r = Vue::new(220, true);
    root_card(&mut r, json!({ "command": "ls /home/lucied/projet", "root": "/home/lucied/projet", "mode": "read" }));
    assert!(r.ligne_d_etat().contains("en lecture"), "{}", r.ligne_d_etat());
}

/// **A wide root's warning jumps to the eye** (archi, c7: « un avertissement de racine large doit sauter aux yeux »):
/// it is on the card, in a segment of its own, in bright red.
#[test]
fn a_wide_root_warning_is_said_in_red() {
    let mut v = Vue::new(260, true);
    root_card(&mut v, json!({ "command": "ls /home/lucied", "root": "/home/lucied", "mode": "read", "warning": "cette racine contient aussi : .ssh, git_workspaces" }));
    let line = v.ligne_vivante(260, v.depuis);
    let warn = line.spans.iter().find(|s| s.content.contains("cette racine contient aussi")).unwrap_or_else(|| panic!("the warning is on the card: {line:?}"));
    assert_eq!(warn.style.fg, Some(ratatui::style::Color::LightRed), "in bright red, apart: {warn:?}");
    assert!(warn.content.starts_with("⚠ "), "{warn:?}");
}

/// **A protected folder is said, with the agent's reason, and opened for this command only.**
#[test]
fn a_protected_card_says_it_opens_for_this_command_only_and_why() {
    let mut v = Vue::new(260, true);
    v.ligne_de_boite(&line(1, "teammate", "approval", json!({ "tool": "run_command", "args": { "command": "cat ~/.local/share/rag3weaver/daemon.json", "reason": "lire le port de l'hôte" }, "command": "cat ~/.local/share/rag3weaver/daemon.json", "class": "protected", "scope": "once" })));
    let s = v.ligne_d_etat();
    assert!(s.contains("ouvre un dossier protégé, pour cette commande seulement"), "{s}");
    assert!(s.contains("« lire le port de l'hôte »"), "{s}");
}

/// **A root granted is said as lasting** (archi, 22 November 2026: unlike a line's grant, it outlives the command, and
/// Lucie must see it); a protected folder is opened for this command only.
#[test]
fn a_granted_root_is_said_lasting_and_a_protected_grant_for_this_command_only() {
    let mut v = Vue::new(260, true);
    root_card(&mut v, json!({ "command": "ls /home/lucied/projet", "root": "/home/lucied/projet", "mode": "write" }));
    written(&mut v);
    v.ligne_de_boite(&line(2, "user", "answer", json!({ "reply_to": "a1/doc:1", "approved": true })));
    let w = written(&mut v);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(w[0].ends_with("racine accordée par toi : /home/lucied/projet (écriture), durable"), "{w:?}");
    let mut p = Vue::new(260, true);
    p.ligne_de_boite(&line(1, "teammate", "approval", json!({ "tool": "run_command", "args": { "command": "cat secret.env", "reason": "lire la clé" }, "command": "cat secret.env", "class": "protected", "scope": "once" })));
    written(&mut p);
    p.ligne_de_boite(&line(2, "user", "answer", json!({ "reply_to": "a1/doc:1", "approved": true })));
    let w = written(&mut p);
    assert!(w[0].ends_with("accordé par toi, pour cette commande seulement : cat secret.env"), "{w:?}");
}

/// **A root the judge granted is said** (archi's host line `root_granted`, the one form: judge-grant-said):
/// « › racine accordée par le juge : … ».
#[test]
fn a_root_granted_by_the_judge_is_said() {
    let mut v = Vue::new(260, true);
    v.ligne_de_boite(&line(1, "host", "root_granted", json!({ "root": "/home/lucied/projet", "mode": "read", "by": "judge", "quote": "regarde mon projet", "text": "racine accordée par le juge : /home/lucied/projet (read), sur ta phrase « regarde mon projet »" })));
    let w = written(&mut v);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(w[0].contains("racine accordée par le juge : /home/lucied/projet (lecture), sur ta phrase « regarde mon projet »"), "{w:?}");
}

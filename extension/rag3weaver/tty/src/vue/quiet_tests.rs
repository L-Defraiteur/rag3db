//! **By default the view shows what speaks to the user** (a trial on 23 November 2026: « les grosses balises hôte etc
//! je m'en fous, trop de bruit »): the agent's text without a `[équipier · …]` prefix, one short human line for what
//! landed (the list under Ctrl-O), no host plumbing in the thread, no tool line left on screen once the turn ended.
//! With `--steps`, everything stays as it was.
use super::*;
use serde_json::json;

fn texts(v: &mut Vue) -> Vec<String> {
    v.prendre_lignes().iter().map(|l| l.texte.clone()).collect()
}

#[test]
fn the_agents_text_has_no_prefix() {
    let mut v = Vue::new(160, true);
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "x" }));
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "teammate", "kind": "report", "text": "Voici la réponse." }));
    let t = texts(&mut v);
    assert!(t.iter().any(|l| l == "Voici la réponse."), "{t:#?}");
    assert!(t.iter().all(|l| !l.contains("[équipier")), "{t:#?}");
}

#[test]
fn what_landed_is_one_short_line_and_the_list_is_under_ctrl_o() {
    let mut v = Vue::new(160, true);
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "x" }));
    let fichiers: Vec<String> = (1..=17).map(|i| format!("src/f{i}.ts")).collect();
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "host", "kind": "report", "text": "1 fichier(s) produits par des commandes ne sont pas commités (package-lock.json) : ajoute un .gitignore", "uncommitted": 1 }));
    v.ligne_de_boite(&json!({ "id": "b/t:3", "from": "host", "kind": "merged", "text": "17 fichier(s) dans ton dossier /home/x/essai-2 (1 commit(s) de la branche agent/t)", "files": 17 }));
    v.ligne_de_boite(&json!({ "id": "b/t:4", "from": "host", "kind": "turn_changes", "text": format!("17 fichier(s) modifié(s) dans ce tour : {}", fichiers.join(", ")), "files": fichiers }));
    v.ligne_de_boite(&json!({ "id": "b/t:5", "from": "teammate", "kind": "step_done", "text": "fait : Le projet est créé." }));
    v.ligne_de_boite(&json!({ "id": "b/t:6", "from": "host", "kind": "ended", "text": "fait sans plan" }));
    let t = texts(&mut v);
    assert!(t.iter().any(|l| l == "✓ 17 fichier(s) dans ton dossier"), "{t:#?}");
    assert!(t.iter().any(|l| l == "Le projet est créé."), "the done step without « fait : » nor a prefix: {t:#?}");
    assert!(t.iter().all(|l| !l.contains("[hôte") && !l.contains("[équipier") && !l.contains("src/f3.ts") && !l.contains("package-lock")), "{t:#?}");
    let n = v.openable_count();
    let tout: String = (0..n).flat_map(|i| v.openable_lines(i)).map(|l| l.texte).collect::<Vec<_>>().join("\n");
    assert!(tout.contains("src/f3.ts") && tout.contains("package-lock.json"), "the detail stays under Ctrl-O: {tout}");
    assert!(v.discret.is_none(), "no tool line left once the turn ended: {:?}", v.discret);
}

#[test]
fn with_the_steps_shown_everything_stays() {
    let mut v = Vue::new(160, true);
    v.montrer_etapes = true;
    v.ligne_de_boite(&json!({ "id": "b/t:1", "from": "supervisor", "kind": "request", "text": "x" }));
    v.ligne_de_boite(&json!({ "id": "b/t:2", "from": "teammate", "kind": "report", "text": "Voici la réponse." }));
    let t = texts(&mut v);
    assert!(t.iter().any(|l| l.contains("[équipier") && l.contains("Voici la réponse.")), "{t:#?}");
}

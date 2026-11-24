//! **A reopened turn keeps its order** (a trial on 24 November 2026: at `--continue`, « ✓ 1 fichier(s) dans ton dossier »
//! came before the words of the turn that wrote it — the host writes `merged` before the end of step copy). In a turn
//! whose text was not streamed (a reopening), the landing line is said after the turn's words; live, it stays in place.
use super::*;
use serde_json::json;

#[test]
fn a_reopened_turn_says_its_words_before_its_landing() {
    let mut v = Vue::new(200, true);
    for l in [
        json!({ "id": "b/t:1", "from": "supervisor", "kind": "instruction", "text": "ajoute une ligne au README" }),
        json!({ "id": "b/t:2", "from": "teammate", "kind": "step_done", "text": "fait : ligne ajoutée" }),
        json!({ "id": "b/t:3", "from": "host", "kind": "merged", "files": 1, "text": "1 fichier(s) commités dans ton dossier" }),
        json!({ "id": "b/t:4", "from": "teammate", "kind": "report", "origin": "host_copy", "text": "La ligne a bien été ajoutée au README." }),
        json!({ "id": "b/t:5", "from": "host", "kind": "turn_changes", "text": "1 fichier(s) modifié(s) dans ce tour : README.md" }),
        json!({ "id": "b/t:6", "from": "host", "kind": "ended", "text": "fait sans plan" }),
    ] {
        v.ligne_de_boite(&l);
    }
    let h: Vec<String> = v.prendre_lignes().iter().map(|l| l.texte.clone()).collect();
    let mots = h.iter().position(|t| t.contains("La ligne a bien été ajoutée")).unwrap_or_else(|| panic!("the words are said: {h:#?}"));
    let pose = h.iter().position(|t| t.contains("✓ 1 fichier(s) dans ton dossier")).unwrap_or_else(|| panic!("the landing is said: {h:#?}"));
    assert!(mots < pose, "the turn's words come before its landing: {h:#?}");
}

//! B4, le Decide de l'archiviste en parts.
//!
//! Un fichier à ce lot (règle de c7, 21 nov. 2026) : un témoin neuf ne s'écrit jamais dans le corps d'un fichier de
//! tests partagé.

use super::*;

/// **B4, the Decide in parts, the stable first** (19 November 2026): the text of a section's Decide is its parts — what
/// precedes (stable for the whole document), the section, the candidate fiches by their declared template — with no
/// prose line around them; the labels and their meaning are in the instruction.
#[test]
#[ignore]
fn the_decide_prompt_is_its_parts_stable_first() {
    let serveur = FakeServer::start_sequence(vec![etiquette("candidate_1")]);
    let adresse = serveur.url.clone();
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path(), |m| {
        m["models"]["llm"] = json!({"provider": "compatible", "protocol": "openai", "model": "faux", "address": adresse});
    });
    appel(&b, "put_fiche", json!({"record": {"name": FICHE, "about": "Comment le moteur rejoue son journal à l'ouverture.", "state": "current"}}));
    appel(&b, "archive", json!({"titre": "Un doc", "contenu": "# Un doc\n\n## Le journal\n\nLe WAL se rejoue avant toute écriture.\n", "jour": "2026-11-20", "context": "CONTEXTMARK"}));
    let first: Value = serde_json::from_str(&serveur.request.recv().unwrap()).unwrap();
    let p = user_prompt(&first);
    assert!(p.starts_with("## Ce qui précède (déjà rangé, ne pas ranger) :\n\nCONTEXTMARK\n\n## Section « Le journal »\n\n"), "{p}");
    assert!(p.contains(&format!("\n\n## Fiches candidates\n\n- candidate_1 : « {FICHE} » : Comment le moteur rejoue son journal")), "{p}");
    assert!(!p.contains("Réponds par"), "no prose line: {p}");
}

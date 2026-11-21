//! **Un dossier en dessous de la base garde le rebâti** (lucivy 4.4, (A)). Revenir n'a de sens que vers une génération
//! gardée au-dessus de laquelle le dossier se trouve. Un dossier plus vieux que la base (une copie restaurée, un
//! dossier d'une autre passe) n'a rien vers quoi avancer : il est jeté et rebâti depuis les lignes, comme avant 4.4.
#![cfg(all(feature = "rag3db-native", feature = "code"))]

use crate::e2e_generations_commun as g;

fn copier(de: &std::path::Path, vers: &std::path::Path) {
    let statut = std::process::Command::new("cp").args(["-a"]).arg(de).arg(vers).status().expect("cp");
    assert!(statut.success(), "copier {} vers {}", de.display(), vers.display());
}

#[test]
#[ignore]
fn a_folder_behind_the_base_is_rebuilt() {
    let d = g::dossier("en-retard");
    let base = d.join("base.rag3db");
    let fts = d.join("base.rag3db.fts");
    let (statut, sortie) = g::lancer("premier", &base, &[]);
    assert!(statut.success(), "{sortie}");
    let vieux = d.join("fts-du-premier");
    copier(&fts, &vieux);
    // Une seconde passe forcée : la base avance ses générations.
    let (statut, sortie) = g::lancer("second", &base, &[]);
    assert!(statut.success(), "{sortie}");
    std::fs::remove_dir_all(&fts).unwrap();
    std::fs::rename(&vieux, &fts).unwrap();
    let avant = g::generations(&base);
    assert!(avant.iter().any(|(_, en_base, disque)| disque < en_base), "le dossier restauré est en dessous de la base : {avant:?}");

    let (statut, sortie) = g::lancer("repreneur", &base, &[]);
    assert!(statut.success(), "{sortie}");
    assert!(sortie.contains(g::REBATI), "un dossier en dessous de la base est jeté et rebâti :\n{sortie}");
    assert_eq!(g::comptes("repreneur", &sortie), g::temoin("en-retard-temoin"), "les comptes d'une passe sans retard");
    let _ = std::fs::remove_dir_all(&d);
}

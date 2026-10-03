//! Sonde : la liste exacte des fichiers que l'index du dépôt entier passe à
//! codeparsers — `WorkingTree::list`, `verdict(p, 0)` puis `verdict(p,
//! taille)` == `Code`, comme `analyze_with`. Écrite dans `SONDE_LISTE`, un
//! chemin absolu par ligne, pour que la sonde de codeparsers mesure
//! `parse_project` sur la même entrée, à plusieurs révisions.
//!
//! Run with: SONDE_LISTE=/chemin cargo test --test sonde_liste_analyse -- --ignored

use rag3weaver::code::{verdict, Verdict};
use rag3weaver::code_tools::{FileSource, WorkingTree};

#[test]
#[ignore]
fn ecrire_la_liste_des_fichiers_de_code() {
    let racine = std::env::var("SONDE_RACINE").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    });
    let sortie = std::env::var("SONDE_LISTE").expect("SONDE_LISTE : où écrire la liste");
    let arbre = WorkingTree::new(&racine);
    let t = std::time::Instant::now();
    let chemins = arbre.list().unwrap();
    let mut retenus = 0usize;
    let mut code = Vec::new();
    let mut octets = 0usize;
    for p in chemins {
        if matches!(verdict(&p, 0), Verdict::Ecarte(_)) {
            continue;
        }
        retenus += 1;
        let Ok(Some(contenu)) = arbre.read(&p) else { continue };
        // `C` : passe par codeparsers ; `T` : retenu, mais pas du code — il
        // compte dans le découpage en lots de l'index.
        let abs = std::path::Path::new(&racine).join(&p).to_string_lossy().to_string();
        if matches!(verdict(&p, contenu.len()), Verdict::Code) {
            octets += contenu.len();
            code.push(format!("C\t{abs}"));
        } else {
            code.push(format!("T\t{abs}"));
        }
    }
    std::fs::write(&sortie, code.join("\n")).unwrap();
    eprintln!(
        "[liste] {retenus} fichiers retenus, {} lignes ({:.1} Mo) en {} ms → {sortie}",
        code.len(),
        octets as f64 / 1e6,
        t.elapsed().as_millis()
    );
}

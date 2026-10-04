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

/// Les bibliothèques qu'`analyze` (un seul appel) tire d'un dossier :
/// `SONDE_DOSSIER`, par défaut `src/dataflow` de ce crate.
#[test]
#[ignore]
fn bibliotheques_d_un_dossier() {
    let dossier = std::env::var("SONDE_DOSSIER").unwrap_or_else(|_| format!("{}/src/dataflow", env!("CARGO_MANIFEST_DIR")));
    let analyse = rag3weaver::code::analyze(&dossier, rag3weaver::code::read_sources(&dossier).unwrap());
    let noms: Vec<&str> = analyse.libraries.iter().map(|l| l.name.as_str()).collect();
    eprintln!("[bibliothèques] {} : {noms:?}", noms.len());
}

/// **Ce qui dépend encore de la taille du paquet** : `analyze_in_project`
/// sur la liste exacte de l'index (`SONDE_RACINE`), par paquets de 64 puis
/// de 512 lignes retenues, comme la synchronisation, avec `project_files`
/// = tout. Rend les relations de l'analyseur et les rendez-vous en attente
/// qui n'existent que d'un côté, par type et par extension, avec des
/// exemples. Lourd : deux analyses du dépôt entier.
#[test]
#[ignore]
fn relations_selon_le_paquet() {
    use std::collections::{BTreeMap, HashSet};
    let racine = std::env::var("SONDE_RACINE").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    });
    let arbre = WorkingTree::new(&racine);
    let mut sources: Vec<(String, String)> = Vec::new();
    for p in arbre.list().unwrap() {
        if matches!(verdict(&p, 0), Verdict::Ecarte(_)) {
            continue;
        }
        if let Ok(Some(c)) = arbre.read(&p) {
            sources.push((p, c));
        }
    }
    let projet: Vec<String> = sources.iter().map(|(p, _)| p.clone()).collect();
    let analyser = |taille: usize| -> (HashSet<String>, HashSet<String>) {
        let (mut rels, mut rv) = (HashSet::new(), HashSet::new());
        for paquet in sources.chunks(taille) {
            let a = rag3weaver::code::analyze_in_project(&racine, paquet.to_vec(), "", Some(&projet));
            rels.extend(a.relations.iter().map(|r| format!("{}\t{}\t{}", r.rel, r.from_key, r.to_key)));
            rv.extend(a.pending.iter().map(|(k, n, g)| format!("{g}\t{k}\t{n}")));
        }
        (rels, rv)
    };
    let (r64, p64) = analyser(64);
    let (r512, p512) = analyser(512);
    eprintln!("[paquets] relations 64 : {}, 512 : {} ; rendez-vous 64 : {}, 512 : {}", r64.len(), r512.len(), p64.len(), p512.len());
    let extension = |cle: &str| cle.split('#').next().and_then(|f| f.rsplit('.').next()).unwrap_or("?").to_string();
    for (titre, a, b) in [("relations seulement à 64", &r64, &r512), ("relations seulement à 512", &r512, &r64), ("rendez-vous seulement à 64", &p64, &p512), ("rendez-vous seulement à 512", &p512, &p64)] {
        let ecart: Vec<&String> = a.difference(b).collect();
        let mut classes: BTreeMap<String, usize> = BTreeMap::new();
        for e in &ecart {
            let mut c = e.split('\t');
            let genre = c.next().unwrap_or("");
            let de = c.next().unwrap_or("");
            *classes.entry(format!("{genre} .{}", extension(de))).or_default() += 1;
        }
        eprintln!("\n[{titre}] {} : {classes:?}", ecart.len());
        let mut tri: Vec<&&String> = ecart.iter().collect();
        tri.sort();
        for e in tri.iter().take(12) {
            eprintln!("  {e}");
        }
    }
}

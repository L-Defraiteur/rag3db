//! E2E : **ce que `WorkingTree` liste** (3 octobre 2026).
//!
//! Par défaut, les fichiers que les règles d'exclusion du dossier ne
//! retirent pas (`.gitignore` dans un dépôt git, `.ignore`, exclusions
//! globales), cachés compris sauf `.git/`, sans les secrets probables — que
//! la liste nomme. `all_files()` reprend la marche sans règles.
//!
//! Les noms des fichiers ignorés sont choisis pour ne pas tomber sous une
//! exclusion globale du poste (`*.log`, `.env`…) : le test ne dépend pas de la
//! configuration git de la personne qui le joue.
//!
//! Run with: ./run_e2e.sh --test e2e_working_tree
#![cfg(feature = "code")]

use std::path::Path;

use rag3weaver::code_tools::{FileSource, WorkingTree};

fn ecrire(racine: &Path, chemin: &str, contenu: &str) {
    let p = racine.join(chemin);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, contenu).unwrap();
}

fn dossier(nom: &str, avec_git: bool) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rag3weaver-wt-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    ecrire(&d, ".gitignore", "produit/\n*.tmpx\n");
    ecrire(&d, ".ignore", "donnees/\n");
    ecrire(&d, "src/a.rs", "fn a() {}\n");
    ecrire(&d, "neuf.rs", "fn neuf() {}\n");
    ecrire(&d, "produit/sortie.rs", "fn sortie() {}\n");
    ecrire(&d, "brouillon.tmpx", "x\n");
    ecrire(&d, "donnees/grosse.csv", "a,b\n");
    ecrire(&d, ".github/workflows/x.yml", "on: push\n");
    ecrire(&d, ".env", "SECRET=1\n");
    ecrire(&d, ".env.example", "SECRET=\n");
    ecrire(&d, "deploiement/cle.pem", "-----BEGIN PRIVATE KEY-----\n");
    if avec_git {
        ecrire(&d, ".git/config", "[core]\n");
    }
    d
}

#[test]
#[ignore]
fn dans_un_depot_les_regles_s_appliquent_et_les_secrets_sont_nommes() {
    let d = dossier("depot", true);
    let (liste, ecartes) = WorkingTree::new(&d).list_with_exclusions().unwrap();
    assert_eq!(
        liste,
        [".env.example", ".github/workflows/x.yml", ".gitignore", ".ignore", "neuf.rs", "src/a.rs"],
        "le caché utile est pris, .git/ jamais, l'ignoré non, le neuf oui"
    );
    let noms: Vec<&str> = ecartes.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(noms, [".env", "deploiement/cle.pem"], "{ecartes:?}");
    assert!(ecartes.iter().all(|(_, raison)| !raison.is_empty()));
    assert_eq!(WorkingTree::new(&d).list().unwrap(), liste, "list() rend la même chose");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
#[ignore]
fn hors_d_un_depot_seul_ignore_s_applique() {
    let d = dossier("dossier", false);
    let liste = WorkingTree::new(&d).list().unwrap();
    // Sans dépôt, `.gitignore` ne vaut pas (comme ripgrep) ; `.ignore`, si.
    assert!(liste.contains(&"produit/sortie.rs".to_string()) && liste.contains(&"brouillon.tmpx".to_string()), "{liste:?}");
    assert!(!liste.iter().any(|p| p.starts_with("donnees/")), "{liste:?}");
    assert!(!liste.contains(&".env".to_string()), "les secrets probables, toujours écartés");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
#[ignore]
fn all_files_reprend_la_marche_sans_regles() {
    let d = dossier("tout", true);
    let tout = WorkingTree::new(&d).all_files().list().unwrap();
    assert!(tout.contains(&"produit/sortie.rs".to_string()) && tout.contains(&"donnees/grosse.csv".to_string()), "{tout:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **La mesure sur ce dépôt** : le compte par défaut, à comparer aux fichiers
/// suivis par git. Affichée, sans seuil serré — le défaut compte aussi les
/// fichiers neufs non ignorés, que git ne suit pas encore.
#[test]
#[ignore]
fn mesure_sur_ce_depot() {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let t = std::time::Instant::now();
    let (liste, ecartes) = WorkingTree::new(&racine).list_with_exclusions().unwrap();
    let ms = t.elapsed().as_millis();
    let suivis = std::process::Command::new("git")
        .args(["-C", &racine.to_string_lossy(), "ls-files"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0);
    eprintln!("MESURE WorkingTree sur ce dépôt : {} listés en {ms} ms, {} secrets probables écartés, {suivis} suivis par git", liste.len(), ecartes.len());
    assert!(liste.len() < suivis * 2, "le défaut reste au voisinage des fichiers suivis : {} pour {suivis}", liste.len());
}

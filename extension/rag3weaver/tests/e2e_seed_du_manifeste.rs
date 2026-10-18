//! E2E : **un gabarit sème ses propres lignes** (`seed`, `BackendManifest::seed`,
//! 18 octobre 2026) — les genres de Ref de la mémoire, d'abord.
//!
//! - **Idempotence** : deux ouvertures de la même base, une seule ligne ; une
//!   ligne déjà là n'est jamais réécrite (ce qu'on en a fait l'emporte).
//! - **Refus** : une ligne de seed qui ne valide pas le schéma de son entité
//!   refuse l'ouverture, en se nommant ; une entité non déclarée aussi.
//!
//! Run with: ./run_e2e.sh --test e2e_seed_du_manifeste

#![cfg(feature = "rag3db-native")]

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

/// `RefType` recopiée du gabarit `memory`, sans signaux, et `seed` tel quel.
fn manifeste(dir: &Path, seed: Value) -> PathBuf {
    let memoire = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/memory");
    let gabarit: Value = serde_json::from_slice(&std::fs::read(memoire.join("backend.json")).unwrap()).unwrap();
    let mut genre = gabarit["entities"]["RefType"].clone();
    let schema = genre["schema"].as_str().unwrap().to_string();
    genre["schema"] = json!(memoire.join(schema).display().to_string());
    genre["config"]["signals"] = json!([]);
    let m = json!({
        "version": 1, "name": "semis", "database": "base.rag3db",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": crate::common::vector_extension(),
        "entities": {"RefType": genre},
        "tools": {},
        "seed": seed
    });
    let chemin = dir.join("backend.json");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

fn ouvrir(chemin: &Path) -> Result<Backend, String> {
    let prepared = PreparedBackend::load(chemin)?;
    let base = prepared.path(&prepared.manifest.database);
    prepared.open(Box::new(Rag3dbConnection::new(&base).expect("la base")), None)
}

fn lignes(backend: &Backend) -> Vec<(String, String)> {
    let r = backend
        .catalog
        .lock()
        .unwrap()
        .execute_raw_with_params("MATCH (t:RefType) RETURN t.name, t.description ORDER BY t.name", &[])
        .expect("la lecture");
    r.rows
        .iter()
        .map(|l| (l[0].as_str().unwrap_or("").to_string(), l[1].as_str().unwrap_or("").to_string()))
        .collect()
}

fn genre(nom: &str, description: &str) -> Value {
    json!({"name": nom, "description": description, "forme": "un chemin relatif", "script": "", "etat": "current"})
}

#[test]
#[ignore]
fn deux_ouvertures_une_seule_ligne_et_la_ligne_modifiee_n_est_pas_reecrite() {
    let dir = tempfile::tempdir().expect("dossier");
    let chemin = manifeste(dir.path(), json!({"RefType": [genre("path", "un fichier ou un dossier")]}));
    {
        let backend = ouvrir(&chemin).expect("la première ouverture");
        assert_eq!(lignes(&backend), vec![("path".into(), "un fichier ou un dossier".into())]);
        // Semer de nouveau dans la même ouverture ne pose rien.
        assert_eq!(backend.seed().expect("le semis"), 0);
        // Ce qu'on fait de la ligne : sa description reprise à la main.
        backend
            .catalog
            .lock()
            .unwrap()
            .execute_raw_with_params("MATCH (t:RefType {name: 'path'}) SET t.description = 'reprise'", &[])
            .expect("la reprise");
    }
    let backend = ouvrir(&chemin).expect("la seconde ouverture");
    assert_eq!(
        lignes(&backend),
        vec![("path".into(), "reprise".into())],
        "deux ouvertures : une seule ligne, et la ligne reprise n'est pas réécrite par le gabarit"
    );
}

#[test]
#[ignore]
fn une_ligne_qui_ne_valide_pas_refuse_l_ouverture_en_se_nommant() {
    let dir = tempfile::tempdir().expect("dossier");
    // `etat` hors de l'énumération du schéma (current | retiree).
    let mut mauvais = genre("symbol", "un symbole");
    mauvais["etat"] = json!("peut-etre");
    let chemin = manifeste(dir.path(), json!({"RefType": [genre("path", "un chemin"), mauvais]}));
    let erreur = ouvrir(&chemin).err().expect("l'ouverture doit être refusée");
    assert!(
        erreur.contains("seed.RefType[1]") && erreur.contains("\"symbol\""),
        "le refus nomme la ligne (entité, rang, identité) : {erreur}"
    );
}

#[test]
#[ignore]
fn une_entite_non_declaree_refuse_l_ouverture() {
    let dir = tempfile::tempdir().expect("dossier");
    let chemin = manifeste(dir.path(), json!({"Fantome": [{"name": "x"}]}));
    let erreur = ouvrir(&chemin).err().expect("l'ouverture doit être refusée");
    assert!(erreur.contains("seed.Fantome"), "le refus nomme l'entité : {erreur}");
}

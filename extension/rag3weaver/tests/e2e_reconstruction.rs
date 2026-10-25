//! E2E : **la reconstruction de la base depuis les fichiers** (lot M3a de la
//! mémoire, `docs/18-octobre-2026-memoire-longue/04-…` §5) — une entité à
//! fichier propre :
//!
//! - **la parité, champ à champ** : des lignes écrites par un vrai outil (une
//!   réécrite, une sans champ facultatif), puis une base neuve reconstruite
//!   depuis leurs fichiers ; les deux bases portent les mêmes lignes, chaque
//!   champ déclaré, dates et révision comprises ;
//! - une reconstruction sur une base qui n'est pas vide est refusée, et le dit ;
//! - un champ que les fichiers ne portent pas est dit (`fields_not_carried`) au
//!   lieu d'être perdu en silence.
//!
//! Run with: ./run_e2e.sh --test e2e_reconstruction

#![cfg(feature = "rag3db-native")]

use std::path::{Path, PathBuf};

use serde_json::json;

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::connection::CypherValue;
use rag3weaver::Rag3dbConnection;

/// Un manifeste sur `base` (un nom de base sous `dir`), les fichiers sous
/// `dir/fichiers` ; `en_tete` : les champs que l'en-tête porte.
fn manifeste(dir: &Path, nom: &str, base: &str, en_tete: &[&str]) -> PathBuf {
    std::fs::write(
        dir.join("note.json"),
        serde_json::to_vec_pretty(&json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object", "additionalProperties": false,
            "required": ["titre", "corps", "revision", "created_at", "updated_at"],
            "properties": {
                "titre": {"type": "string", "minLength": 1}, "corps": {"type": "string", "minLength": 1},
                "etiquette": {"type": "string"},
                "revision": {"type": "integer"}, "created_at": {"type": "integer"}, "updated_at": {"type": "integer"}
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let put = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/backends/notebook/graphs/put.mmd");
    let m = json!({
        "version": 1, "name": "reconstruction", "database": base, "files_root": "fichiers",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": crate::common::vector_extension(),
        "entities": {"Note": {
            "schema": "note.json",
            "config": {"fields": {"titre": {"type": "string", "isTitle": true}, "corps": {"type": "string", "isContent": true}},
                       "hashsafe": ["titre"], "signals": []},
            "writes": {"created_at": "created_at", "updated_at": "updated_at", "revision": "revision"},
            "files": {"dir": "notes", "name": "{uuid}.md", "header": en_tete, "body": "corps"}
        }},
        "tools": {"put_note": {"graph": put, "bindings": {"entity": "Note"},
                               "input_payloads": {"record": {"entity": "Note", "view": "editable"}}}}
    });
    let chemin = dir.join(nom);
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

const TOUT: &[&str] = &["titre", "etiquette", "revision", "created_at", "updated_at"];

fn ouvrir(chemin: &Path) -> Backend {
    let prepared = PreparedBackend::load(chemin).expect("le manifeste charge");
    let base = prepared.path(&prepared.manifest.database);
    prepared.open(Box::new(Rag3dbConnection::new(&base).expect("la base")), None).expect("l'ouverture")
}

/// Les lignes de la base, chaque champ déclaré, triées par uuid.
fn lignes(b: &Backend) -> Vec<Vec<CypherValue>> {
    let cat = b.catalog.lock().unwrap();
    cat.execute_raw(
        "MATCH (n:Note) RETURN n._uuid, n.titre, n.etiquette, n.corps, n.revision, n.created_at, n.updated_at ORDER BY n._uuid",
    )
    .expect("les lignes")
    .rows
}

#[test]
#[ignore]
fn une_base_reconstruite_depuis_les_fichiers_porte_les_memes_lignes_champ_a_champ() {
    let dir = tempfile::tempdir().expect("dossier");
    let vivante = ouvrir(&manifeste(dir.path(), "backend.json", "base.rag3db", TOUT));
    vivante.call("put_note", json!({"record": {"titre": "Le WAL", "etiquette": "moteur", "corps": "# Le WAL\n\nLe journal se rejoue.\n"}})).expect("une note");
    vivante.call("put_note", json!({"record": {"titre": "Sans étiquette", "corps": "Une note sans champ facultatif.\n"}})).expect("une note sans étiquette");
    vivante.call("put_note", json!({"record": {"titre": "Réécrite", "etiquette": "v1", "corps": "Première version.\n"}})).expect("une note");
    // Le temps passe : la réécriture doit garder sa date de création, et la sienne.
    std::thread::sleep(std::time::Duration::from_millis(20));
    vivante
        .call("put_note", json!({"record": {"titre": "Réécrite", "etiquette": "v2", "corps": "Seconde version, ---\n\navec un séparateur.\n"}, "expected_revision": 1}))
        .expect("la réécriture");
    let attendu = lignes(&vivante);
    assert_eq!(attendu.len(), 3);
    assert!(attendu.iter().any(|l| l[4] == CypherValue::Int(2)), "une ligne en révision 2 : {attendu:?}");
    drop(vivante);

    // Une base neuve, sur les mêmes fichiers.
    let neuve = ouvrir(&manifeste(dir.path(), "neuve.json", "neuve.rag3db", TOUT));
    let rapport = neuve.rebuild_from_files().expect("la reconstruction");
    assert_eq!(rapport["Note"]["restored"], 3, "{rapport}");
    assert_eq!(rapport["Note"]["refused"], json!([]), "{rapport}");
    assert_eq!(rapport["Note"]["fields_not_carried"], json!([]), "{rapport}");
    assert_eq!(lignes(&neuve), attendu, "la base reconstruite porte les mêmes lignes, champ à champ");

    // Une seconde reconstruction sur une base qui n'est plus vide : refusée, et dite.
    let refus = neuve.rebuild_from_files().expect_err("une base pleine");
    assert!(refus.contains("n'est pas vide") && refus.contains("Note : 3"), "{refus}");
}

#[test]
#[ignore]
fn un_champ_que_les_fichiers_ne_portent_pas_est_dit() {
    let dir = tempfile::tempdir().expect("dossier");
    let vivante = ouvrir(&manifeste(dir.path(), "backend.json", "base.rag3db", &["titre", "revision", "created_at", "updated_at"]));
    vivante.call("put_note", json!({"record": {"titre": "Une note", "etiquette": "perdue", "corps": "Le corps.\n"}})).expect("une note");
    drop(vivante);
    let neuve = ouvrir(&manifeste(dir.path(), "neuve.json", "neuve.rag3db", &["titre", "revision", "created_at", "updated_at"]));
    let rapport = neuve.rebuild_from_files().expect("la reconstruction");
    assert_eq!(rapport["Note"]["restored"], 1, "{rapport}");
    assert_eq!(rapport["Note"]["fields_not_carried"], json!(["etiquette"]), "le champ perdu se dit : {rapport}");
}

//! E2E : **le miroir en fichiers d'une entité** (lot M2 de la mémoire,
//! `src/fichiers_entite.rs`) — par un vrai `put` d'outil :
//!
//! - le fichier est écrit sous `files_root`, avec l'uuid et l'en-tête déclarés,
//!   le corps tel quel, et la réponse dit son chemin ;
//! - un `put` refusé par le schéma n'écrit aucun fichier ;
//! - une entité qui déclare `files` sans `files_root` refuse le chargement.
//!
//! Run with: ./run_e2e.sh --test e2e_fichiers_entite

#![cfg(feature = "rag3db-native")]

use std::path::{Path, PathBuf};

use serde_json::json;

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

fn manifeste(dir: &Path, avec_racine: bool) -> PathBuf {
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
    let mut m = json!({
        "version": 1, "name": "fichiers", "database": "base.rag3db",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": crate::common::vector_extension(),
        "entities": {"Note": {
            "schema": "note.json",
            "config": {"fields": {"titre": {"type": "string", "isTitle": true}, "corps": {"type": "string", "isContent": true}},
                       "hashsafe": ["titre"], "signals": []},
            "writes": {"created_at": "created_at", "updated_at": "updated_at", "revision": "revision"},
            "files": {"dir": "notes", "name": "{uuid}.md", "header": ["titre", "etiquette", "revision", "created_at", "updated_at"], "body": "corps"}
        }},
        "tools": {"put_note": {"graph": put, "bindings": {"entity": "Note"},
                               "input_payloads": {"record": {"entity": "Note", "view": "editable"}}}}
    });
    if avec_racine {
        m["files_root"] = json!("fichiers");
    }
    let chemin = dir.join("backend.json");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

fn ouvrir(chemin: &Path) -> Backend {
    let prepared = PreparedBackend::load(chemin).expect("le manifeste charge");
    let base = prepared.path(&prepared.manifest.database);
    prepared.open(Box::new(Rag3dbConnection::new(&base).expect("la base")), None).expect("l'ouverture")
}

fn fichiers(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir.join("fichiers/notes"))
        .map(|d| d.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    v.sort();
    v
}

#[test]
#[ignore]
fn un_put_ecrit_le_fichier_avant_la_ligne_et_dit_son_chemin() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(&manifeste(dir.path(), true));
    let r = b
        .call("put_note", json!({"record": {"titre": "Le WAL : la reprise", "etiquette": "no", "corps": "# Reprise\n\nLe journal se rejoue.\n"}}))
        .expect("le put");
    let uuid = r["result"]["uuid"].as_str().expect("l'uuid").to_string();
    let rel = r["result"]["file"].as_str().unwrap_or_else(|| panic!("la réponse dit le fichier : {r}"));
    assert_eq!(rel, format!("notes/{uuid}.md"));
    let texte = std::fs::read_to_string(dir.path().join("fichiers").join(rel)).expect("le fichier existe");
    let (yaml, corps) = rag3weaver::en_tete::separer(&texte).expect("un en-tête");
    assert_eq!(corps, "# Reprise\n\nLe journal se rejoue.\n", "le corps tel quel");
    assert!(yaml.starts_with(&format!("uuid: \"{uuid}\"\nformat: 1\ntitre: \"Le WAL : la reprise\"\netiquette: \"no\"\nrevision: 1\n")), "{yaml}");

    // Une seconde écriture réécrit le même fichier (même nom), avec la révision suivante.
    b.call("put_note", json!({"record": {"titre": "Le WAL : la reprise", "corps": "# Reprise\n\nRejoué en entier.\n"}, "expected_revision": 1}))
        .expect("le second put");
    assert_eq!(fichiers(dir.path()), vec![format!("{uuid}.md")], "un seul fichier par ligne");
    let texte = std::fs::read_to_string(dir.path().join("fichiers").join(rel)).unwrap();
    assert!(texte.contains("revision: 2\n") && texte.ends_with("Rejoué en entier.\n"), "{texte}");
}

#[test]
#[ignore]
fn un_put_refuse_par_le_schema_n_ecrit_aucun_fichier() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(&manifeste(dir.path(), true));
    let e = b.call("put_note", json!({"record": {"titre": "x", "corps": ""}})).err().expect("un corps vide est refusé");
    assert!(!e.is_empty());
    assert!(fichiers(dir.path()).is_empty(), "aucun fichier pour une ligne refusée");
}

#[test]
#[ignore]
fn files_sans_files_root_refuse_le_chargement() {
    let dir = tempfile::tempdir().expect("dossier");
    let e = PreparedBackend::load(&manifeste(dir.path(), false)).err().expect("le chargement doit refuser");
    assert!(e.contains("files_root"), "le refus dit quoi déclarer : {e}");
}

//! E2E : **le fichier composé** (lot M2c de la mémoire,
//! `docs/18-octobre-2026-memoire-longue/04-…` §5 bis) — une vue de la base,
//! réécrite par la réaction de miroir, jamais par l'écrivain :
//!
//! - le `put` d'un sujet ne l'écrit pas lui-même ; la réaction l'écrit, juste après ;
//! - une note liée au sujet, puis réécrite, réécrit le fichier du sujet (sa
//!   ligne liée a changé) ;
//! - à l'ouverture, un fichier perdu ou retouché est régénéré depuis la base.
//!
//! Le lien posé après coup, la transition et la consolidation demandent que le
//! catalogue publie ces écritures (la pièce 1 du lot) : leurs témoins viennent
//! avec elle.
//!
//! Run with: ./run_e2e.sh --test e2e_fichier_compose

#![cfg(feature = "rag3db-native")]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::json;

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

fn schema(dir: &Path, nom: &str, champs: &[&str]) {
    let mut props = serde_json::Map::new();
    for c in champs {
        props.insert(c.to_string(), json!({"type": "string", "minLength": 1}));
    }
    for c in ["revision", "created_at", "updated_at"] {
        props.insert(c.into(), json!({"type": "integer"}));
    }
    let mut requis: Vec<&str> = champs.to_vec();
    requis.extend(["revision", "created_at", "updated_at"]);
    std::fs::write(
        dir.join(nom),
        serde_json::to_vec_pretty(&json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
            "additionalProperties": false, "required": requis, "properties": props}))
        .unwrap(),
    )
    .unwrap();
}

fn manifeste(dir: &Path) -> PathBuf {
    schema(dir, "sujet.json", &["name", "about"]);
    schema(dir, "note.json", &["titre", "texte"]);
    std::fs::write(
        dir.join("sujet.md.jinja"),
        "# {{ record.name }}\n\n{{ record.about }}\n\n## Notes\n{% for n in related.notes %}\n- {{ n.titre }} : {{ n.texte }}\n{% endfor %}\n",
    )
    .unwrap();
    let tpl = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let m = json!({
        "version": 1, "name": "compose", "database": "base.rag3db", "files_root": "fichiers",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": crate::common::vector_extension(),
        "entities": {
            "Sujet": {"schema": "sujet.json",
                "config": {"fields": {"name": {"type": "string", "isTitle": true}, "about": {"type": "string", "isContent": true}}, "hashsafe": ["name"], "signals": []},
                "writes": {"created_at": "created_at", "updated_at": "updated_at", "revision": "revision"},
                "files": {"dir": "sujets", "name": "{uuid}.md", "header": ["name", "revision"], "template": "sujet.md.jinja",
                          "related": {"notes": {"relation": "A_NOTE", "order_by": "titre"}}}},
            "Note": {"schema": "note.json",
                "config": {"fields": {"titre": {"type": "string", "isTitle": true}, "texte": {"type": "string", "isContent": true}}, "hashsafe": ["titre"], "signals": []},
                "writes": {"created_at": "created_at", "updated_at": "updated_at", "revision": "revision"}}
        },
        "relations": {"A_NOTE": {"from": "Sujet", "to": "Note"}},
        "tools": {
            "put_sujet": {"graph": tpl.join("backends/notebook/graphs/put.mmd"), "bindings": {"entity": "Sujet"},
                          "input_payloads": {"record": {"entity": "Sujet", "view": "editable"}}},
            "put_note": {"graph": tpl.join("backends/notebook/graphs/put.mmd"), "bindings": {"entity": "Note"},
                         "input_payloads": {"record": {"entity": "Note", "view": "editable"}}},
            "noter": {"graph": tpl.join("tools/link.mmd"), "bindings": {"relation": "A_NOTE"}}
        }
    });
    let chemin = dir.join("backend.json");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

fn ouvrir(chemin: &Path) -> Backend {
    let prepared = PreparedBackend::load(chemin).expect("le manifeste charge");
    let base = prepared.path(&prepared.manifest.database);
    prepared.open(Box::new(Rag3dbConnection::new(&base).expect("la base")), None).expect("l'ouverture")
}

/// Attend (au plus 5 s) que le fichier contienne `attendu`.
fn attendre(fichier: &Path, attendu: &str) -> String {
    let debut = Instant::now();
    loop {
        let texte = std::fs::read_to_string(fichier).unwrap_or_default();
        if texte.contains(attendu) {
            return texte;
        }
        assert!(debut.elapsed() < Duration::from_secs(5), "{} ne contient pas « {attendu} » après 5 s : {texte:?}", fichier.display());
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
#[ignore]
fn la_reaction_ecrit_le_fichier_du_sujet_et_le_reecrit_quand_une_note_liee_change() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(&manifeste(dir.path()));
    let r = b.call("put_sujet", json!({"record": {"name": "le WAL", "about": "la reprise du journal"}})).expect("le put");
    assert!(r["result"]["file"].is_null(), "un fichier composé n'est pas écrit par le put : {r}");
    let uuid = r["result"]["uuid"].as_str().unwrap().to_string();
    let fichier = dir.path().join("fichiers/sujets").join(format!("{uuid}.md"));
    let texte = attendre(&fichier, "# le WAL");
    assert!(texte.starts_with(&format!("---\nuuid: \"{uuid}\"\nname: \"le WAL\"\nrevision: 1\n---\n")), "{texte}");

    b.call("put_note", json!({"record": {"titre": "rejeu", "texte": "en entier"}})).expect("la note");
    b.call("noter", json!({"links": [{"from": {"name": "le WAL"}, "to": {"titre": "rejeu"}}]})).expect("le lien");
    // La note réécrite : sa ligne liée change, le fichier du sujet suit.
    b.call("put_note", json!({"record": {"titre": "rejeu", "texte": "en entier, avant toute écriture"}, "expected_revision": 1}))
        .expect("la note réécrite");
    attendre(&fichier, "- rejeu : en entier, avant toute écriture");
}

#[test]
#[ignore]
fn a_l_ouverture_un_fichier_perdu_ou_retouche_est_regenere_depuis_la_base() {
    let dir = tempfile::tempdir().expect("dossier");
    let chemin = manifeste(dir.path());
    let uuid;
    let attendu;
    {
        let b = ouvrir(&chemin);
        uuid = b.call("put_sujet", json!({"record": {"name": "le WAL", "about": "la reprise"}})).expect("le put")["result"]["uuid"]
            .as_str()
            .unwrap()
            .to_string();
        let fichier = dir.path().join("fichiers/sujets").join(format!("{uuid}.md"));
        attendu = attendre(&fichier, "# le WAL");
    }
    let fichier = dir.path().join("fichiers/sujets").join(format!("{uuid}.md"));
    // Retouché : l'ouverture le remet dans la forme de la base.
    std::fs::write(&fichier, "retouché à la main\n").unwrap();
    {
        let _b = ouvrir(&chemin);
        assert_eq!(std::fs::read_to_string(&fichier).unwrap(), attendu, "régénéré depuis la base à l'ouverture");
    }
    // Perdu : l'ouverture le recrée.
    std::fs::remove_file(&fichier).unwrap();
    let _b = ouvrir(&chemin);
    assert_eq!(std::fs::read_to_string(&fichier).unwrap(), attendu);
}

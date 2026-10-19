//! E2E : **le fichier composé** (lot M2c de la mémoire,
//! `docs/18-octobre-2026-memoire-longue/04-…` §5 bis) — une vue de la base,
//! réécrite par la réaction de miroir, jamais par l'écrivain :
//!
//! - le `put` d'un sujet ne l'écrit pas lui-même ; la réaction l'écrit, juste après ;
//! - une note liée au sujet, puis réécrite, réécrit le fichier du sujet (sa
//!   ligne liée a changé) ;
//! - à l'ouverture, un fichier perdu ou retouché est régénéré depuis la base.
//!
//! - un lien posé après coup, une transition de la fiche, puis la consolidation
//!   d'une ligne liée (sa transition) la réécrivent, sans qu'aucun écrivain l'ait
//!   demandé (le catalogue publie ces écritures depuis la pièce 1 du lot,
//!   19 octobre 2026).
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
    schema(dir, "sujet.json", &["name", "about", "etat"]);
    schema(dir, "note.json", &["titre", "texte", "etat"]);
    std::fs::write(
        dir.join("sujet.md.jinja"),
        "# {{ record.name }}\n\n{{ record.about }}\n\n## Notes\n{% for n in related.notes %}\n- {{ n.titre }} : {{ n.texte }} [{{ n.etat }}]\n{% endfor %}\n",
    )
    .unwrap();
    let tpl = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let m = json!({
        "version": 1, "name": "compose", "database": "base.rag3db", "files_root": "fichiers",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": crate::common::vector_extension(),
        "entities": {
            "Sujet": {"schema": "sujet.json",
                "config": {"fields": {"name": {"type": "string", "isTitle": true}, "about": {"type": "string", "isContent": true}, "etat": {"type": "string"}}, "hashsafe": ["name"], "signals": [],
                           "lifecycle": {"field": "etat", "initial": "courant", "transitions": [{"name": "revoir", "from": "courant", "to": "revu"}]}},
                "writes": {"created_at": "created_at", "updated_at": "updated_at", "revision": "revision"},
                "files": {"dir": "sujets", "name": "{uuid}.md", "header": ["name", "etat", "revision"], "template": "sujet.md.jinja",
                          "related": {"notes": {"relation": "A_NOTE", "order_by": "titre"}}}},
            "Note": {"schema": "note.json",
                "config": {"fields": {"titre": {"type": "string", "isTitle": true}, "texte": {"type": "string", "isContent": true}, "etat": {"type": "string"}}, "hashsafe": ["titre"], "signals": [],
                           "lifecycle": {"field": "etat", "initial": "courant", "transitions": [{"name": "consolider", "from": "courant", "to": "consolide"}]}},
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
    let r = b.call("put_sujet", json!({"record": {"name": "le WAL", "about": "la reprise du journal", "etat": "courant"}})).expect("le put");
    assert!(r["result"]["file"].is_null(), "un fichier composé n'est pas écrit par le put : {r}");
    let uuid = r["result"]["uuid"].as_str().unwrap().to_string();
    let fichier = dir.path().join("fichiers/sujets").join(format!("{uuid}.md"));
    let texte = attendre(&fichier, "# le WAL");
    assert!(texte.starts_with(&format!("---\nuuid: \"{uuid}\"\nname: \"le WAL\"\netat: \"courant\"\nrevision: 1\n---\n")), "{texte}");

    b.call("put_note", json!({"record": {"titre": "rejeu", "texte": "en entier", "etat": "courant"}})).expect("la note");
    b.call("noter", json!({"links": [{"from": {"name": "le WAL"}, "to": {"titre": "rejeu"}}]})).expect("le lien");
    // La note réécrite : sa ligne liée change, le fichier du sujet suit.
    b.call("put_note", json!({"record": {"titre": "rejeu", "texte": "en entier, avant toute écriture", "etat": "courant"}, "expected_revision": 1}))
        .expect("la note réécrite");
    attendre(&fichier, "- rejeu : en entier, avant toute écriture [courant]");
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
        uuid = b.call("put_sujet", json!({"record": {"name": "le WAL", "about": "la reprise", "etat": "courant"}})).expect("le put")["result"]["uuid"]
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

#[test]
#[ignore]
fn un_lien_pose_apres_coup_une_transition_et_une_consolidation_reecrivent_le_fichier() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(&manifeste(dir.path()));
    let uuid = b.call("put_sujet", json!({"record": {"name": "le WAL", "about": "la reprise", "etat": "courant"}})).expect("le sujet")["result"]
        ["uuid"]
        .as_str()
        .unwrap()
        .to_string();
    let fichier = dir.path().join("fichiers/sujets").join(format!("{uuid}.md"));
    attendre(&fichier, "# le WAL");
    b.call("put_note", json!({"record": {"titre": "rejeu", "texte": "en entier", "etat": "courant"}})).expect("la note");

    // 1. Le lien posé après coup : RelationsChanged, la note entre dans le fichier.
    b.call("noter", json!({"links": [{"from": {"name": "le WAL"}, "to": {"titre": "rejeu"}}]})).expect("le lien");
    attendre(&fichier, "- rejeu : en entier [courant]");

    // 2. La transition de la fiche (un écrivain direct, aucun outil) : l'en-tête suit.
    {
        let mut cat = b.catalog.lock().unwrap();
        let mut etat = std::collections::BTreeMap::new();
        etat.insert("etat".to_string(), rag3weaver::connection::CypherValue::String("revu".into()));
        cat.update_many("Sujet", vec![(uuid.clone(), etat)]).expect("la transition du sujet");
    }
    attendre(&fichier, "etat: \"revu\"");

    // 3. La consolidation d'une ligne liée (sa transition) : le corps suit.
    {
        let mut cat = b.catalog.lock().unwrap();
        let note = cat.entity_uuid("Note", &{
            let mut d = std::collections::BTreeMap::new();
            d.insert("titre".to_string(), rag3weaver::connection::CypherValue::String("rejeu".into()));
            d
        })
        .expect("l'uuid de la note");
        let mut etat = std::collections::BTreeMap::new();
        etat.insert("etat".to_string(), rag3weaver::connection::CypherValue::String("consolide".into()));
        cat.update_many("Note", vec![(note, etat)]).expect("la consolidation");
    }
    attendre(&fichier, "- rejeu : en entier [consolide]");
}

/// **Sous une annonce tronquée, le miroir reste exact** (c7, 19 octobre) :
/// plafond abaissé à 3 paires, cinq notes liées d'un coup. L'annonce ne les
/// nomme pas toutes ; le miroir, qui lit cette relation, régénère tout depuis
/// la base, et les cinq sont dans le fichier.
#[test]
#[ignore]
fn sous_une_annonce_tronquee_le_miroir_reste_exact() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(&manifeste(dir.path()));
    b.catalog.lock().unwrap().set_paires_par_evenement(3);
    let uuid = b.call("put_sujet", json!({"record": {"name": "le tampon", "about": "les pages", "etat": "courant"}})).expect("le sujet")["result"]
        ["uuid"]
        .as_str()
        .unwrap()
        .to_string();
    let fichier = dir.path().join("fichiers/sujets").join(format!("{uuid}.md"));
    attendre(&fichier, "# le tampon");
    let mut liens = Vec::new();
    for i in 0..5 {
        let titre = format!("page {i}");
        b.call("put_note", json!({"record": {"titre": titre, "texte": "à relire", "etat": "courant"}})).expect("la note");
        liens.push(json!({"from": {"name": "le tampon"}, "to": {"titre": titre}}));
    }
    b.call("noter", json!({"links": liens})).expect("les liens");
    for i in 0..5 {
        attendre(&fichier, &format!("- page {i} : à relire [courant]"));
    }
}

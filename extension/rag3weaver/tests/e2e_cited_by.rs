//! E2E : **`cited_by` sur la fiche** (`docs/18-octobre-2026-memoire-longue/04-…` §7),
//! lu par un Plan déclaré (`templates/backends/memory/plans/cited_by.json`) :
//!
//! - une fiche répond quand elle est **ancrée** à la chose (ANCRE vers une Ref),
//!   ou quand **un de ses apports la cite** (A_POUR_APPORT, puis APPORT_CITE) ;
//! - `under` prend ce qui est dessous : un dossier (`valeur/…`) et les symboles
//!   d'un fichier (`valeur#…`), jamais un voisin qui partage le préfixe
//!   (`src/storagex`) ;
//! - chaque fiche rend ses Refs qui répondent et ses apports qui citent, rien
//!   d'autre ; les plus récentes d'abord ; le total compté, la page bornée.
//!
//! Sur le vrai gabarit `memory` (ses signaux : `RAG3WEAVER_EMBED_SERVICE`).
//!
//! Run with: ./run_e2e.sh --test e2e_cited_by

#![cfg(feature = "rag3db-native")]

use std::path::Path;

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

/// Le gabarit `memory` dans une copie de `templates/`, son extension vector
/// prise au bâti (comme `e2e_memoire_modele`).
fn ouvrir(dir: &Path) -> Backend {
    fn copier(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for e in std::fs::read_dir(src).unwrap() {
            let e = e.unwrap();
            let cible = dst.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copier(&e.path(), &cible);
            } else {
                std::fs::copy(e.path(), &cible).unwrap();
            }
        }
    }
    copier(&Path::new(env!("CARGO_MANIFEST_DIR")).join("templates"), &dir.join("templates"));
    let manifeste = dir.join("templates/backends/memory/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["vector_extension"] = json!(crate::common::vector_extension());
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let prepared = PreparedBackend::load(&manifeste).expect("le gabarit memory charge");
    let (embedder, _) = prepared.connect_embedder().unwrap_or_else(|e| {
        panic!("le témoin embarque : posez RAG3WEAVER_EMBED_SERVICE — un témoin sauté n'est pas vert ({e})")
    });
    prepared
        .open(Box::new(Rag3dbConnection::new(&dir.join("memoire.rag3db")).expect("la base")), Some(embedder))
        .expect("l'ouverture")
}

fn appel(b: &Backend, outil: &str, args: Value) -> Value {
    b.call(outil, args.clone()).unwrap_or_else(|e| panic!("{outil} {args} : {e}"))
}

/// Une fiche, après une pause : les dates se suivent, l'ordre se juge.
fn fiche(b: &Backend, name: &str, state: &str) {
    std::thread::sleep(std::time::Duration::from_millis(5));
    appel(b, "put_fiche", json!({"record": {"name": name, "about": format!("La fiche {name}."), "state": state}}));
}

fn ancrer(b: &Backend, name: &str, genre: &str, valeur: &str) {
    appel(b, "ancrer_fiche", json!({"genre": genre, "valeur": valeur, "fiche": {"name": name}}));
}

const SOURCE: &str = "sources/2026-10-25-essai.md";

/// Un apport rangé sur sa fiche, qui cite `doc:` sa section.
fn apport_qui_cite(b: &Backend, name: &str, section: &str) {
    let cle = json!({"fiche": name, "source": SOURCE, "section": section});
    let mut record = cle.clone();
    record["texte"] = json!(format!("Ce que la section {section} dit de {name}."));
    record["niveau"] = json!("L1");
    record["state"] = json!("current");
    appel(b, "put_apport", json!({"record": record}));
    appel(b, "relier_apport", json!({"links": [{"from": {"name": name}, "to": cle}]}));
    appel(b, "citer_source", json!({"genre": "doc", "valeur": format!("{SOURCE}#{section}"), "fiche": cle}));
}

/// `cited_by` en json : les fiches (leur ligne du Plan) et le total.
fn cite(b: &Backend, args: Value) -> (Vec<Value>, i64) {
    let mut args = args;
    args["format"] = json!("json");
    let r = appel(b, "cited_by", args);
    let texte = r["result"].as_str().unwrap_or_else(|| panic!("un texte json : {r}"));
    let resultats: Vec<Value> = serde_json::from_str(texte).unwrap_or_else(|e| panic!("{texte} : {e}"));
    let total = r["metadata"]["plan.total"].as_i64().unwrap_or_else(|| panic!("le total : {r}"));
    (resultats.into_iter().map(|x| x["data"].clone()).collect(), total)
}

fn noms(fiches: &[Value]) -> Vec<&str> {
    fiches.iter().map(|f| f["name"].as_str().unwrap()).collect()
}

/// Les genres du témoin, et quatre fiches : deux sous `src/storage/wal`, une
/// voisine de préfixe, une qui ne cite que par son apport.
fn peupler(b: &Backend) {
    for (genre, description) in [("doc", "un document source et sa section"), ("path", "un chemin du dépôt"), ("symbol", "un symbole d'un fichier")] {
        appel(b, "create_ref_type", json!({"name": genre, "description": description, "forme": "relatif", "confirm": true}));
    }
    fiche(b, "le rejeu", "current");
    ancrer(b, "le rejeu", "path", "src/storage/wal/replay.cpp");
    fiche(b, "l'ouverture", "to_review");
    ancrer(b, "l'ouverture", "symbol", "src/storage/wal/replay.cpp#Replay::run");
    ancrer(b, "l'ouverture", "path", "docs/ouverture.md");
    fiche(b, "le voisin", "current");
    ancrer(b, "le voisin", "path", "src/storagex/x.cpp");
    fiche(b, "la reprise", "current");
    apport_qui_cite(b, "la reprise", "Reprise");
    apport_qui_cite(b, "la reprise", "Autre");
}

#[test]
#[ignore]
fn un_dossier_rend_les_fiches_ancrees_dessous_par_date_et_seulement_leurs_refs_qui_repondent() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path());
    peupler(&b);

    let (fiches, total) = cite(&b, json!({"value": "src/storage/wal"}));
    assert_eq!(noms(&fiches), ["l'ouverture", "le rejeu"], "les plus récentes d'abord, sans le voisin de préfixe : {fiches:?}");
    assert_eq!(total, 2);
    let refs: Vec<&str> = fiches[0]["refs"].as_array().unwrap().iter().map(|r| r["valeur"].as_str().unwrap()).collect();
    assert_eq!(refs, ["src/storage/wal/replay.cpp#Replay::run"], "seule la Ref qui répond, pas docs/ouverture.md : {fiches:?}");
    assert_eq!(fiches[1]["apports"], json!([]), "une fiche ancrée sans apport qui cite : une liste vide");

    // Sans `under`, la valeur seule : le fichier, pas ses symboles.
    let (fiches, total) = cite(&b, json!({"value": "src/storage/wal/replay.cpp", "under": false}));
    assert_eq!((noms(&fiches), total), (vec!["le rejeu"], 1));
    // Un genre gardé : les symboles seuls.
    let (fiches, _) = cite(&b, json!({"value": "src/storage/wal", "genres": ["symbol"]}));
    assert_eq!(noms(&fiches), ["l'ouverture"]);
    // Un état gardé.
    let (fiches, _) = cite(&b, json!({"value": "src/storage/wal", "states": ["current"]}));
    assert_eq!(noms(&fiches), ["le rejeu"]);
}

#[test]
#[ignore]
fn un_source_rend_les_fiches_dont_un_apport_le_cite_avec_ces_apports() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path());
    peupler(&b);

    // Le source entier : ses sections sont dessous (`#`).
    let (fiches, total) = cite(&b, json!({"value": SOURCE, "genres": ["doc"]}));
    assert_eq!((noms(&fiches), total), (vec!["la reprise"], 1), "{fiches:?}");
    let mut sections: Vec<&str> = fiches[0]["apports"].as_array().unwrap().iter().map(|a| a["section"].as_str().unwrap()).collect();
    sections.sort();
    assert_eq!(sections, ["Autre", "Reprise"], "{fiches:?}");
    assert_eq!(fiches[0]["refs"], json!([]), "la fiche n'est pas ancrée : elle répond par ses apports");

    // Une section : seul l'apport qui la cite.
    let (fiches, _) = cite(&b, json!({"value": format!("{SOURCE}#Reprise"), "under": false}));
    let apports = fiches[0]["apports"].as_array().unwrap();
    assert_eq!(apports.len(), 1, "{fiches:?}");
    assert_eq!(apports[0]["section"], "Reprise");
    assert_eq!(apports[0]["niveau"], "L1");
}

#[test]
#[ignore]
fn la_page_est_bornee_et_le_total_compte_tout() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path());
    peupler(&b);
    let (page1, total) = cite(&b, json!({"value": "src", "limit": 1}));
    assert_eq!(total, 3, "le rejeu, l'ouverture, le voisin");
    assert_eq!(noms(&page1), ["le voisin"]);
    let (page2, _) = cite(&b, json!({"value": "src", "limit": 1, "offset": 1}));
    assert_eq!(noms(&page2), ["l'ouverture"]);
    let (page3, _) = cite(&b, json!({"value": "src", "limit": 2, "offset": 2}));
    assert_eq!(noms(&page3), ["le rejeu"]);

    // Le plafond du Plan tient.
    let e = b.call("cited_by", json!({"value": "src", "limit": 201})).err().expect("au-delà du plafond");
    assert!(e.contains("200"), "le refus dit le plafond : {e}");

    // En markdown, le rendu par défaut : un texte, la fiche nommée.
    let r = appel(&b, "cited_by", json!({"value": "src/storage/wal"}));
    assert!(r["result"].as_str().is_some_and(|t| t.contains("le rejeu")), "{r}");
}

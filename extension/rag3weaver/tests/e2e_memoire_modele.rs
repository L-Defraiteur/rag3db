//! E2E : **le modèle de la mémoire, lot M1** — la fiche est le sujet, un apport
//! (L1) par ce qu'une source en a dit, la source (L0) gardée telle quelle
//! (`docs/18-octobre-2026-memoire-longue/04-les-fiches-dans-des-fichiers.md`).
//!
//! Sur le vrai gabarit `memory` (ses signaux, donc un service d'embarquement :
//! `RAG3WEAVER_EMBED_SERVICE`) :
//!
//! - une source, une fiche, un apport rangé sur la fiche, sa provenance par une
//!   Ref `doc:` vers `<source>#<section>`, une ancre `path` ; tout se relit ;
//! - **la source ne change jamais** : la réécrire autrement est refusé ;
//! - la fiche se retrouve par la recherche, l'apport aussi ;
//! - les outils d'avant le modèle (`put_memory`, `put_subject`) marchent encore :
//!   M1 ajoute, il ne retire rien (les outils changent avec l'archiviste, M4).
//!
//! Run with: ./run_e2e.sh --test e2e_memoire_modele

#![cfg(feature = "rag3db-native")]

use std::path::Path;

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Le gabarit `memory` dans une copie de `templates/`, son extension vector
/// prise au bâti (comme le banc des fiches classées).
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
    copier(&crate_dir().join("templates"), &dir.join("templates"));
    let manifeste = dir.join("templates/backends/memory/backend.json");
    let mut m: Value = serde_json::from_slice(&std::fs::read(&manifeste).unwrap()).unwrap();
    m["vector_extension"] = json!(crate::common::vector_extension());
    std::fs::write(&manifeste, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    let prepared = PreparedBackend::load(&manifeste).expect("le gabarit memory charge");
    let (embedder, _) = prepared.connect_embedder().unwrap_or_else(|e| {
        panic!("le témoin embarque (bge-m3) : posez RAG3WEAVER_EMBED_SERVICE — un témoin sauté n'est pas vert ({e})")
    });
    prepared
        .open(Box::new(Rag3dbConnection::new(&dir.join("memoire.rag3db")).expect("la base")), Some(embedder))
        .expect("l'ouverture")
}

fn appel(b: &Backend, outil: &str, args: Value) -> Value {
    b.call(outil, args.clone()).unwrap_or_else(|e| panic!("{outil} {args} : {e}"))
}

const SOURCE: &str = "sources/2026-10-18-essai.md";
const CONTENU: &str = "# Le journal\n\n## Reprise\n\nLe journal se rejoue à l'ouverture, en entier, avant toute écriture.\n";
const FICHE: &str = "le moteur de reprise du WAL";

#[test]
#[ignore]
fn une_source_une_fiche_un_apport_et_sa_provenance() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path());

    appel(&b, "put_source", json!({"record": {"chemin": SOURCE, "titre": "Le journal", "contenu": CONTENU, "origine": "envoi"}}));
    appel(&b, "put_fiche", json!({"record": {"name": FICHE, "about": "Comment le moteur rejoue son journal à l'ouverture.", "state": "current"}}));
    let apport = json!({"fiche": FICHE, "source": SOURCE, "section": "Reprise",
                        "texte": "Le journal se rejoue à l'ouverture, en entier, avant toute écriture.", "niveau": "L1", "state": "current"});
    appel(&b, "put_apport", json!({"record": apport}));
    let lien = appel(&b, "relier_apport", json!({"links": [{"from": {"name": FICHE}, "to": {"fiche": FICHE, "source": SOURCE, "section": "Reprise"}}]}));
    assert!(lien.to_string().contains('1'), "un lien posé : {lien}");

    // La provenance : une Ref doc:<source>#<section> ; l'ancre : une Ref path.
    for (genre, description) in [("doc", "un document source et sa section"), ("path", "un chemin du dépôt")] {
        appel(&b, "create_ref_type", json!({"name": genre, "description": description, "forme": "relatif", "confirm": true}));
    }
    appel(&b, "citer_source", json!({"genre": "doc", "valeur": format!("{SOURCE}#Reprise"),
                                       "fiche": {"fiche": FICHE, "source": SOURCE, "section": "Reprise"}}));
    appel(&b, "ancrer_fiche", json!({"genre": "path", "valeur": "src/storage/wal/replay.cpp", "fiche": {"name": FICHE}}));

    let cat = b.catalog.lock().unwrap();
    let compter = |q: &str| cat.execute_raw_with_params(q, &[]).expect(q).rows[0][0].as_i64().unwrap();
    assert_eq!(compter("MATCH (f:Fiche)-[:APPORT]->(a:Apport {niveau: 'L1'}) RETURN count(*)"), 1);
    assert_eq!(compter("MATCH (a:Apport)-[:APPORT_CITE]->(r:Ref {genre: 'doc'}) WHERE r.valeur = 'sources/2026-10-18-essai.md#Reprise' RETURN count(*)"), 1);
    assert_eq!(compter("MATCH (f:Fiche)-[:ANCRE]->(r:Ref {genre: 'path'}) RETURN count(*)"), 1);
    let gardee = cat
        .execute_raw_with_params("MATCH (s:Source {chemin: 'sources/2026-10-18-essai.md'}) RETURN s.contenu", &[])
        .expect("la source");
    assert_eq!(gardee.rows[0][0].as_str(), Some(CONTENU), "la source est gardée telle quelle");
    drop(cat);

    // La fiche et l'apport se retrouvent par la recherche.
    let r = appel(&b, "recall_fiches", json!({"query": "rejouer le journal", "options": {"limit": 3}}));
    assert!(r.to_string().contains(FICHE), "la fiche se retrouve : {r}");
    let r = appel(&b, "recall_apports", json!({"query": "rejoue à l'ouverture", "options": {"limit": 3}}));
    assert!(r.to_string().contains("Reprise"), "l'apport se retrouve : {r}");
}

#[test]
#[ignore]
fn une_source_ne_change_jamais() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path());
    let source = json!({"chemin": SOURCE, "titre": "Le journal", "contenu": CONTENU, "origine": "envoi"});
    appel(&b, "put_source", json!({"record": source}));
    // La même : rejouée à l'identique, acceptée.
    appel(&b, "put_source", json!({"record": source}));
    // Autre contenu sous le même chemin : refusé.
    let mut autre = source.clone();
    autre["contenu"] = json!("# Autre chose\n");
    let e = b.call("put_source", json!({"record": autre})).err().expect("une source ne se réécrit pas");
    assert!(e.contains("immutable"), "le refus dit pourquoi : {e}");
}

#[test]
#[ignore]
fn les_outils_d_avant_le_modele_marchent_encore() {
    let dir = tempfile::tempdir().expect("dossier");
    let b = ouvrir(dir.path());
    appel(&b, "put_subject", json!({"record": {"name": "régime GPU", "about": "le rapport cyclique des passes", "state": "current"}}));
    appel(&b, "put_memory", json!({"record": {"claim": "Un rapport cyclique de 70 % garde l'écran utilisable.", "why": "mesuré", "kind": "fact",
                                              "origin": "inferred", "said_as": "", "reach": "person", "state": "current"}}));
}

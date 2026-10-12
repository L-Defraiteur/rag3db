//! E2E : **un outil d'écriture dit s'il est atomique, et ce qui est resté après
//! un échec** — la forme A de la page de l'enveloppe d'un outil d'écriture
//! (`docs/11-octobre-2026-arbre-principal/03-l-enveloppe-d-un-outil-d-ecriture.md`, §2 et §4).
//!
//! Trois outils des gabarits livrés, chacun avec une faute injectée dans
//! l'insertion de ses lignes (deux variables, jamais l'une sans l'autre : la
//! table et le rang du groupe, sur le modèle de celle du drain des liens) :
//!
//! - un `put` qui échoue dit ce qui est prêt et si la ligne existe, au lieu de
//!   « inspect state before retry » ; un `put` qui réussit dit `atomic: false` et
//!   pourquoi ;
//! - `add_ref` et `create_ref_type` **ne lisaient pas** le rapport de leur
//!   ingestion : un échec compté passait pour un succès (ticket fermé par ce lot).
//!
//! **Une suite à part, et un verrou** : les variables valent pour tout le
//! processus, et les tests d'une suite tournent en parallèle. Ici, chaque test
//! prend le verrou ; aucune autre suite ne voit ces variables (une suite, un
//! processus).
//!
//! Run with: ./run_e2e.sh --test e2e_outils_disent_ce_qui_reste

#![cfg(feature = "rag3db-native")]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{json, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::Rag3dbConnection;

static FAUTES: Mutex<()> = Mutex::new(());

/// Joue `f` avec la faute armée sur `table`, groupe 1, puis la désarme.
fn sous_faute<T>(table: &str, f: impl FnOnce() -> T) -> T {
    std::env::set_var("RAG3WEAVER_TEST_INSERT_FAULT_TABLE", table);
    std::env::set_var("RAG3WEAVER_TEST_INSERT_FAULT_GROUP", "1");
    let r = f();
    std::env::remove_var("RAG3WEAVER_TEST_INSERT_FAULT_TABLE");
    std::env::remove_var("RAG3WEAVER_TEST_INSERT_FAULT_GROUP");
    r
}

fn extension_vecteur() -> String {
    crate::common::vector_extension()
}

/// Une `Fiche` révisée, et `Ref` / `RefType` **recopiées du gabarit `memory`**,
/// retirées de leurs signaux (on mesure ce que les outils disent, pas la
/// recherche).
fn manifeste(dir: &Path) -> PathBuf {
    let gabarits = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let memoire = gabarits.join("backends/memory");
    let gabarit: Value = serde_json::from_slice(&std::fs::read(memoire.join("backend.json")).unwrap()).unwrap();
    let recopier = |nom: &str| {
        let mut e = gabarit["entities"][nom].clone();
        let schema = e["schema"].as_str().unwrap().to_string();
        e["schema"] = json!(memoire.join(schema).display().to_string());
        e["config"]["signals"] = json!([]);
        e
    };
    std::fs::write(
        dir.join("fiche.json"),
        serde_json::to_vec_pretty(&json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object", "additionalProperties": false,
            "required": ["titre", "corps", "revision", "created_at", "updated_at"],
            "properties": {
                "titre": {"type": "string", "minLength": 1}, "corps": {"type": "string", "minLength": 1},
                "revision": {"type": "integer"}, "created_at": {"type": "integer"}, "updated_at": {"type": "integer"}
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let put = gabarits.join("backends/notebook/graphs/put.mmd");
    let m = json!({
        "version": 1, "name": "outils-honnetes", "database": "base.rag3db",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": extension_vecteur(),
        "entities": {
            "Fiche": {
                "schema": "fiche.json",
                "config": {"fields": {"titre": {"type": "string", "isTitle": true}, "corps": {"type": "string", "isContent": true}},
                           "hashsafe": ["titre"], "signals": []},
                "writes": {"created_at": "created_at", "updated_at": "updated_at", "revision": "revision"}
            },
            "Ref": recopier("Ref"),
            "RefType": recopier("RefType")
        },
        "relations": {"CITE": {"from": "Fiche", "to": "Ref"}},
        "tools": {
            "put_fiche": {"graph": put, "bindings": {"entity": "Fiche"},
                          "input_payloads": {"record": {"entity": "Fiche", "view": "editable"}}},
            "add_ref": {"graph": memoire.join("graphs/add_ref.mmd"),
                        "bindings": {"refs_entity": "Ref", "types_entity": "RefType", "from_entity": "Fiche", "relation": "CITE"}},
            "create_ref_type": {"graph": memoire.join("graphs/create_ref_type.mmd"), "bindings": {"types_entity": "RefType"}}
        }
    });
    let chemin = dir.join("backend.json");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&m).unwrap()).unwrap();
    chemin
}

fn ouvrir(chemin: &Path) -> Backend {
    let prepared = PreparedBackend::load(chemin).expect("le manifeste charge");
    let base = prepared.path(&prepared.manifest.database);
    prepared
        .open(Box::new(Rag3dbConnection::new(&base).expect("la base")), None)
        .expect("l'ouverture")
}

fn compter(backend: &Backend, motif: &str) -> i64 {
    backend.catalog.lock().unwrap().execute_raw_with_params(motif, &[]).expect("le compte").rows[0][0]
        .as_i64()
        .expect("un entier")
}

#[test]
#[ignore]
fn un_put_dit_qu_il_n_est_pas_atomique_et_ce_qui_reste_apres_un_echec() {
    let _verrou = FAUTES.lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));

    let r = backend
        .call("put_fiche", json!({"record": {"titre": "une", "corps": "x"}}))
        .expect("le put réussit");
    assert_eq!(r["result"]["atomic"], false, "{r}");
    assert!(r["result"]["atomic_reason"].as_str().is_some_and(|s| !s.is_empty()), "la raison est dite : {r}");

    let refus = sous_faute("Fiche", || backend.call("put_fiche", json!({"record": {"titre": "deux", "corps": "y"}})))
        .expect_err("la ligne n'a pas pu s'écrire");
    assert!(
        refus.contains("ce qui est prêt) = rien;") && refus.contains("does not exist"),
        "l'échec dit ce qui est prêt et que la ligne n'existe pas — plus « inspect state » : {refus}"
    );
    assert!(!refus.contains("inspect state"), "{refus}");
    assert_eq!(compter(&backend, "MATCH (f:Fiche {titre: 'deux'}) RETURN count(f)"), 0);
}

#[test]
#[ignore]
fn create_ref_type_ne_dit_plus_cree_un_genre_qui_n_est_pas_ecrit() {
    let _verrou = FAUTES.lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    let refus = sous_faute("RefType", || {
        backend.call("create_ref_type", json!({"name": "fichier", "description": "un chemin", "forme": "relatif", "confirm": true}))
    })
    .expect_err("le genre n'a pas pu s'écrire");
    assert!(refus.contains("ne s'est pas écrit entier") && refus.contains("fichier"), "{refus}");

    let r = backend
        .call("create_ref_type", json!({"name": "fichier", "description": "un chemin", "forme": "relatif", "confirm": true}))
        .expect("sans faute");
    assert_eq!(r["result"]["atomic"], false, "le succès dit qu'il n'est pas atomique : {r}");
}

#[test]
#[ignore]
fn add_ref_ne_lie_pas_une_reference_qui_n_est_pas_ecrite() {
    let _verrou = FAUTES.lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    backend.call("put_fiche", json!({"record": {"titre": "une", "corps": "x"}})).expect("la fiche");
    backend
        .call("create_ref_type", json!({"name": "fichier", "description": "un chemin", "forme": "relatif", "confirm": true}))
        .expect("le genre");

    let r = sous_faute("Ref", || {
        backend.call("add_ref", json!({"genre": "fichier", "valeur": "src/a.rs", "fiche": {"titre": "une"}}))
    })
    .expect("une réponse");
    assert_eq!(r["result"]["executed"], false, "{r}");
    assert!(
        r["result"]["why"].as_str().unwrap_or("").contains("ne s'est pas écrite entière"),
        "l'échec compté de l'ingestion est dit, au lieu de passer pour un succès : {r}"
    );
    assert_eq!(compter(&backend, "MATCH ()-[r:CITE]->() RETURN count(r)"), 0, "rien n'est lié");

    let r = backend
        .call("add_ref", json!({"genre": "fichier", "valeur": "src/a.rs", "fiche": {"titre": "une"}}))
        .expect("sans faute");
    assert_eq!(r["result"]["executed"], true, "{r}");
    assert_eq!(r["result"]["atomic"], false, "{r}");
    assert_eq!(compter(&backend, "MATCH ()-[r:CITE]->() RETURN count(r)"), 1);
}


/// **`add_ref` dans la cellule de l'appel** (la cellule des bouts, migration (a)) :
/// une session d'une autre cellule que celle du catalogue écrit sa fiche dans sa
/// cellule ; `add_ref` doit l'y trouver, et y écrire la référence. Avant, il
/// calculait dans la cellule du catalogue : la fiche n'était « pas là ».
#[test]
#[ignore]
fn add_ref_cherche_la_fiche_et_ecrit_la_reference_dans_la_cellule_de_l_appel() {
    let _verrou = FAUTES.lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    let alpha = rag3weaver::scope::Scope::new("acme", "alpha");
    backend
        .call("create_ref_type", json!({"name": "fichier", "description": "un chemin", "forme": "relatif", "confirm": true}))
        .expect("le genre");
    backend.call_in(&alpha, "put_fiche", json!({"record": {"titre": "une", "corps": "x"}})).expect("la fiche d'alpha");
    let r = backend
        .call_in(&alpha, "add_ref", json!({"genre": "fichier", "valeur": "src/a.rs", "fiche": {"titre": "une"}}))
        .expect("une réponse");
    assert_eq!(r["result"]["executed"], true, "la fiche d'alpha est trouvée dans alpha : {r}");
    assert_eq!(compter(&backend, "MATCH (f:Fiche)-[r:CITE]->(x:Ref) WHERE f._project = 'alpha' AND x._project = 'alpha' RETURN count(r)"), 1);
}

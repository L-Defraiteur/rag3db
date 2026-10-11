//! E2E : **ce que la garde de `RelationBatchNode` protège — établi, pas supposé.**
//!
//! `RelationBatchNode` refuse de poser une relation dès qu'un bout est une entité
//! **gérée** : cycle de vie, révision, dates serveur, dates de transition ou
//! immuabilité (`backend_nodes.rs`, « snapshot links cannot modify relations of
//! managed entities »). Aucun test ne l'épinglait, aucun commit ne la justifiait :
//! elle est arrivée le 20 septembre 2026 dans un commit de sauvegarde d'expériences
//! (`ab95c3a2d`), copie de la garde d'`EntityBatchNode` — dont la clause jumelle de
//! cycle de vie a été retirée le 2 octobre comme « une prudence » (`213eeb214`).
//!
//! Avant d'écrire un nœud de lien générique à côté d'elle (lot accepté par Lucie le
//! 11 octobre : la face dataflow de `Write::Link`, valable pour toute paire
//! d'entités déclarées), on établit **ce qu'elle protège**. Règle du dépôt : une
//! garde ne se relâche pas sans prouver ce qu'elle mesurait, champ à champ.
//!
//! Ce témoin fait trois choses :
//!
//! 1. **il épingle la garde telle qu'elle est** : un outil déclaré sur
//!    `link_snapshot.mmd` refuse deux bouts gérés, en le disant ;
//! 2. **il pose la même relation par le chemin que la garde barre** — le verbe du
//!    catalogue (`Catalog::link`), celui qu'`AddRefNode` emploie déjà, depuis la
//!    liste blanche de base, entre deux entités gérées du gabarit `memory` — et
//!    relit chaque bout **champ à champ** avant et après ;
//! 3. **il vérifie que chaque garantie déclarée tient encore après le lien** : la
//!    révision attendue est la même, l'immuable rejoue à l'identique, la machine à
//!    états refuse toujours un passage non déclaré.
//!
//! Si ces trois points tiennent, la garde ne protège **aucune garantie déclarée** :
//! elle délimite une portée (« liens d'un instantané entre entités brutes »), et
//! c'est ce que la page du nœud générique doit dire avant le code.
//!
//! Run with: ./run_e2e.sh --test e2e_garde_des_liens

#![cfg(feature = "rag3db-native")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use rag3weaver::backend::{Backend, PreparedBackend};
use rag3weaver::connection::CypherValue;
use rag3weaver::Rag3dbConnection;

fn extension_vecteur() -> String {
    let racine = std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .display()
            .to_string()
    });
    let chemin = format!("{racine}/extension/vector/build/libvector.rag3db_extension");
    assert!(
        Path::new(&chemin).is_file(),
        "l'extension vectorielle manque : {chemin} — posez RAG3DB_ROOT sur l'arbre où le \
         moteur est bâti"
    );
    chemin
}

/// Un schéma JSON à partir de ses propriétés ; tout est requis.
fn schema(proprietes: Value) -> Value {
    let requis: Vec<String> = proprietes.as_object().unwrap().keys().cloned().collect();
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "additionalProperties": false,
        "required": requis,
        "properties": proprietes
    })
}

fn texte() -> Value {
    json!({"type": "string", "minLength": 1})
}
fn entier() -> Value {
    json!({"type": "integer"})
}

/// Trois entités, et **toutes les formes de gestion** que la garde regarde :
///
/// - `Fiche` : révision, dates serveur, cycle de vie, date de transition ;
/// - `Sujet` : révision et dates serveur, sans cycle de vie ;
/// - `Piece` : **immuable** (insérée puis rejouée à l'identique, jamais réécrite).
fn manifeste(dir: &Path) -> PathBuf {
    let gabarits = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let put = gabarits.join("backends/notebook/graphs/put.mmd");
    let get = gabarits.join("backends/notebook/graphs/get.mmd");
    let lien_d_instantane = gabarits.join("tools/link_snapshot.mmd");

    let ecrire = |nom: &str, v: Value| {
        std::fs::write(dir.join(nom), serde_json::to_vec_pretty(&v).unwrap()).unwrap();
    };
    ecrire(
        "fiche.json",
        schema(json!({
            "titre": texte(), "corps": texte(),
            "etat": {"type": "string", "enum": ["courant", "a_revoir"]},
            "revision": entier(), "created_at": entier(), "updated_at": entier(),
            "revue_le": {"type": ["integer", "null"]}
        })),
    );
    ecrire(
        "sujet.json",
        schema(json!({
            "titre": texte(), "corps": texte(),
            "revision": entier(), "created_at": entier(), "updated_at": entier()
        })),
    );
    ecrire(
        "piece.json",
        schema(json!({"cle": texte(), "contenu": texte(), "created_at": entier()})),
    );

    let config = |titre: &str, contenu: &str, autres: &[&str], cycle: Option<Value>| {
        let mut fields = Map::new();
        fields.insert(titre.into(), json!({"type": "string", "isTitle": true}));
        fields.insert(contenu.into(), json!({"type": "string", "isContent": true}));
        for a in autres {
            fields.insert((*a).into(), json!({"type": "string"}));
        }
        let mut c = json!({"fields": fields, "hashsafe": [titre], "signals": []});
        if let Some(cycle) = cycle {
            c["lifecycle"] = cycle;
        }
        c
    };
    let manifest = json!({
        "version": 1,
        "name": "garde-des-liens",
        "database": "base.rag3db",
        "embeddings": {"address": "127.0.0.1:9", "model": "none", "dimensions": 8},
        "vector_extension": extension_vecteur(),
        "entities": {
            "Fiche": {
                "schema": "fiche.json",
                "config": config("titre", "corps", &["etat"], Some(json!({
                    "field": "etat", "initial": "courant",
                    "transitions": [{"name": "revoir", "from": "courant", "to": "a_revoir"}]
                }))),
                "writes": {
                    "created_at": "created_at", "updated_at": "updated_at", "revision": "revision",
                    "transition_dates": [{"field": "etat", "to": "a_revoir", "timestamp_field": "revue_le"}]
                }
            },
            "Sujet": {
                "schema": "sujet.json",
                "config": config("titre", "corps", &[], None),
                "writes": {"created_at": "created_at", "updated_at": "updated_at", "revision": "revision"}
            },
            "Piece": {
                "schema": "piece.json",
                "config": config("cle", "contenu", &[], None),
                "writes": {"immutable": true, "created_at": "created_at"}
            }
        },
        "relations": {
            "ANCRE": {"from": "Fiche", "to": "Sujet"},
            "PREUVE": {"from": "Fiche", "to": "Piece"}
        },
        "tools": {
            "put_fiche": {"graph": put, "bindings": {"entity": "Fiche"},
                          "input_payloads": {"record": {"entity": "Fiche", "view": "editable"}}},
            "put_sujet": {"graph": put, "bindings": {"entity": "Sujet"},
                          "input_payloads": {"record": {"entity": "Sujet", "view": "editable"}}},
            "put_piece": {"graph": put, "bindings": {"entity": "Piece"},
                          "input_payloads": {"record": {"entity": "Piece", "view": "editable"}}},
            "get_fiche": {"graph": get, "bindings": {"entity": "Fiche"},
                          "input_payloads": {"record": {"entity": "Fiche", "view": "identity"}}},
            "get_sujet": {"graph": get, "bindings": {"entity": "Sujet"},
                          "input_payloads": {"record": {"entity": "Sujet", "view": "identity"}}},
            "get_piece": {"graph": get, "bindings": {"entity": "Piece"},
                          "input_payloads": {"record": {"entity": "Piece", "view": "identity"}}},
            // La garde, telle qu'elle est : l'outil d'instantané sur la même relation.
            "lier_par_instantane": {"graph": lien_d_instantane, "bindings": {"relation": "ANCRE"}}
        }
    });
    let chemin = dir.join("backend.json");
    std::fs::write(&chemin, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    chemin
}

fn ouvrir(chemin: &Path) -> Backend {
    let prepared = PreparedBackend::load(chemin).expect("le manifeste charge");
    assert!(!prepared.needs_embeddings());
    let base = prepared.path(&prepared.manifest.database);
    let conn = Rag3dbConnection::new(&base).expect("la base");
    prepared.open(Box::new(conn), None).expect("l'ouverture")
}

/// Le record tel que l'outil `get_*` le rend — la vérité vue par un client.
fn lire(backend: &Backend, outil: &str, identite: Value) -> Map<String, Value> {
    let r = backend
        .call(outil, json!({"record": identite}))
        .unwrap_or_else(|e| panic!("{outil} : {e}"));
    r["result"]["data"]
        .as_object()
        .unwrap_or_else(|| panic!("{outil} ne rend pas de données : {r}"))
        .clone()
}

fn uuid(backend: &Backend, entite: &str, cle: &str, valeur: &str) -> String {
    let cat = backend.catalog.lock().unwrap();
    let mut ligne = BTreeMap::new();
    ligne.insert(cle.to_string(), CypherValue::String(valeur.to_string()));
    cat.entity_uuid(entite, &ligne).expect("l'uuid recomposé")
}

fn compter(backend: &Backend, relation: &str) -> i64 {
    let cat = backend.catalog.lock().unwrap();
    let r = cat
        .execute_raw_with_params(&format!("MATCH ()-[r:{relation}]->() RETURN count(r)"), &[])
        .expect("le compte");
    r.rows[0][0].as_i64().expect("un entier")
}

/// Compare deux lectures d'un même record champ à champ, et rend **tous** les
/// écarts d'un coup (pas le premier).
fn ecarts(avant: &Map<String, Value>, apres: &Map<String, Value>) -> Vec<String> {
    let mut cles: Vec<&String> = avant.keys().chain(apres.keys()).collect();
    cles.sort();
    cles.dedup();
    cles.into_iter()
        .filter(|k| avant.get(*k) != apres.get(*k))
        .map(|k| format!("{k} : {:?} → {:?}", avant.get(k), apres.get(k)))
        .collect()
}

#[test]
#[ignore]
fn la_garde_des_liens_ne_protege_aucune_garantie_declaree() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));

    backend
        .call("put_fiche", json!({"record": {"titre": "duty cycle", "corps": "70 %", "etat": "courant"}}))
        .expect("la fiche");
    backend
        .call("put_sujet", json!({"record": {"titre": "régime GPU", "corps": "les cartes"}}))
        .expect("le sujet");
    backend
        .call("put_piece", json!({"record": {"cle": "mesure-12", "contenu": "à 100 % le compositeur fige"}}))
        .expect("la pièce");

    let fiche_id = json!({"titre": "duty cycle"});
    let sujet_id = json!({"titre": "régime GPU"});
    let piece_id = json!({"cle": "mesure-12"});

    // ── 1. La garde, telle qu'elle est ───────────────────────────────────
    let refus = backend
        .call(
            "lier_par_instantane",
            json!({"links": [{"from": {"titre": "duty cycle"}, "to": {"titre": "régime GPU"}}]}),
        )
        .expect_err("RelationBatchNode refuse deux bouts gérés aujourd'hui");
    assert!(
        refus.contains("managed entities"),
        "le refus de la garde, mot pour mot : {refus}"
    );
    assert_eq!(compter(&backend, "ANCRE"), 0, "le refus n'a rien écrit");

    // ── 2. La même relation par le chemin que la garde barre ──────────────
    let avant = [
        lire(&backend, "get_fiche", fiche_id.clone()),
        lire(&backend, "get_sujet", sujet_id.clone()),
        lire(&backend, "get_piece", piece_id.clone()),
    ];
    {
        let f = uuid(&backend, "Fiche", "titre", "duty cycle");
        let s = uuid(&backend, "Sujet", "titre", "régime GPU");
        let p = uuid(&backend, "Piece", "cle", "mesure-12");
        let mut cat = backend.catalog.lock().unwrap();
        cat.link("ANCRE", f.clone(), s, BTreeMap::new()).expect("ANCRE entre deux bouts gérés");
        cat.link("PREUVE", f, p, BTreeMap::new()).expect("PREUVE vers un bout immuable");
    }
    assert_eq!(compter(&backend, "ANCRE"), 1, "le lien est posé");
    assert_eq!(compter(&backend, "PREUVE"), 1, "le lien est posé");
    let apres = [
        lire(&backend, "get_fiche", fiche_id.clone()),
        lire(&backend, "get_sujet", sujet_id.clone()),
        lire(&backend, "get_piece", piece_id.clone()),
    ];
    let tous: Vec<String> = ["Fiche", "Sujet", "Piece"]
        .iter()
        .zip(avant.iter().zip(apres.iter()))
        .flat_map(|(nom, (a, b))| ecarts(a, b).into_iter().map(move |e| format!("{nom}.{e}")))
        .collect();
    assert!(
        tous.is_empty(),
        "poser un lien ne doit changer AUCUN champ d'un bout — écarts : {tous:#?}"
    );

    // ── 3. Chaque garantie déclarée tient encore ─────────────────────────
    //
    // La révision : l'édition attendue sur la révision lue AVANT le lien passe,
    // donc le lien ne l'a pas fait monter.
    let r = backend
        .call(
            "put_sujet",
            json!({"record": {"titre": "régime GPU", "corps": "les cartes, et le budget"},
                   "expected_revision": avant[1]["revision"]}),
        )
        .expect("la révision lue avant le lien est toujours la bonne");
    assert_eq!(r["result"]["data"]["revision"], 2, "{r}");

    // L'immuable : rejoué à l'identique, il rend « inchangé » ; modifié, il refuse.
    let r = backend
        .call("put_piece", json!({"record": {"cle": "mesure-12", "contenu": "à 100 % le compositeur fige"}}))
        .expect("le rejeu identique d'un immuable");
    assert_eq!(r["result"]["unchanged"], true, "le lien n'a rien changé de l'immuable : {r}");
    let refus = backend
        .call("put_piece", json!({"record": {"cle": "mesure-12", "contenu": "autre chose"}}))
        .expect_err("un immuable modifié se refuse toujours");
    assert!(refus.contains("immutable"), "{refus}");

    // La machine à états : la transition déclarée passe et date sa revue ; le
    // retour non déclaré se refuse toujours.
    let r = backend
        .call(
            "put_fiche",
            json!({"record": {"titre": "duty cycle", "corps": "70 %", "etat": "a_revoir"},
                   "expected_revision": avant[0]["revision"]}),
        )
        .expect("la transition déclarée, après le lien");
    assert_eq!(r["result"]["data"]["etat"], "a_revoir", "{r}");
    assert!(r["result"]["data"]["revue_le"].is_i64(), "la date de transition est posée : {r}");
    let refus = backend
        .call(
            "put_fiche",
            json!({"record": {"titre": "duty cycle", "corps": "70 %", "etat": "courant"},
                   "expected_revision": r["result"]["data"]["revision"]}),
        )
        .expect_err("a_revoir → courant n'est pas déclaré");
    assert!(!refus.is_empty());

    // Et les liens sont toujours là après ces écritures.
    assert_eq!(compter(&backend, "ANCRE"), 1);
    assert_eq!(compter(&backend, "PREUVE"), 1);
}

/// **Un lien vers un bout absent rend `Ok` et n'écrit rien** — mesuré le
/// 11 octobre 2026 par ce test, qui était d'abord une sonde.
///
/// `Catalog::link` met le lien en file et le draine ; la relation se pose par
/// `MATCH (a), (b) MERGE (a)-[r]->(b)`, et un `MATCH` qui ne trouve pas un bout ne
/// produit rien — sans erreur. Le verbe rend donc `Ok` pour un lien qui n'existe
/// pas. `RelationBatchNode` s'en garde en vérifiant l'existence des bouts
/// (`get_many`) ; **`AddRefNode` ne s'en garde pas** et annonce
/// `executed: true, lien: {…}` pour une fiche qui n'existe pas (ticket
/// `2026-10-11-un-lien-vers-un-bout-absent-rend-ok-sans-rien-ecrire.md`).
///
/// Ce test épingle le comportement **tel qu'il est**, pour que le nœud de lien
/// générique sache qu'il doit vérifier l'existence lui-même. Le jour où le verbe
/// refuse un bout absent, il rougit : le retourner, et fermer le ticket.
#[test]
#[ignore]
fn un_lien_vers_un_bout_absent_rend_ok_et_n_ecrit_rien() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    backend
        .call("put_fiche", json!({"record": {"titre": "seule", "corps": "x", "etat": "courant"}}))
        .expect("la fiche");
    let f = uuid(&backend, "Fiche", "titre", "seule");
    let absent = uuid(&backend, "Sujet", "titre", "jamais écrit");
    let rendu = {
        let mut cat = backend.catalog.lock().unwrap();
        cat.link("ANCRE", f, absent, BTreeMap::new())
    };
    assert!(
        rendu.is_ok(),
        "le verbe refuse maintenant un bout absent — c'est le correctif attendu : retourner ce \
         test et fermer le ticket ({:?})",
        rendu.err()
    );
    assert_eq!(compter(&backend, "ANCRE"), 0, "et rien n'est écrit en base");
}

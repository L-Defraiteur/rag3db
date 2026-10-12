//! E2E : **ce que la garde de `RelationBatchNode` protégeait — établi, puis la garde retirée.**
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
//! **Premier temps** (`49142d58f`) : la garde épinglée telle qu'elle était, puis la
//! même relation posée par le chemin qu'elle barrait (`Catalog::link`) entre une
//! entité à cycle de vie, révision, dates serveur et date de transition, une
//! entité révisée et une entité **immuable** — aucun champ d'aucun bout ne
//! changeait, et chaque garantie déclarée tenait après. Elle ne protégeait donc
//! aucune garantie déclarée.
//!
//! **Second temps, ce fichier aujourd'hui** (marche 1 de la page
//! `docs/11-octobre-2026-memoire-longue/01-un-lien-entre-deux-identites-declarees.md`) :
//! la garde est retirée, et **les mêmes liens passent par l'outil**, avec les mêmes
//! relectures champ à champ. Plus ce que le nœud général doit faire de neuf :
//!
//! - un bout absent refuse le lot entier, nommé par l'identité que l'appelant a
//!   donnée ;
//! - « existe » veut dire en base **ou en file**, et une suppression en file
//!   l'emporte ;
//! - les propriétés sont permises quand la relation les déclare, et une propriété
//!   non déclarée se refuse en la nommant ;
//! - `add_ref` vers une fiche qui n'existe pas ne prétend plus avoir lié.
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
    let chemin = crate::common::vector_extension();
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
    let lien = gabarits.join("tools/link_snapshot.mmd");
    let memoire = gabarits.join("backends/memory");
    // Ref et RefType **recopiées du gabarit livré** puis retirées de leurs
    // signaux — la leçon du ticket `--demon` : copier un gabarit et en retirer,
    // jamais deviner ce qui est exigé.
    let gabarit_memoire: Value =
        serde_json::from_slice(&std::fs::read(memoire.join("backend.json")).unwrap()).unwrap();
    let recopier = |nom: &str| {
        let mut e = gabarit_memoire["entities"][nom].clone();
        let schema = e["schema"].as_str().unwrap().to_string();
        e["schema"] = json!(memoire.join(schema).display().to_string());
        e["config"]["signals"] = json!([]);
        e
    };

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
            },
            "Ref": recopier("Ref"),
            "RefType": recopier("RefType")
        },
        "relations": {
            // Une garde d'états déclarée : on n'ancre ni ne cite depuis une fiche
            // « à revoir » (marche 2).
            "ANCRE": {"from": "Fiche", "to": "Sujet", "from_states": ["courant"]},
            "PREUVE": {"from": "Fiche", "to": "Piece"},
            // Une relation qui déclare une propriété.
            "NOTE": {"from": "Fiche", "to": "Sujet",
                     "properties": {"poids": {"type": "string"}, "rang": {"type": "integer"}}},
            "CITE": {"from": "Fiche", "to": "Ref", "from_states": ["courant"]}
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
            "ancrer": {"graph": lien, "bindings": {"relation": "ANCRE"}},
            "prouver": {"graph": lien, "bindings": {"relation": "PREUVE"}},
            "noter": {"graph": lien, "bindings": {"relation": "NOTE"}},
            "add_ref": {"graph": memoire.join("graphs/add_ref.mmd"),
                        "bindings": {"refs_entity": "Ref", "types_entity": "RefType",
                                     "from_entity": "Fiche", "relation": "CITE"}},
            "create_ref_type": {"graph": memoire.join("graphs/create_ref_type.mmd"),
                                "bindings": {"types_entity": "RefType"}}
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

fn ecrire_les_trois(backend: &Backend) {
    backend
        .call("put_fiche", json!({"record": {"titre": "duty cycle", "corps": "70 %", "etat": "courant"}}))
        .expect("la fiche");
    backend
        .call("put_sujet", json!({"record": {"titre": "régime GPU", "corps": "les cartes"}}))
        .expect("le sujet");
    backend
        .call("put_piece", json!({"record": {"cle": "mesure-12", "contenu": "à 100 % le compositeur fige"}}))
        .expect("la pièce");
}

#[test]
#[ignore]
fn deux_bouts_geres_se_lient_par_l_outil_et_leurs_garanties_tiennent() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    ecrire_les_trois(&backend);
    let fiche_id = json!({"titre": "duty cycle"});
    let sujet_id = json!({"titre": "régime GPU"});
    let piece_id = json!({"cle": "mesure-12"});

    // ── 1. Les liens passent par l'outil, là où la garde les refusait ────
    let avant = [
        lire(&backend, "get_fiche", fiche_id.clone()),
        lire(&backend, "get_sujet", sujet_id.clone()),
        lire(&backend, "get_piece", piece_id.clone()),
    ];
    let r = backend
        .call("ancrer", json!({"links": [{"from": fiche_id, "to": sujet_id}]}))
        .expect("ANCRE entre deux bouts gérés, par l'outil");
    assert_eq!(r["result"]["verified"], 1, "{r}");
    assert_eq!(r["result"]["atomic"], true, "un lien, une instruction : atomique par le moteur, et dit : {r}");
    backend
        .call("prouver", json!({"links": [{"from": fiche_id, "to": piece_id}]}))
        .expect("PREUVE vers un bout immuable, par l'outil");
    assert_eq!(compter(&backend, "ANCRE"), 1, "le lien est posé");
    assert_eq!(compter(&backend, "PREUVE"), 1, "le lien est posé");
    // Reposé : idempotent, pas d'arête en double.
    backend
        .call("ancrer", json!({"links": [{"from": fiche_id, "to": sujet_id}]}))
        .expect("reposer le même lien");
    assert_eq!(compter(&backend, "ANCRE"), 1, "reposer ne double pas");

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

    // ── 2. Chaque garantie déclarée tient encore ─────────────────────────
    let r = backend
        .call(
            "put_sujet",
            json!({"record": {"titre": "régime GPU", "corps": "les cartes, et le budget"},
                   "expected_revision": avant[1]["revision"]}),
        )
        .expect("la révision lue avant le lien est toujours la bonne");
    assert_eq!(r["result"]["data"]["revision"], 2, "{r}");

    let r = backend
        .call("put_piece", json!({"record": {"cle": "mesure-12", "contenu": "à 100 % le compositeur fige"}}))
        .expect("le rejeu identique d'un immuable");
    assert_eq!(r["result"]["unchanged"], true, "le lien n'a rien changé de l'immuable : {r}");
    let refus = backend
        .call("put_piece", json!({"record": {"cle": "mesure-12", "contenu": "autre chose"}}))
        .expect_err("un immuable modifié se refuse toujours");
    assert!(refus.contains("immutable"), "{refus}");

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

    assert_eq!(compter(&backend, "ANCRE"), 1);
    assert_eq!(compter(&backend, "PREUVE"), 1);
}

#[test]
#[ignore]
fn un_bout_absent_refuse_le_lot_entier_en_le_nommant() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    ecrire_les_trois(&backend);
    let refus = backend
        .call(
            "ancrer",
            json!({"links": [
                {"from": {"titre": "duty cycle"}, "to": {"titre": "régime GPU"}},
                {"from": {"titre": "duty cycle"}, "to": {"titre": "jamais écrit"}}
            ]}),
        )
        .expect_err("un bout absent refuse le lot");
    assert!(
        refus.contains("Sujet") && refus.contains("jamais écrit") && refus.contains("no links"),
        "le refus nomme le bout par l'identité donnée, et dit que rien n'est écrit : {refus}"
    );
    assert_eq!(compter(&backend, "ANCRE"), 0, "ni le bon lien ni le mauvais : le lot entier");
}

#[test]
#[ignore]
fn exister_c_est_en_base_ou_en_file_et_une_suppression_en_file_l_emporte() {
    use rag3weaver::disponibilite::Disponibilites;
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    ecrire_les_trois(&backend);
    let f = uuid(&backend, "Fiche", "titre", "duty cycle");
    let s = uuid(&backend, "Sujet", "titre", "régime GPU");
    let mut cat = backend.catalog.lock().unwrap();

    // En base : présent.
    assert_eq!(cat.bouts_absents("ANCRE", &[(f.clone(), s.clone())]).unwrap(), vec![]);

    // En file, pas encore drainé : présent.
    let mut nouveau = BTreeMap::new();
    nouveau.insert("titre".to_string(), CypherValue::String("en file".into()));
    nouveau.insert("corps".to_string(), CypherValue::String("pas encore posé".into()));
    cat.create_jusqu_a("Sujet", nouveau, Disponibilites::AUCUNE).expect("mis en file");
    let en_file = {
        let mut l = BTreeMap::new();
        l.insert("titre".to_string(), CypherValue::String("en file".into()));
        cat.entity_uuid("Sujet", &l).unwrap()
    };
    assert_eq!(
        cat.bouts_absents("ANCRE", &[(f.clone(), en_file.clone())]).unwrap(),
        vec![],
        "un bout créé dans le même run et pas encore drainé n'est pas absent"
    );

    // En base mais en file de suppression : absent.
    cat.delete_jusqu_a("Sujet", &s, Disponibilites::AUCUNE).expect("suppression en file");
    assert_eq!(
        cat.bouts_absents("ANCRE", &[(f.clone(), s.clone())]).unwrap(),
        vec![("Sujet".to_string(), s.clone())],
        "une suppression en file l'emporte sur la présence en base"
    );
}

#[test]
#[ignore]
fn une_propriete_declaree_se_pose_et_une_autre_se_refuse_en_la_nommant() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    ecrire_les_trois(&backend);
    let paire = json!({"from": {"titre": "duty cycle"}, "to": {"titre": "régime GPU"}});

    let mut avec = paire.clone();
    avec["props"] = json!({"poids": "fort"});
    backend.call("noter", json!({"links": [avec]})).expect("une propriété déclarée");
    let r = backend
        .catalog
        .lock()
        .unwrap()
        .execute_raw_with_params("MATCH ()-[r:NOTE]->() RETURN r.poids", &[])
        .expect("la relecture");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0].as_str(), Some("fort"), "la propriété est posée");

    let mut faux = paire.clone();
    faux["props"] = json!({"couleur": "bleu"});
    let refus = backend.call("noter", json!({"links": [faux]})).expect_err("non déclarée");
    assert!(refus.contains("couleur") && refus.contains("poids"), "{refus}");

    // Le type déclaré se tient ici, pas dans le moteur.
    let mut mal_typee = paire.clone();
    mal_typee["props"] = json!({"rang": "trois"});
    let refus = backend.call("noter", json!({"links": [mal_typee]})).expect_err("rang est un entier");
    assert!(refus.contains("NOTE.rang"), "le refus nomme la propriété mal typée : {refus}");

    let mut sur_ancre = paire.clone();
    sur_ancre["props"] = json!({"poids": "fort"});
    let refus = backend.call("ancrer", json!({"links": [sur_ancre]})).expect_err("ANCRE n'en déclare aucune");
    assert!(refus.contains("declares no properties"), "{refus}");
}

/// **Une paire donnée deux fois** (relecture de rag3db-97) : identique, elle se
/// fond ; avec d'autres propriétés, le lot se refuse. Garder la première en
/// silence aurait été la perte même que le nœud ferme.
#[test]
#[ignore]
fn une_paire_donnee_deux_fois_se_fond_ou_se_refuse() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    ecrire_les_trois(&backend);
    let lien = |poids: &str| {
        json!({"from": {"titre": "duty cycle"}, "to": {"titre": "régime GPU"}, "props": {"poids": poids}})
    };
    backend
        .call("noter", json!({"links": [lien("fort"), lien("fort")]}))
        .expect("deux fois la même paire, mêmes propriétés : fondue");
    assert_eq!(compter(&backend, "NOTE"), 1);

    let refus = backend
        .call("noter", json!({"links": [lien("fort"), lien("faible")]}))
        .expect_err("deux fois la même paire, propriétés différentes");
    assert!(refus.contains("given twice"), "{refus}");
    let r = backend
        .catalog
        .lock()
        .unwrap()
        .execute_raw_with_params("MATCH ()-[r:NOTE]->() RETURN r.poids", &[])
        .expect("la relecture");
    assert_eq!(r.rows[0][0].as_str(), Some("fort"), "le refus n'a rien réécrit");
}

/// **Le bout `to` en file** (relecture de rag3db-97) : la graine du drain est
/// le bout `from` ; il faut prouver que sa fermeture emporte l'autre bout, créé
/// dans le même run et pas encore posé.
#[test]
#[ignore]
fn un_bout_to_encore_en_file_est_pose_avec_le_lien() {
    use rag3weaver::disponibilite::Disponibilites;
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    ecrire_les_trois(&backend);
    {
        let mut cat = backend.catalog.lock().unwrap();
        let mut nouveau = BTreeMap::new();
        nouveau.insert("titre".to_string(), CypherValue::String("en file".into()));
        nouveau.insert("corps".to_string(), CypherValue::String("pas encore posé".into()));
        cat.create_jusqu_a("Sujet", nouveau, Disponibilites::AUCUNE).expect("mis en file");
    }
    backend
        .call("ancrer", json!({"links": [{"from": {"titre": "duty cycle"}, "to": {"titre": "en file"}}]}))
        .expect("le bout `to` en file compte comme présent");
    assert_eq!(compter(&backend, "ANCRE"), 1, "le lien est posé avec son bout");
    let r = backend
        .catalog
        .lock()
        .unwrap()
        .execute_raw_with_params("MATCH (s:Sujet {titre: 'en file'}) RETURN count(s)", &[])
        .expect("la relecture");
    assert_eq!(r.rows[0][0].as_i64(), Some(1), "et le bout est en base");
}

#[test]
#[ignore]
fn add_ref_vers_une_fiche_absente_ne_pretend_plus_avoir_lie() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    ecrire_les_trois(&backend);
    backend
        .call("create_ref_type", json!({"name": "fichier", "description": "un chemin", "forme": "relatif", "confirm": true}))
        .expect("le genre");

    let r = backend
        .call("add_ref", json!({"genre": "fichier", "valeur": "src/a.rs", "fiche": {"titre": "pas de fiche"}}))
        .expect("l'appel rend une réponse");
    let rendu = &r["result"];
    assert_eq!(rendu["executed"], false, "la citation n'est pas liée : {r}");
    assert!(rendu["why"].as_str().unwrap_or("").contains("pas de fiche"), "la fiche est nommée : {r}");
    assert_eq!(rendu["ref"]["ecrite_seule"], true, "{r}");
    assert_eq!(compter(&backend, "CITE"), 0);

    let r = backend
        .call("add_ref", json!({"genre": "fichier", "valeur": "src/a.rs", "fiche": {"titre": "duty cycle"}}))
        .expect("avec la bonne fiche");
    assert_eq!(r["result"]["executed"], true, "{r}");
    assert_eq!(compter(&backend, "CITE"), 1, "le lien est posé, la référence n'est pas doublée");
}

/// Passer la fiche « à revoir » par sa transition déclarée.
fn faire_revoir(backend: &Backend) {
    let fiche = lire(backend, "get_fiche", json!({"titre": "duty cycle"}));
    backend
        .call(
            "put_fiche",
            json!({"record": {"titre": "duty cycle", "corps": "70 %", "etat": "a_revoir"},
                   "expected_revision": fiche["revision"]}),
        )
        .expect("la transition déclarée");
}

/// **La garde d'états déclarée** (marche 2) : un lien ne se pose que depuis un
/// bout dans les états que la relation permet — et **seuls les outils du
/// backend la voient** : un appel nu au catalogue passe outre, c'est voulu et
/// c'est épinglé ici (relecture de rag3db-97).
#[test]
#[ignore]
fn un_bout_hors_des_etats_permis_refuse_le_lien_et_le_catalogue_nu_passe_outre() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    ecrire_les_trois(&backend);
    faire_revoir(&backend);

    let refus = backend
        .call("ancrer", json!({"links": [{"from": {"titre": "duty cycle"}, "to": {"titre": "régime GPU"}}]}))
        .expect_err("une fiche « à revoir » ne s'ancre pas");
    assert!(
        refus.contains("a_revoir") && refus.contains("from_states: [courant]") && refus.contains("no links"),
        "le refus nomme l'état du bout et les états permis : {refus}"
    );
    assert_eq!(compter(&backend, "ANCRE"), 0);

    // Le prix, épinglé : le verbe brut du catalogue ne voit pas la politique.
    {
        let f = uuid(&backend, "Fiche", "titre", "duty cycle");
        let s = uuid(&backend, "Sujet", "titre", "régime GPU");
        backend.catalog.lock().unwrap().link("ANCRE", f, s, BTreeMap::new()).expect("le verbe brut");
    }
    assert_eq!(compter(&backend, "ANCRE"), 1, "Catalog::link passe outre la garde du backend — voulu");
}

#[test]
#[ignore]
fn add_ref_depuis_une_fiche_hors_etat_ne_lie_pas() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    ecrire_les_trois(&backend);
    backend
        .call("create_ref_type", json!({"name": "fichier", "description": "un chemin", "forme": "relatif", "confirm": true}))
        .expect("le genre");
    faire_revoir(&backend);
    let r = backend
        .call("add_ref", json!({"genre": "fichier", "valeur": "src/a.rs", "fiche": {"titre": "duty cycle"}}))
        .expect("l'appel rend une réponse");
    assert_eq!(r["result"]["executed"], false, "{r}");
    assert!(r["result"]["why"].as_str().unwrap_or("").contains("a_revoir"), "l'état est nommé : {r}");
    assert_eq!(compter(&backend, "CITE"), 0);
}

/// Au chargement : une garde sur un bout sans cycle de vie, ou qui nomme un
/// état inconnu, se refuse en le nommant. Et une garde **se recharge** : ce
/// n'est pas une migration du schéma.
#[test]
#[ignore]
fn la_garde_d_etats_se_verifie_au_chargement_et_se_recharge() {
    let dir = tempfile::tempdir().expect("dossier");
    let chemin = manifeste(dir.path());
    let original: Value = serde_json::from_slice(&std::fs::read(&chemin).unwrap()).unwrap();
    let ecrire = |m: &Value| std::fs::write(&chemin, serde_json::to_vec_pretty(m).unwrap()).unwrap();

    let mut sans_cycle = original.clone();
    sans_cycle["relations"]["ANCRE"]["to_states"] = json!(["courant"]);
    ecrire(&sans_cycle);
    let e = PreparedBackend::load(&chemin).err().expect("Sujet n'a pas de cycle de vie");
    assert!(e.contains("to_states") && e.contains("Sujet") && e.contains("cycle de vie"), "{e}");

    let mut inconnu = original.clone();
    inconnu["relations"]["ANCRE"]["from_states"] = json!(["perdu"]);
    ecrire(&inconnu);
    let e = PreparedBackend::load(&chemin).err().expect("état inconnu");
    assert!(e.contains("perdu") && e.contains("a_revoir") && e.contains("courant"), "{e}");

    ecrire(&original);
    let backend = ouvrir(&chemin);
    let mut elargi = original.clone();
    elargi["relations"]["ANCRE"]["from_states"] = json!(["courant", "a_revoir"]);
    ecrire(&elargi);
    backend.reload().expect("une garde d'états se recharge, ce n'est pas une migration");
    ecrire_les_trois(&backend);
    faire_revoir(&backend);
    backend
        .call("ancrer", json!({"links": [{"from": {"titre": "duty cycle"}, "to": {"titre": "régime GPU"}}]}))
        .expect("la garde rechargée permet maintenant « à revoir »");
    assert_eq!(compter(&backend, "ANCRE"), 1);
}

// ─── Le rapport dit la vérité sur `atomic` (11 octobre 2026) ────────────────

fn deux_sujets(backend: &Backend) {
    ecrire_les_trois(backend);
    backend
        .call("put_sujet", json!({"record": {"titre": "cartes", "corps": "les GPU du poste"}}))
        .expect("le second sujet");
}

/// Un lot qui part en **une seule instruction** est atomique par le moteur, et
/// le rapport le dit — il disait `false` avant, ce qui était faux.
#[test]
#[ignore]
fn un_lot_en_une_instruction_se_dit_atomique() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    deux_sujets(&backend);
    let r = backend
        .call("noter", json!({"links": [
            {"from": {"titre": "duty cycle"}, "to": {"titre": "régime GPU"}, "props": {"poids": "fort"}},
            {"from": {"titre": "duty cycle"}, "to": {"titre": "cartes"}, "props": {"poids": "faible"}}
        ]}))
        .expect("deux liens, mêmes clés de propriétés : une instruction");
    assert_eq!(r["result"]["atomic"], true, "{r}");
}

/// **Le cas a** : un lot qui mêle des liens avec et sans propriétés part en deux
/// instructions (le drain regroupe par ensemble de propriétés). La seconde
/// échoue — faute injectée —, et le lot doit être **tout ou rien**. Rouge avant
/// l'enveloppe : le premier lien restait écrit.
#[test]
#[ignore]
fn un_lot_en_deux_instructions_est_tout_ou_rien() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    deux_sujets(&backend);
    let mixte = json!({"links": [
        {"from": {"titre": "duty cycle"}, "to": {"titre": "régime GPU"}, "props": {"poids": "fort"}},
        {"from": {"titre": "duty cycle"}, "to": {"titre": "cartes"}}
    ]});

    // Armé, et seulement le temps de cet appel : la relation NOTE, son
    // second groupe. Aucun autre témoin de ce fichier ne pose deux groupes NOTE.
    std::env::set_var("RAG3WEAVER_TEST_LINK_FAULT_RELATION", "NOTE");
    std::env::set_var("RAG3WEAVER_TEST_LINK_FAULT_GROUP", "2");
    let rendu = backend.call("noter", mixte.clone());
    std::env::remove_var("RAG3WEAVER_TEST_LINK_FAULT_RELATION");
    std::env::remove_var("RAG3WEAVER_TEST_LINK_FAULT_GROUP");
    assert!(rendu.is_err(), "une instruction a échoué : l'appel le dit — {rendu:?}");
    assert_eq!(
        compter(&backend, "NOTE"),
        0,
        "tout ou rien : la première instruction est défaite avec la seconde — {rendu:?}"
    );

    // Sans faute, le même lot passe entier, et se dit atomique (enveloppé).
    let r = backend.call("noter", mixte).expect("sans faute");
    assert_eq!(compter(&backend, "NOTE"), 2);
    assert_eq!(r["result"]["atomic"], true, "{r}");
}

/// **Le cas b** : la fermeture emporte une création en file. La ligne et le lien
/// partent ensemble, mais le plein texte et les vecteurs vivent hors de la
/// transaction de la base : le lot n'est pas atomique, et le rapport le dit
/// avec sa raison.
#[test]
#[ignore]
fn une_fermeture_avec_une_creation_se_dit_non_atomique() {
    use rag3weaver::disponibilite::Disponibilites;
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    ecrire_les_trois(&backend);
    {
        let mut cat = backend.catalog.lock().unwrap();
        let mut nouveau = BTreeMap::new();
        nouveau.insert("titre".to_string(), CypherValue::String("en file".into()));
        nouveau.insert("corps".to_string(), CypherValue::String("pas encore posé".into()));
        cat.create_jusqu_a("Sujet", nouveau, Disponibilites::AUCUNE).expect("mis en file");
    }
    let r = backend
        .call("ancrer", json!({"links": [{"from": {"titre": "duty cycle"}, "to": {"titre": "en file"}}]}))
        .expect("le lien et sa création");
    assert_eq!(r["result"]["atomic"], false, "{r}");
    assert!(
        r["result"]["atomic_reason"].as_str().unwrap_or("").contains("plein texte"),
        "la raison est dite : {r}"
    );
}

/// **Un lien vers un bout absent se refuse, en le nommant** — retourné le
/// 11 octobre 2026 (marche 1 bis du lien générique).
///
/// Ce test a d'abord épinglé le défaut : `Catalog::link` mettait le lien en file
/// et le drainait, la relation se posait par `MATCH … MERGE`, et un bout absent
/// n'y était pas une erreur — le verbe rendait `Ok` pour un lien qui n'existait
/// pas, et `AddRefNode` l'annonçait comme posé. Le verbe vérifie maintenant ses
/// bouts quand le lien se pose (exigence non vide) : ni en base ni en file, c'est
/// un refus qui nomme la relation et le bout, et rien n'est écrit.
#[test]
#[ignore]
fn un_lien_vers_un_bout_absent_se_refuse_en_le_nommant() {
    let dir = tempfile::tempdir().expect("dossier");
    let backend = ouvrir(&manifeste(dir.path()));
    backend
        .call("put_fiche", json!({"record": {"titre": "seule", "corps": "x", "etat": "courant"}}))
        .expect("la fiche");
    let f = uuid(&backend, "Fiche", "titre", "seule");
    let absent = uuid(&backend, "Sujet", "titre", "jamais écrit");
    let rendu = {
        let mut cat = backend.catalog.lock().unwrap();
        cat.link("ANCRE", f, absent.clone(), BTreeMap::new())
    };
    let refus = rendu.err().map(|e| e.to_string()).expect("le verbe refuse un bout absent");
    assert!(
        refus.contains("« ANCRE »") && refus.contains("Sujet") && refus.contains(&absent),
        "le refus nomme la relation et le bout : {refus}"
    );
    assert_eq!(compter(&backend, "ANCRE"), 0, "et rien n'est écrit en base");
}

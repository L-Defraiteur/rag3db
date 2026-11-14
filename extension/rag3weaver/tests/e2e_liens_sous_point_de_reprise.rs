//! E2E : **reverser des liens déjà posés pendant qu'un point de reprise passe ne les double pas**
//! (14 novembre 2026, ticket
//! `docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-14-des-morceaux-de-scope-portent-plusieurs-aretes-vers-le-meme-scope.md`).
//!
//! Dans la base de dépôt de Lucie, 1 291 morceaux de Scope portaient 2 à 7 arêtes `Scope_CHUNKED_FROM`
//! vers le même Scope : une de plus à chaque réindexation du fichier. Au-delà de 200 arêtes, les liens
//! partent par COPY (`copier_les_liens`), qui saute les paires déjà en base en les lisant par
//! `UNWIND $froms MATCH (a {_uuid: f})-[r]->(b)`, une recherche par clé primaire. Une telle recherche
//! ratait des clés validées pendant un point de reprise V2 (ticket du cœur
//! `docs/coeur-cpp/tickets/2026-11-13-une-recherche-par-cle-primaire-rate-des-cles-validees-pendant-un-point-de-reprise.md`,
//! corrigé dans la lib 0c41f9fe6). La paire ratée paraissait absente, et le COPY la reposait.
//!
//! Le témoin : 600 paires posées une fois et un point de reprise ; puis, pendant qu'une seconde
//! connexion insère des nœuds de la même table et force des points de reprise, les mêmes 600 paires
//! sont reversées quarante fois, en écriture automatique (la lecture des paires déjà là n'est pas dans
//! une transaction d'écriture, elle peut donc croiser le point de reprise). Après chaque reversement,
//! aucune paire ne doit porter plus d'une arête.
//!
//! Attendu : rouge sous une lib d'avant 0c41f9fe6 (e93710f36), vert sous 0c41f9fe6.
//!
//! Run with: ./run_e2e.sh --test e2e_liens_sous_point_de_reprise
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::records::RefOrUuid;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

const PAIRES: usize = 600;
const TOURS: usize = 40;

fn bout() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "nom".to_string(),
        SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: true, ..Default::default() },
    );
    EntityConfig { fields, signals: SearchSignals::BM25, hashsafe: Some(vec!["nom".into()]), ..Default::default() }
}

fn ligne(nom: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("nom".to_string(), CypherValue::String(nom.into()))])
}

fn entier(catalog: &Catalog, requete: &str) -> i64 {
    catalog.execute_raw(requete).unwrap().rows[0][0].as_i64().unwrap_or(-1)
}

#[test]
#[ignore]
fn des_liens_reverses_pendant_un_point_de_reprise_ne_se_doublent_pas() {
    let d = tempfile::tempdir().unwrap();
    let conn = Rag3dbConnection::new(d.path().join("base.rag3db")).expect("base sur disque");
    let autre = conn.create_sync_connection().expect("seconde connexion");
    let config = CatalogConfig { name: Some("liens-sous-point-de-reprise".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.execute_raw("CALL auto_checkpoint=false").unwrap();
    catalog.register_entity("Gauche", bout()).unwrap();
    catalog.register_entity("Droite", bout()).unwrap();
    catalog.register_relation_with("Lien", "Gauche", "Droite", HashMap::new()).unwrap();

    let gauches: Vec<String> = (0..PAIRES).map(|i| format!("g{i}")).collect();
    let droites: Vec<String> = (0..PAIRES).map(|i| format!("d{i}")).collect();
    catalog.ingest_entities("Gauche", gauches.iter().map(|n| ligne(n)).collect()).unwrap();
    catalog.ingest_entities("Droite", droites.iter().map(|n| ligne(n)).collect()).unwrap();
    let paires: Vec<(String, String)> = gauches
        .iter()
        .zip(&droites)
        .map(|(g, d)| (catalog.entity_uuid("Gauche", &ligne(g)).unwrap(), catalog.entity_uuid("Droite", &ligne(d)).unwrap()))
        .collect();
    let verser = |catalog: &mut Catalog| {
        for (g, d) in &paires {
            catalog
                .link_jusqu_a("Lien", RefOrUuid::Uuid(g.clone()), RefOrUuid::Uuid(d.clone()), BTreeMap::new(), Disponibilites::AUCUNE)
                .unwrap();
        }
        let res = catalog.drain();
        assert_eq!(res.failed, 0, "{:?}", res.warnings);
    };
    verser(&mut catalog);
    catalog.execute_raw("CHECKPOINT").unwrap();
    assert_eq!(entier(&catalog, "MATCH ()-[r:Lien]->() RETURN count(r)"), PAIRES as i64, "le premier versement");

    // La seconde connexion : des Gauche neufs, par lots, et un point de reprise tous les trois lots.
    let arret = Arc::new(AtomicBool::new(false));
    let points = Arc::new(AtomicUsize::new(0));
    let fond = {
        let (arret, points, autre) = (arret.clone(), points.clone(), autre.clone());
        std::thread::spawn(move || {
            let mut lot = 0usize;
            while !arret.load(Ordering::Relaxed) {
                let debut = 1_000_000 + lot * 200;
                let _ = autre.execute(&format!(
                    "UNWIND range({debut}, {}) AS i CREATE (:Gauche {{_uuid: 'bruit-' + CAST(i AS STRING), nom: 'bruit'}})",
                    debut + 199
                ));
                if lot % 3 == 2 && autre.execute("CHECKPOINT").is_ok() {
                    points.fetch_add(1, Ordering::Relaxed);
                }
                lot += 1;
            }
        })
    };

    let mut premier_rouge: Option<(usize, i64, i64)> = None;
    for tour in 0..TOURS {
        verser(&mut catalog);
        let doubles = entier(
            &catalog,
            "MATCH (a:Gauche)-[r:Lien]->(b:Droite) WITH a, b, count(r) AS n WHERE n > 1 RETURN count(*)",
        );
        if doubles != 0 && premier_rouge.is_none() {
            premier_rouge = Some((tour, doubles, entier(&catalog, "MATCH ()-[r:Lien]->() RETURN count(r)")));
        }
    }
    arret.store(true, Ordering::Relaxed);
    fond.join().unwrap();
    let total = entier(&catalog, "MATCH ()-[r:Lien]->() RETURN count(r)");
    println!(
        "[témoin] {TOURS} reversements de {PAIRES} paires, {} points de reprise de la seconde connexion ; arêtes à la fin : {total}",
        points.load(Ordering::Relaxed)
    );
    assert!(points.load(Ordering::Relaxed) > 0, "[check: un-point-de-reprise-a-croise] aucun point de reprise n'a tourné");
    if let Some((tour, doubles, arretes)) = premier_rouge {
        panic!("[check: liens-sans-doublon] au reversement {tour}, {doubles} paires portaient plus d'une arête ({arretes} arêtes pour {PAIRES} paires)");
    }
    assert_eq!(total, PAIRES as i64, "[check: liens-sans-doublon] {total} arêtes pour {PAIRES} paires");
}

//! **Un bout prouvé ne se vérifie plus, et une paire sans bout se compte
//! toujours** (5 octobre 2026, levier 2 du chargement final).
//!
//! Le COPY des liens demandait au moteur, par tranches, si chaque bout
//! existait : COPY refuse tout le fichier dès qu'une clé manque, là où MERGE
//! sautait la paire. Pendant une synchronisation sous la transaction par
//! paquet, les lignes qu'un `InsertRecordNode` a vraiment posées font preuve
//! (`Catalog::begin_proving_presence`) : elles ne se demandent plus. Une
//! suppression retire ses uuids de la preuve.
//!
//! Le cas fabriqué, dans un processus fils (la ligne `[link-profile]` est
//! sur sa sortie d'erreur), une fois avec la preuve et une fois sans :
//! - 30 `Gauche` et 30 `Droite` posés pendant la preuve ;
//! - 10 `Droite` posés avant elle (non prouvés, vérifiés comme avant) ;
//! - 5 des 30 `Droite` prouvés supprimés avant les liens ;
//! - 10 uuids de `Droite` jamais posés.
//!
//! Chaque `Gauche` se lie aux 50 cibles : 30 × 35 = 1 050 liens posés, 30 × 15
//! = 450 paires sans bout comptées. Le COPY n'est jamais refusé. Les deux
//! passes rendent les mêmes comptes, et seule la première a des bouts prouvés.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_preuve_d_existence
//! ```
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::records::RefOrUuid;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

const ENFANT: &str = "PREUVE_D_EXISTENCE_ENFANT";

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

/// Le rôle du fils : le scénario, avec ou sans la preuve, puis le compte.
fn jouer(avec_preuve: bool) {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig { name: Some("preuve".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Gauche", bout()).unwrap();
    catalog.register_entity("Droite", bout()).unwrap();
    catalog.register_relation_with("Lien", "Gauche", "Droite", HashMap::new()).unwrap();

    // Avant la preuve : ces lignes existent sans être prouvées.
    let anciennes: Vec<String> = (0..10).map(|i| format!("ancienne{i}")).collect();
    catalog.ingest_entities("Droite", anciennes.iter().map(|n| ligne(n)).collect()).unwrap();

    if avec_preuve {
        catalog.begin_proving_presence();
    }
    let gauches: Vec<String> = (0..30).map(|i| format!("g{i}")).collect();
    let droites: Vec<String> = (0..30).map(|i| format!("d{i}")).collect();
    catalog.ingest_entities("Gauche", gauches.iter().map(|n| ligne(n)).collect()).unwrap();
    catalog.ingest_entities("Droite", droites.iter().map(|n| ligne(n)).collect()).unwrap();
    // Cinq lignes prouvées, puis supprimées : la preuve doit les oublier.
    for d in &droites[..5] {
        let u = catalog.entity_uuid("Droite", &ligne(d)).unwrap();
        catalog.delete_jusqu_a("Droite", &u, Disponibilites::AUCUNE).unwrap();
    }
    let vidange = catalog.drain();
    assert_eq!(vidange.failed, 0, "{:?}", vidange.warnings);

    let mut cibles: Vec<String> = droites.iter().chain(anciennes.iter()).map(|d| catalog.entity_uuid("Droite", &ligne(d)).unwrap()).collect();
    cibles.extend((0..10).map(|i| format!("jamais-posee-{i}")));
    for g in &gauches {
        let ug = catalog.entity_uuid("Gauche", &ligne(g)).unwrap();
        for ud in &cibles {
            catalog
                .link_jusqu_a("Lien", RefOrUuid::Uuid(ug.clone()), RefOrUuid::Uuid(ud.clone()), BTreeMap::new(), Disponibilites::AUCUNE)
                .unwrap();
        }
    }
    let res = catalog.drain();
    catalog.end_proving_presence();
    assert_eq!(res.failed, 0, "{:?}", res.warnings);
    assert!(
        !res.warnings.iter().any(|w| w.contains("refusé") || w.contains("retour au chemin par lots")),
        "le COPY des liens ne doit pas être refusé : {:?}",
        res.warnings
    );
    let n = catalog.execute_raw("MATCH ()-[r:Lien]->() RETURN count(r)").unwrap().rows;
    println!("LIENS={:?}", n[0][0]);
}

/// Les nombres d'une ligne `[link-profile] Lien : …`.
fn profil(sortie: &str) -> (u64, u64, u64) {
    let ligne = sortie
        .lines()
        .find(|l| l.starts_with("[link-profile] Lien :"))
        .unwrap_or_else(|| panic!("pas de ligne de profil des liens — le COPY n'a pas servi :\n{sortie}"));
    let nombre_avant = |mot: &str| -> u64 {
        let i = ligne.find(mot).unwrap_or_else(|| panic!("« {mot} » absent de « {ligne} »"));
        ligne[..i].split_whitespace().last().and_then(|n| n.parse().ok()).unwrap_or_else(|| panic!("nombre illisible avant « {mot} » : {ligne}"))
    };
    (nombre_avant(" arêtes"), nombre_avant(" sans bout"), nombre_avant(" bouts prouvés"))
}

fn lancer(avec_preuve: bool) -> String {
    let sortie = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "un_bout_prouve_ne_se_verifie_plus_et_une_paire_sans_bout_se_compte", "--nocapture", "--ignored"])
        .env(ENFANT, if avec_preuve { "avec" } else { "sans" })
        .env("RAG3WEAVER_INGEST_PROFILE", "1")
        .output()
        .expect("lancer le fils");
    let texte = format!("{}{}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
    assert!(sortie.status.success(), "le fils a échoué ({:?}) :\n{texte}", sortie.status);
    texte
}

#[test]
#[ignore]
fn un_bout_prouve_ne_se_verifie_plus_et_une_paire_sans_bout_se_compte() {
    if let Ok(cas) = std::env::var(ENFANT) {
        jouer(cas == "avec");
        return;
    }
    let avec = lancer(true);
    let sans = lancer(false);
    let (ecrites, absents, prouves) = profil(&avec);
    let (ecrites_sans, absents_sans, prouves_sans) = profil(&sans);
    println!("▸ avec la preuve : {ecrites} posés, {absents} sans bout, {prouves} bouts prouvés");
    println!("▸ sans la preuve : {ecrites_sans} posés, {absents_sans} sans bout, {prouves_sans} bouts prouvés");
    assert!(avec.contains("LIENS=Int(1050)"), "1 050 liens en base avec la preuve");
    assert!(sans.contains("LIENS=Int(1050)"), "1 050 liens en base sans la preuve");
    assert_eq!((ecrites, absents), (1050, 450), "avec la preuve : chaque paire sans bout comptée, aucune posée");
    assert_eq!((ecrites_sans, absents_sans), (1050, 450), "sans la preuve : les mêmes comptes");
    // 30 départs et 25 arrivées prouvés ; ni les supprimées, ni les
    // anciennes, ni les jamais posées.
    assert_eq!(prouves, 55, "les bouts prouvés : 30 Gauche et les 25 Droite restées");
    assert_eq!(prouves_sans, 0, "sans la preuve, rien n'est supposé");
}

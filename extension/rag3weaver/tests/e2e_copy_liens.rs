//! E2E : **le chargement en masse des liens nomme ses colonnes** (3 octobre
//! 2026).
//!
//! Au-delà d'un seuil, les liens d'une relation partent par `COPY` : le CSV
//! écrit les propriétés dans l'ordre de leurs noms triés. Sans liste de
//! colonnes, le moteur les remplissait dans l'ordre de la table — celui des
//! `ALTER` successifs —, et deux colonnes texte s'échangeaient **sans
//! erreur** (relevé par la session codeparsers). Ici la table est créée à
//! l'envers de l'ordre trié (`zeta` puis `alpha`) : sans les colonnes
//! nommées, l'échange est certain.
//!
//! Run with: ./run_e2e.sh --test e2e_copy_liens
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::{FieldDef, FieldType};
use rag3weaver::connection::CypherValue;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::records::RefOrUuid;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

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

#[test]
#[ignore]
fn deux_proprietes_texte_ne_s_echangent_pas_au_chargement_en_masse() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig { name: Some("copy-liens".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Gauche", bout()).unwrap();
    catalog.register_entity("Droite", bout()).unwrap();
    // La table d'abord, à l'envers de l'ordre trié des noms.
    catalog.execute_raw("CREATE REL TABLE IF NOT EXISTS Lien(FROM Gauche TO Droite, zeta STRING, alpha STRING)").unwrap();
    let texte = || -> FieldDef { serde_json::from_value(serde_json::json!({"type": "string"})).unwrap() };
    let proprietes = HashMap::from([("zeta".to_string(), texte()), ("alpha".to_string(), texte())]);
    catalog.register_relation_with("Lien", "Gauche", "Droite", proprietes).unwrap();

    // 45 × 45 = 2 025 liens distincts : au-delà du seuil, ils partent par COPY.
    let gauches: Vec<String> = (0..45).map(|i| format!("g{i}")).collect();
    let droites: Vec<String> = (0..45).map(|i| format!("d{i}")).collect();
    catalog.ingest_entities("Gauche", gauches.iter().map(|n| ligne(n)).collect()).unwrap();
    catalog.ingest_entities("Droite", droites.iter().map(|n| ligne(n)).collect()).unwrap();
    for g in &gauches {
        let ug = catalog.entity_uuid("Gauche", &ligne(g)).unwrap();
        for d in &droites {
            let ud = catalog.entity_uuid("Droite", &ligne(d)).unwrap();
            let props = BTreeMap::from([
                ("alpha".to_string(), CypherValue::String("A".into())),
                ("zeta".to_string(), CypherValue::String("Z".into())),
            ]);
            catalog
                .link_jusqu_a("Lien", RefOrUuid::Uuid(ug.clone()), RefOrUuid::Uuid(ud), props, Disponibilites::AUCUNE)
                .unwrap();
        }
    }
    let res = catalog.drain();
    assert_eq!(res.failed, 0, "{:?}", res.warnings);
    let lu = catalog.execute_raw("MATCH ()-[r:Lien]->() RETURN r.alpha, r.zeta, count(*)").unwrap().rows;
    assert_eq!(
        lu,
        vec![vec![CypherValue::String("A".into()), CypherValue::String("Z".into()), CypherValue::Int(2025)]],
        "chaque propriété dans sa colonne"
    );
}

/// **Une propriété texte qui porte une virgule, un guillemet ou un saut de
/// ligne passe par le COPY** (3 octobre 2026). Sans les options de lecture du
/// COPY des nœuds, le renifleur du moteur décidait seul qu'il n'y avait pas
/// de guillemets : `"other,type"` comptait pour deux colonnes, le COPY était
/// refusé, et tous les liens repartaient par le chemin par lots — 56 s et
/// 145 s sur le dépôt entier, mesurés par la session embarquements.
#[test]
#[ignore]
fn une_propriete_a_virgule_ne_fait_pas_refuser_le_copy() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig { name: Some("copy-virgule".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Gauche", bout()).unwrap();
    catalog.register_entity("Droite", bout()).unwrap();
    let texte = || -> FieldDef { serde_json::from_value(serde_json::json!({"type": "string"})).unwrap() };
    catalog.register_relation_with("Lien", "Gauche", "Droite", HashMap::from([("usages".to_string(), texte())])).unwrap();

    let gauches: Vec<String> = (0..30).map(|i| format!("g{i}")).collect();
    let droites: Vec<String> = (0..30).map(|i| format!("d{i}")).collect();
    catalog.ingest_entities("Gauche", gauches.iter().map(|n| ligne(n)).collect()).unwrap();
    catalog.ingest_entities("Droite", droites.iter().map(|n| ligne(n)).collect()).unwrap();
    // Les valeurs piégées **après** 800 lignes sans guillemet : le renifleur
    // ne lit que le début du fichier (sur le dépôt entier, le premier
    // guillemet tombait à la ligne 727).
    let pieges = ["other,type", "dit \"oui\"", "deux\nlignes"];
    let valeur = |k: usize| if k < 800 { "simple" } else { pieges[k % pieges.len()] };
    let mut attendu: BTreeMap<String, i64> = BTreeMap::new();
    for (i, g) in gauches.iter().enumerate() {
        let ug = catalog.entity_uuid("Gauche", &ligne(g)).unwrap();
        for (j, d) in droites.iter().enumerate() {
            let ud = catalog.entity_uuid("Droite", &ligne(d)).unwrap();
            let v = valeur(i * droites.len() + j);
            *attendu.entry(v.to_string()).or_default() += 1;
            let props = BTreeMap::from([("usages".to_string(), CypherValue::String(v.into()))]);
            catalog.link_jusqu_a("Lien", RefOrUuid::Uuid(ug.clone()), RefOrUuid::Uuid(ud), props, Disponibilites::AUCUNE).unwrap();
        }
    }
    let res = catalog.drain();
    assert_eq!(res.failed, 0, "{:?}", res.warnings);
    assert!(!res.warnings.iter().any(|w| w.contains("refusé")), "le COPY ne doit pas être refusé : {:?}", res.warnings);
    let mut lu = catalog.execute_raw("MATCH ()-[r:Lien]->() RETURN r.usages, count(*)").unwrap().rows;
    lu.sort_by_key(|r| format!("{r:?}"));
    let mut attendu: Vec<Vec<CypherValue>> =
        attendu.into_iter().map(|(v, n)| vec![CypherValue::String(v), CypherValue::Int(n)]).collect();
    attendu.sort_by_key(|r| format!("{r:?}"));
    assert_eq!(lu, attendu, "chaque valeur intacte, 900 liens");
}

/// **Aucune donnée n'est lue NULL par le COPY** (4 octobre 2026). Le mot que
/// le COPY lit comme NULL était une constante du code, `__rag3weaver_null__` ;
/// indexer `dialect.rs` en faisait un symbole, et ce symbole devenait NULL au
/// chargement en masse. Le mot est maintenant tiré au démarrage du processus.
/// Ici, l'ancien mot circule comme une valeur ordinaire, par le COPY des
/// nœuds et par celui des liens, et revient intact.
#[test]
#[ignore]
fn l_ancien_mot_du_null_revient_intact_du_copy() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig { name: Some("copy-null".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Gauche", bout()).unwrap();
    catalog.register_entity("Droite", bout()).unwrap();
    let texte = || -> FieldDef { serde_json::from_value(serde_json::json!({"type": "string"})).unwrap() };
    catalog.register_relation_with("Lien", "Gauche", "Droite", HashMap::from([("p".to_string(), texte())])).unwrap();
    let mot = "__rag3weaver_null__";
    // Au-delà des seuils : les nœuds (table vide) et les liens partent par COPY.
    let gauches: Vec<String> = (0..30).map(|i| if i == 0 { mot.to_string() } else { format!("g{i}") }).collect();
    let droites: Vec<String> = (0..30).map(|i| format!("d{i}")).collect();
    catalog.ingest_entities("Gauche", gauches.iter().map(|n| ligne(n)).collect()).unwrap();
    catalog.ingest_entities("Droite", droites.iter().map(|n| ligne(n)).collect()).unwrap();
    for g in &gauches {
        let ug = catalog.entity_uuid("Gauche", &ligne(g)).unwrap();
        for d in &droites {
            let ud = catalog.entity_uuid("Droite", &ligne(d)).unwrap();
            let props = BTreeMap::from([("p".to_string(), CypherValue::String(mot.into()))]);
            catalog.link_jusqu_a("Lien", RefOrUuid::Uuid(ug.clone()), RefOrUuid::Uuid(ud), props, Disponibilites::AUCUNE).unwrap();
        }
    }
    let res = catalog.drain();
    assert_eq!(res.failed, 0, "{:?}", res.warnings);
    assert!(catalog.take_bulk_load_refusals().is_empty());
    let noeud = catalog.execute_raw(&format!("MATCH (g:Gauche) WHERE g.nom = '{mot}' RETURN count(g)")).unwrap().rows;
    assert_eq!(noeud, vec![vec![CypherValue::Int(1)]], "le nœud garde son nom");
    let liens = catalog.execute_raw(&format!("MATCH ()-[r:Lien]->() WHERE r.p = '{mot}' RETURN count(r)")).unwrap().rows;
    assert_eq!(liens, vec![vec![CypherValue::Int(900)]], "chaque lien garde sa propriété");
}

/// **Une paire répétée dans un lot garde la dernière valeur** (4 octobre
/// 2026). Sous le seuil du COPY, les liens partent par `MERGE` ; le moteur
/// perdait le `SET` de la seconde occurrence d'une paire quand une autre
/// relation était créée entre les deux (recette du banc :
/// `[a→b x1, a→c w5, a→b y2]` laissait `a→b` à `x1`). Le lot est maintenant
/// dédoublonné avant l'envoi, en gardant la dernière occurrence.
#[test]
#[ignore]
fn une_paire_repetee_dans_un_lot_garde_la_derniere_valeur() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig { name: Some("lien-repete".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Gauche", bout()).unwrap();
    catalog.register_entity("Droite", bout()).unwrap();
    let texte = || -> FieldDef { serde_json::from_value(serde_json::json!({"type": "string"})).unwrap() };
    catalog.register_relation_with("Lien", "Gauche", "Droite", HashMap::from([("p".to_string(), texte())])).unwrap();
    catalog.ingest_entities("Gauche", vec![ligne("a")]).unwrap();
    catalog.ingest_entities("Droite", vec![ligne("b"), ligne("c")]).unwrap();
    let ua = catalog.entity_uuid("Gauche", &ligne("a")).unwrap();
    let ub = catalog.entity_uuid("Droite", &ligne("b")).unwrap();
    let uc = catalog.entity_uuid("Droite", &ligne("c")).unwrap();
    for (vers, p) in [(&ub, "x1"), (&uc, "w5"), (&ub, "y2")] {
        let props = BTreeMap::from([("p".to_string(), CypherValue::String(p.into()))]);
        catalog
            .link_jusqu_a("Lien", RefOrUuid::Uuid(ua.clone()), RefOrUuid::Uuid(vers.clone()), props, Disponibilites::AUCUNE)
            .unwrap();
    }
    let res = catalog.drain();
    assert_eq!(res.failed, 0, "{:?}", res.warnings);
    let mut lu = catalog.execute_raw("MATCH (a:Gauche)-[r:Lien]->(b:Droite) RETURN b.nom, r.p").unwrap().rows;
    lu.sort_by_key(|r| format!("{r:?}"));
    assert_eq!(
        lu,
        vec![
            vec![CypherValue::String("b".into()), CypherValue::String("y2".into())],
            vec![CypherValue::String("c".into()), CypherValue::String("w5".into())],
        ],
        "a→b porte la dernière valeur, une seule fois"
    );
}

/// **Un NULL en tête d'un lot ne fait pas tomber le lot.** Un NULL dans la
/// liste de paramètres n'a pas de type ; le moteur le lisait en STRING et
/// toute la colonne prenait ce type, qu'une colonne entière refusait
/// (`STRUCT_EXTRACT(item,line) has data type STRING but expected INT64`).
/// Le 4 octobre 2026, deux rendez-vous sans ligne en tête d'un lot de
/// `MENTIONS` faisaient échouer les cinq du lot, et seul un compteur que
/// personne ne lisait le disait. Le lot passe sous le seuil du COPY : c'est
/// le chemin par `UNWIND`.
#[test]
#[ignore]
fn un_null_en_tete_d_un_lot_ne_fait_pas_tomber_le_lot() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig { name: Some("lien-null".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Gauche", bout()).unwrap();
    catalog.register_entity("Droite", bout()).unwrap();
    let entier = || -> FieldDef { serde_json::from_value(serde_json::json!({"type": "int64"})).unwrap() };
    catalog.register_relation_with("Lien", "Gauche", "Droite", HashMap::from([("line".to_string(), entier())])).unwrap();
    catalog.ingest_entities("Gauche", vec![ligne("a")]).unwrap();
    catalog.ingest_entities("Droite", vec![ligne("b"), ligne("c"), ligne("d")]).unwrap();
    let ua = catalog.entity_uuid("Gauche", &ligne("a")).unwrap();
    for (vers, line) in [("b", CypherValue::Null), ("c", CypherValue::Int(7)), ("d", CypherValue::Null)] {
        let uv = catalog.entity_uuid("Droite", &ligne(vers)).unwrap();
        let props = BTreeMap::from([("line".to_string(), line)]);
        catalog.link_jusqu_a("Lien", RefOrUuid::Uuid(ua.clone()), RefOrUuid::Uuid(uv), props, Disponibilites::AUCUNE).unwrap();
    }
    let res = catalog.drain();
    assert_eq!(res.failed, 0, "{:?}", res.warnings);
    let mut lu = catalog.execute_raw("MATCH (a:Gauche)-[r:Lien]->(b:Droite) RETURN b.nom, r.line").unwrap().rows;
    lu.sort_by_key(|r| format!("{r:?}"));
    assert_eq!(
        lu,
        vec![
            vec![CypherValue::String("b".into()), CypherValue::Null],
            vec![CypherValue::String("c".into()), CypherValue::Int(7)],
            vec![CypherValue::String("d".into()), CypherValue::Null],
        ],
        "les trois liens posés, la ligne seulement où elle est connue"
    );
}

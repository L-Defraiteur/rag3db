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

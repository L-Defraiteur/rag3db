//! **Dans une transaction, une table repliée sur MERGE ne repasse pas par
//! COPY** (4 octobre 2026).
//!
//! Le moteur range les lignes insérées hors COPY d'une transaction sous des
//! décalages provisoires qui commencent au nombre de lignes de la table ; un
//! COPY de la même transaction écrit les siennes aux mêmes décalages, et une
//! recherche par clé prend alors l'une pour l'autre — plantage dans
//! `NodeGroup::lookup` (ticket
//! `2026-10-04-copy-apres-des-insertions-dans-la-meme-transaction`, session
//! cœur C++). La transaction par paquet le rencontrait quand un paquet se
//! repliait sur MERGE et qu'un paquet plus tardif du même groupe reprenait
//! le COPY dans la même table.
//!
//! La règle (`Catalog::merge_in_transaction`) garde la table en MERGE jusqu'à
//! la validation. Le témoin : dans une transaction, une première indexation
//! d'une entité, un premier lot dont une valeur ne s'écrit pas en CSV (repli
//! sur MERGE), puis un lot de 3 000 lignes, puis une recherche par clé dans
//! la transaction. Sans la règle, le second lot partait par COPY et la
//! recherche plantait. Joué dans un processus fils : un plantage y reste.
//!
//! ```bash
//! ./run_e2e.sh --test e2e_tx_merge_puis_copy
//! ```
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::disponibilite::Disponibilites;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

const ENFANT: &str = "TX_MERGE_PUIS_COPY_ENFANT";

fn doc() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "nom".to_string(),
        SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: true, ..Default::default() },
    );
    fields.insert("note".to_string(), SimpleFieldDef { field_type: FieldType::String, ..Default::default() });
    EntityConfig { fields, signals: SearchSignals::BM25, hashsafe: Some(vec!["nom".into()]), ..Default::default() }
}

fn ligne(nom: &str, note: CypherValue) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([("nom".to_string(), CypherValue::String(nom.into())), ("note".to_string(), note)])
}

/// Le rôle du fils : tout le scénario, puis « FINI » s'il n'a pas planté.
fn jouer() {
    let conn = Rag3dbConnection::in_memory().expect("base en mémoire");
    let config = CatalogConfig { name: Some("merge-puis-copy".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(Box::new(conn), Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Doc", doc()).unwrap();
    catalog.prepare_schema_for_ingest().unwrap();
    let retenues = catalog.begin_fresh_ingest(&["Doc"], &[]);
    assert_eq!(retenues, vec!["Doc".to_string()], "la table est vide : une première indexation");

    catalog.conn().execute("BEGIN TRANSACTION").unwrap();
    catalog.set_in_transaction(true);
    // Un NULL ne s'écrit pas en CSV : ce lot se replie sur MERGE.
    let premier = catalog
        .ingest_entities_jusqu_a("Doc", vec![ligne("d-0", CypherValue::Null)], Disponibilites::RECHERCHE_TEXTE)
        .unwrap();
    assert_eq!(premier.failed, 0, "{:?}", premier.warnings);
    let refus = catalog.take_bulk_load_refusals();
    assert!(!refus.is_empty(), "le premier lot s'est replié sur MERGE, et l'a dit");
    // Le second lot : 3 000 naissances, qui partiraient par COPY.
    let lot: Vec<_> = (1..=3000).map(|i| ligne(&format!("d-{i}"), CypherValue::String("x".into()))).collect();
    let second = catalog.ingest_entities_jusqu_a("Doc", lot, Disponibilites::RECHERCHE_TEXTE).unwrap();
    assert_eq!(second.failed, 0, "{:?}", second.warnings);
    // La recherche par clé, dans la transaction.
    let uuid = catalog.entity_uuid("Doc", &ligne("d-10", CypherValue::String("x".into()))).unwrap();
    let trouve = catalog
        .execute_raw(&format!("MATCH (d:Doc {{_uuid: '{uuid}'}}) RETURN d.nom"))
        .unwrap()
        .rows;
    assert_eq!(trouve, vec![vec![CypherValue::String("d-10".into())]], "la recherche par clé trouve la bonne ligne");
    catalog.set_in_transaction(false);
    catalog.conn().execute("COMMIT").unwrap();
    catalog.end_fresh_ingest();
    let n = catalog.execute_raw("MATCH (d:Doc) RETURN count(d)").unwrap().rows[0][0].clone();
    assert_eq!(n, CypherValue::Int(3001), "toutes les lignes, une fois");
    println!("FINI");
}

#[test]
#[ignore]
fn une_table_repliee_sur_merge_ne_repasse_pas_par_copy_dans_la_transaction() {
    if std::env::var_os(ENFANT).is_some() {
        jouer();
        return;
    }
    let sortie = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "une_table_repliee_sur_merge_ne_repasse_pas_par_copy_dans_la_transaction", "--nocapture", "--ignored"])
        .env(ENFANT, "1")
        .output()
        .expect("lancer le fils");
    let texte = format!("{}{}", String::from_utf8_lossy(&sortie.stdout), String::from_utf8_lossy(&sortie.stderr));
    assert!(sortie.status.success() && texte.contains("FINI"), "le fils a planté ou échoué ({:?}) :\n{texte}", sortie.status);
}

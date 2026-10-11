//! E2E : **un nom réservé du dialecte passe de bout en bout**, parce que le
//! dialecte cite tout nom du schéma qu'il écrit.
//!
//! Ticket `docs/tickets/2026-10-11-une-entite-au-nom-reserve-casse-l-ouverture.md` :
//! une entité `Order` passait le chargement, puis l'ouverture envoyait
//! `CREATE NODE TABLE IF NOT EXISTS Order(` et le parseur du moteur refusait
//! (« mismatched input 'Order' »). Le refus posé la nuit du 11 octobre est
//! retiré : décision de Lucie, la voie de l'échappement systématique
//! (`docs/8-octobre-2026-16h29/embarquements/07-echappement-des-noms.md`).
//! Chaque dialecte cite par sa règle (`SchemaDialect::quote`) ; `Order` et
//! `order` s'enregistrent, s'écrivent et se relisent.
//!
//! Le seul nom encore refusé est celui qui porte lui-même le caractère de
//! citation (accent grave, guillemet double) : la validation des
//! identifiants l'arrête avant toute requête.
//!
//! Run with: ./run_e2e.sh --test e2e_nom_reserve

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::{EntityConfig, FieldType, SimpleFieldDef};
use rag3weaver::connection::{CypherValue, QueryParam};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};
use rag3weaver_ir::{Column, Predicate, Select};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn catalogue() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let ext = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    assert!(std::path::Path::new(&ext).exists(), "vector extension not found at {ext} — ./run_e2e.sh --build-only");
    boxed.execute(&format!("LOAD EXTENSION '{ext}'")).unwrap();
    let config = CatalogConfig { name: Some("nom-reserve-e2e".into()), embedding_dim: 64, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(64)), config);
    catalog.initialize().unwrap();
    catalog
}

fn entite(champ: &str) -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(champ.to_string(), SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: true, ..Default::default() });
    EntityConfig { fields, ..Default::default() }
}

fn ligne(champ: &str, valeur: &str) -> BTreeMap<String, CypherValue> {
    BTreeMap::from([(champ.to_string(), CypherValue::String(valeur.to_string()))])
}

/// L'entité `Order` et son champ `order` : enregistrés, écrits, puis relus
/// par une sélection du dialecte filtrée sur le champ.
#[test]
#[ignore]
fn une_entite_et_un_champ_au_nom_reserve_passent_de_bout_en_bout() {
    let mut cat = catalogue();
    cat.register_entity("Order", entite("order")).expect("`Order` s'enregistre : le dialecte cite son nom");
    let attendu = cat.entity_uuid("Order", &ligne("order", "commande 42")).unwrap();
    cat.ingest_entities("Order", vec![ligne("order", "commande 42"), ligne("order", "commande 43")]).expect("l'écriture passe");

    let select = Select::filtered("Order", Predicate::Equals { field: "order".into(), param: "v".into() }, vec![Column::Node("_uuid".into())]);
    let requete = cat.dialect_arc().select(&select).unwrap();
    let lignes = cat
        .execute_raw_with_params(&requete, &[QueryParam::new("v", CypherValue::String("commande 42".into()))])
        .unwrap_or_else(|e| panic!("la lecture passe : {e}\n{requete}"));
    assert_eq!(lignes.rows.len(), 1, "une seule ligne porte cette valeur : {:?}", lignes.rows);
    assert_eq!(lignes.rows[0][0].as_str(), Some(attendu.as_str()), "la ligne relue est celle écrite");
}

/// Un nom ordinaire passe, et un mot-clé que la grammaire accepte comme nom
/// (`Match`, non réservé) aussi.
#[test]
#[ignore]
fn un_nom_ordinaire_et_un_mot_cle_non_reserve_passent() {
    let mut cat = catalogue();
    cat.register_entity("Purchase", entite("title")).unwrap();
    cat.register_entity("Match", entite("title")).unwrap();
}

/// Le cas limite : un nom qui porte le caractère de citation d'un dialecte
/// ne peut pas être cité sans échappement interne. Il est refusé par la
/// validation des identifiants, en le nommant, avant toute requête.
#[test]
#[ignore]
fn un_nom_qui_porte_le_caractere_de_citation_est_refuse() {
    for nom in ["Ord`er", "Ord\"er", "order`", "\"order\""] {
        for genre in ["entity", "field", "relation"] {
            let e = rag3weaver::schema::validate_identifier(nom, genre).unwrap_err().to_string();
            assert!(e.contains(nom), "le refus nomme le nom : {e}");
        }
    }
    let mut cat = catalogue();
    assert!(cat.register_entity("Ord`er", entite("title")).is_err());
    assert!(cat.register_entity("Purchase", entite("ord\"er")).is_err());
}

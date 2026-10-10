//! E2E : **un nom réservé du dialecte est refusé en le nommant**, avant
//! qu'aucune requête ne parte.
//!
//! Ticket `docs/tickets/2026-10-11-une-entite-au-nom-reserve-casse-l-ouverture.md` :
//! une entité `Order` passait le chargement, puis l'ouverture envoyait
//! `CREATE NODE TABLE IF NOT EXISTS Order(` et le parseur du moteur refusait
//! (« mismatched input 'Order' »). La règle (orchestration, 11 octobre 2026,
//! renversable par Lucie) : refuser le nom, dire lequel et quoi faire.
//!
//! Run with: ./run_e2e.sh --test e2e_nom_reserve

#![cfg(feature = "rag3db-native")]

use std::collections::HashMap;

use rag3weaver::config::{EntityConfig, FieldType, SimpleFieldDef};
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::{Catalog, CatalogConfig, Rag3dbConnection};

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

/// Une entité `Order` : refusée par son nom, et le message dit quoi faire.
#[test]
#[ignore]
fn une_entite_au_nom_reserve_est_refusee_en_le_nommant() {
    let mut cat = catalogue();
    let e = cat.register_entity("Order", entite("title")).map(|_| ()).unwrap_err().to_string();
    assert!(e.contains("Order") && e.contains("réservé") && e.contains("renommez"), "le refus nomme le mot et dit quoi faire : {e}");
}

/// Un champ au nom réservé : même refus, pour le champ.
#[test]
#[ignore]
fn un_champ_au_nom_reserve_est_refuse_en_le_nommant() {
    let mut cat = catalogue();
    let e = cat.register_entity("Purchase", entite("order")).map(|_| ()).unwrap_err().to_string();
    assert!(e.contains("order") && e.contains("réservé") && e.contains("renommez"), "{e}");
}

/// Un nom ordinaire passe, et un mot-clé que la grammaire accepte comme nom
/// (`Match`, non réservé) aussi : la liste suit la grammaire, pas l'intuition.
#[test]
#[ignore]
fn un_nom_ordinaire_et_un_mot_cle_non_reserve_passent() {
    let mut cat = catalogue();
    cat.register_entity("Purchase", entite("title")).unwrap();
    cat.register_entity("Match", entite("title")).unwrap();
}

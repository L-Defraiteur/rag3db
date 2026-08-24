//! E2E — multi-tenant natif : `org` × `project` (doc 37).
//!
//! Étapes A/B : colonnes système, nœuds `_Org`/`_Project`, stamp à
//! l'ingestion (entités, chunks, lignes d'index), migration des bases d'avant.
//! Les tests d'isolation de la recherche (étapes C/D) sont plus bas dans le
//! fichier, ajoutés au fur et à mesure.
//!
//! Run: cargo test --features rag3db-native --test e2e_scope -- --ignored --test-threads=1
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::FieldType;
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::scope::{Scope, ORG_COLUMN, PROJECT_COLUMN};
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest)
            .join("../..")
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .to_string()
    })
}

fn load_extensions(conn: &dyn DbConnection) {
    let root = rag3db_root();
    let path = format!("{root}/extension/vector/build/libvector.rag3db_extension");
    assert!(std::path::Path::new(&path).exists(), "extension vector absente : {path} (run_e2e.sh --build-only)");
    conn.execute(&format!("LOAD EXTENSION '{path}'")).expect("load vector");
}

fn make_empty_config(dim: usize) -> CatalogConfig {
    CatalogConfig {
        name: Some("scope-test".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: dim,
        ..Default::default()
    }
}

fn make_product_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: false, ..Default::default() });
    fields.insert("description".into(), SimpleFieldDef { field_type: FieldType::Text, is_title: false, is_content: true, ..Default::default() });
    fields.insert("price".into(), SimpleFieldDef { field_type: FieldType::Double, is_title: false, is_content: false, ..Default::default() });
    EntityConfig { fields, signals: SearchSignals::BM25, ..Default::default() }
}

fn make_product(name: &str, description: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("name".into(), CypherValue::String(name.into()));
    d.insert("description".into(), CypherValue::String(description.into()));
    d.insert("price".into(), CypherValue::Float(1.0));
    d
}

fn catalog_in_memory() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), make_empty_config(4));
    catalog.initialize().unwrap();
    catalog
}

fn catalog_on_disk(db: &str) -> Catalog {
    let conn = Rag3dbConnection::new(db).expect("open DB");
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), make_empty_config(4));
    catalog.initialize().unwrap();
    catalog
}

/// `RETURN` de deux colonnes STRING sur toutes les lignes d'une table.
fn scope_pairs(catalog: &Catalog, table: &str) -> Vec<(String, String)> {
    let res = catalog
        .conn_arc()
        .execute(&format!("MATCH (n:{table}) RETURN n.{ORG_COLUMN}, n.{PROJECT_COLUMN}"))
        .unwrap();
    res.rows
        .iter()
        .map(|r| {
            (
                r.get(0).and_then(|v| v.as_str()).unwrap_or("<null>").to_string(),
                r.get(1).and_then(|v| v.as_str()).unwrap_or("<null>").to_string(),
            )
        })
        .collect()
}

fn ids(catalog: &Catalog, table: &str) -> Vec<String> {
    let res = catalog.conn_arc().execute(&format!("MATCH (n:{table}) RETURN n._uuid")).unwrap();
    let mut v: Vec<String> = res.rows.iter().filter_map(|r| r.get(0).and_then(|v| v.as_str()).map(String::from)).collect();
    v.sort();
    v
}

// ─── A/B : stamp, nœuds, migration ──────────────────────────────────────────

#[test]
#[ignore]
fn scope_default_is_stamped_without_ceremony() {
    let mut catalog = catalog_in_memory();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.ingest_entities("Product", vec![make_product("A", "alpha text")]).unwrap();

    assert_eq!(scope_pairs(&catalog, "Product"), vec![("default".into(), "default".into())]);
    let chunks = scope_pairs(&catalog, "Product_Chunk");
    assert!(!chunks.is_empty(), "des chunks doivent exister");
    assert!(chunks.iter().all(|p| p == &("default".into(), "default".into())), "chunks: {chunks:?}");
    assert_eq!(ids(&catalog, "_Org"), vec!["default"]);
    assert_eq!(ids(&catalog, "_Project"), vec!["default"]);
}

#[test]
#[ignore]
fn scope_stamps_entities_chunks_and_creates_nodes() {
    let mut catalog = catalog_in_memory();
    catalog.set_scope(Scope::new("acme/eu", "search")).unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.ingest_entities("Product", vec![make_product("A", "alpha text"), make_product("B", "beta text")]).unwrap();

    let want = ("acme/eu".to_string(), "search".to_string());
    let rows = scope_pairs(&catalog, "Product");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|p| p == &want), "rows: {rows:?}");
    let chunks = scope_pairs(&catalog, "Product_Chunk");
    assert!(!chunks.is_empty() && chunks.iter().all(|p| p == &want), "chunks: {chunks:?}");

    // Changer de cellule : les lignes suivantes portent la nouvelle, les anciennes gardent la leur.
    catalog.set_scope(Scope::new("acme/eu", "billing")).unwrap();
    catalog.ingest_entities("Product", vec![make_product("C", "gamma text")]).unwrap();
    let mut rows = scope_pairs(&catalog, "Product");
    rows.sort();
    assert_eq!(rows, vec![
        ("acme/eu".to_string(), "billing".to_string()),
        ("acme/eu".to_string(), "search".to_string()),
        ("acme/eu".to_string(), "search".to_string()),
    ]);
    assert_eq!(ids(&catalog, "_Org"), vec!["acme/eu", "default"]);
    assert_eq!(ids(&catalog, "_Project"), vec!["billing", "default", "search"]);
}

#[test]
#[ignore]
fn scope_rejects_unsafe_ids_and_reserved_fields() {
    let mut catalog = catalog_in_memory();
    assert!(catalog.set_scope(Scope::new("acme corp", "x")).is_err(), "espace interdit");
    assert!(catalog.set_scope(Scope::new("a/../b", "x")).is_err(), "'..' interdit");
    assert_eq!(catalog.scope(), &Scope::default(), "un scope refusé ne remplace pas le courant");

    let mut cfg = make_product_config();
    cfg.fields.insert(ORG_COLUMN.into(), SimpleFieldDef { field_type: FieldType::String, ..Default::default() });
    assert!(catalog.register_entity("Bad", cfg).is_err(), "`_org` est réservé");
}

#[test]
#[ignore]
fn scope_migration_adds_columns_to_a_pre_scope_database() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("old.db").to_string_lossy().to_string();

    {
        let mut catalog = catalog_on_disk(&db);
        catalog.register_entity("Product", make_product_config()).unwrap();
        catalog.ingest_entities("Product", vec![make_product("Old", "text from before")]).unwrap();
        // Simuler une base d'avant : plus de colonnes de scope, plus de version.
        let conn = catalog.conn_arc();
        for table in ["Product", "Product_Chunk"] {
            for col in [ORG_COLUMN, PROJECT_COLUMN] {
                conn.execute(&format!("ALTER TABLE {table} DROP {col}")).unwrap();
            }
        }
        conn.execute("MATCH (m:_catalog_meta {_key: 'schema_version'}) DELETE m").unwrap();
        assert!(
            conn.execute("MATCH (n:Product) RETURN n._org").is_err(),
            "la colonne doit avoir disparu avant la migration"
        );
    }

    let catalog = catalog_on_disk(&db);
    assert_eq!(
        scope_pairs(&catalog, "Product"),
        vec![("default".into(), "default".into())],
        "la réouverture doit rajouter les colonnes avec 'default'"
    );
    let chunks = scope_pairs(&catalog, "Product_Chunk");
    assert!(!chunks.is_empty() && chunks.iter().all(|p| p == &("default".into(), "default".into())), "chunks: {chunks:?}");
}

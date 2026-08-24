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
use rag3weaver::filter::{FilterCondition, FilterValue};
use rag3weaver::scope::{Scope, ORG_COLUMN, PROJECT_COLUMN};
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
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
    make_product_config_with(SearchSignals::BM25)
}

fn make_product_config_with(signals: SearchSignals) -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, is_content: false, ..Default::default() });
    fields.insert("description".into(), SimpleFieldDef { field_type: FieldType::Text, is_title: false, is_content: true, ..Default::default() });
    fields.insert("price".into(), SimpleFieldDef { field_type: FieldType::Double, is_title: false, is_content: false, ..Default::default() });
    EntityConfig { fields, signals, ..Default::default() }
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

// ─── C/D : isolation de la recherche ────────────────────────────────────────

fn search_names(catalog: &mut Catalog, q: &str, signals: SearchSignals) -> Vec<String> {
    let resp = catalog
        .search("Product", q, SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(signals),
            limit: 50,
            ..Default::default()
        })
        .unwrap();
    let mut names: Vec<String> = resp
        .results
        .iter()
        .filter_map(|r| r.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str()).map(String::from))
        .collect();
    names.sort();
    names.dedup();
    names
}

fn blob_keys(catalog: &Catalog) -> Vec<String> {
    let res = catalog.conn_arc().execute("MATCH (b:_index_blobs) RETURN b._key").unwrap();
    res.rows.iter().filter_map(|r| r.get(0).and_then(|v| v.as_str()).map(String::from)).collect()
}

/// Deux cellules, même entité, même texte : chaque cellule a son propre index
/// (blobs préfixés par la cellule) et BM25 ne voit jamais l'autre.
#[test]
#[ignore]
fn scope_bm25_index_per_cell_never_sees_the_other_cell() {
    let mut catalog = catalog_in_memory();
    catalog.register_entity("Product", make_product_config()).unwrap();

    catalog.set_scope(Scope::new("acme", "alpha")).unwrap();
    catalog.ingest_entities("Product", vec![make_product("A-shared", "kernel scheduler notes"), make_product("A-only", "alpha private text")]).unwrap();
    catalog.set_scope(Scope::new("acme", "beta")).unwrap();
    catalog.ingest_entities("Product", vec![make_product("B-shared", "kernel scheduler notes"), make_product("B-only", "beta private text")]).unwrap();

    let keys = blob_keys(&catalog);
    assert!(keys.iter().any(|k| k.starts_with("Lucivy_Product__acme__alpha")), "blobs alpha: {keys:?}");
    assert!(keys.iter().any(|k| k.starts_with("Lucivy_Product__acme__beta")), "blobs beta: {keys:?}");
    assert!(!keys.iter().any(|k| k.starts_with("Lucivy_Product/") || k == "Lucivy_Product"), "pas d'index partagé: {keys:?}");

    // Cellule courante = beta.
    assert_eq!(search_names(&mut catalog, "kernel scheduler", SearchSignals::BM25), vec!["B-shared"]);
    assert!(search_names(&mut catalog, "alpha private", SearchSignals::BM25).is_empty());

    catalog.set_scope(Scope::new("acme", "alpha")).unwrap();
    assert_eq!(search_names(&mut catalog, "kernel scheduler", SearchSignals::BM25), vec!["A-shared"]);
    assert!(search_names(&mut catalog, "beta private", SearchSignals::BM25).is_empty());
}

/// Le vecteur HNSW reste par table : l'isolation vient du filtre sur les
/// colonnes de scope (étape D). Échoue tant que ce filtre n'existe pas.
#[test]
#[ignore]
fn scope_vector_search_never_sees_the_other_cell() {
    let mut catalog = catalog_in_memory();
    catalog.register_entity("Product", make_product_config_with(SearchSignals::HYBRID)).unwrap();

    catalog.set_scope(Scope::new("acme", "alpha")).unwrap();
    catalog.ingest_entities("Product", vec![make_product("A1", "kernel scheduler notes"), make_product("A2", "alpha private text")]).unwrap();
    catalog.set_scope(Scope::new("acme", "beta")).unwrap();
    catalog.ingest_entities("Product", vec![make_product("B1", "kernel scheduler notes"), make_product("B2", "beta private text")]).unwrap();

    let names = search_names(&mut catalog, "kernel scheduler notes", SearchSignals::VECTOR);
    assert!(!names.is_empty(), "le vecteur doit trouver quelque chose dans beta");
    assert!(names.iter().all(|n| n.starts_with('B')), "fuite vectorielle entre cellules : {names:?}");

    let names = search_names(&mut catalog, "kernel scheduler notes", SearchSignals::HYBRID);
    assert!(names.iter().all(|n| n.starts_with('B')), "fuite hybride entre cellules : {names:?}");
}

fn search_with(catalog: &mut Catalog, q: &str, opts: SearchOptions) -> (Vec<String>, Vec<String>) {
    let resp = catalog.search("Product", q, opts).unwrap();
    let mut names: Vec<String> = resp
        .results
        .iter()
        .filter_map(|r| r.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str()).map(String::from))
        .collect();
    names.sort();
    names.dedup();
    (names, resp.meta.warnings.clone())
}

/// `SearchOptions.scope` : chercher dans une autre cellule sans changer la
/// cellule courante du Catalog ; `scopes` : fan-out + fusion par rang.
#[test]
#[ignore]
fn scope_option_and_fan_out_across_cells() {
    let mut catalog = catalog_in_memory();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.set_scope(Scope::new("acme", "alpha")).unwrap();
    catalog.ingest_entities("Product", vec![make_product("A-shared", "kernel scheduler notes")]).unwrap();
    catalog.set_scope(Scope::new("acme", "beta")).unwrap();
    catalog.ingest_entities("Product", vec![make_product("B-shared", "kernel scheduler notes"), make_product("B-only", "beta private text")]).unwrap();

    let bm25 = |scope: Option<Scope>, scopes: Vec<Scope>| SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        limit: 50,
        scope,
        scopes,
        ..Default::default()
    };

    // Cellule explicite (alpha) alors que la courante est beta.
    let (names, _) = search_with(&mut catalog, "kernel scheduler", bm25(Some(Scope::new("acme", "alpha")), vec![]));
    assert_eq!(names, vec!["A-shared"]);
    assert_eq!(catalog.scope(), &Scope::new("acme", "beta"), "la cellule courante est restaurée");

    // Fan-out sur les deux cellules.
    let (names, warnings) = search_with(&mut catalog, "kernel scheduler", bm25(None, vec![Scope::new("acme", "alpha"), Scope::new("acme", "beta")]));
    assert_eq!(names, vec!["A-shared", "B-shared"]);
    assert!(warnings.iter().any(|w| w.contains("fan-out")), "warnings: {warnings:?}");
    assert_eq!(catalog.scope(), &Scope::new("acme", "beta"));

    // Fan-out avec pagination : limit 1 → un seul résultat, offset 1 → l'autre.
    let mut o = bm25(None, vec![Scope::new("acme", "alpha"), Scope::new("acme", "beta")]);
    o.limit = 1;
    let (first, _) = search_with(&mut catalog, "kernel scheduler", o.clone());
    o.offset = 1;
    let (second, _) = search_with(&mut catalog, "kernel scheduler", o);
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_ne!(first, second);
}

/// Un `FilterCondition` utilisateur (champ ordinaire) reste appliqué au BM25
/// à l'intérieur de la cellule — premier test E2E du filtre.
#[test]
#[ignore]
fn scope_user_filter_condition_applies_inside_cell() {
    let mut catalog = catalog_in_memory();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.set_scope(Scope::new("acme", "alpha")).unwrap();
    catalog.ingest_entities("Product", vec![
        make_product("Cheap", "kernel scheduler notes"),
        make_product("Pricey", "kernel scheduler notes"),
    ]).unwrap();
    let cond = FilterCondition::Field {
        key: "name".into(),
        value: FilterValue::Direct(CypherValue::String("Pricey".into())),
    };
    let (names, _) = search_with(&mut catalog, "kernel scheduler", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        filter_condition: Some(cond),
        ..Default::default()
    });
    assert_eq!(names, vec!["Pricey"]);
}

/// CANARI — bug kuzu/extension vector : `QUERY_VECTOR_INDEX` sur un graphe
/// projeté par Cypher rend des nœuds **hors** projection. C'est pour ça que
/// `Catalog::search` post-filtre les hits vectoriels par colonnes de scope
/// (sur-fetch ×4). Ce test affirme le bug : le jour où il échoue, kuzu est
/// corrigé et le post-filtre peut sauter (et les filtres vectoriels
/// utilisateur redeviennent de vrais pré-filtres).
#[test]
#[ignore]
fn canary_kuzu_projected_graph_vector_filter_is_ignored() {
    let mut catalog = catalog_in_memory();
    catalog.register_entity("Product", make_product_config_with(SearchSignals::HYBRID)).unwrap();
    catalog.set_scope(Scope::new("acme", "alpha")).unwrap();
    catalog.ingest_entities("Product", vec![make_product("A1", "kernel scheduler notes")]).unwrap();
    catalog.set_scope(Scope::new("acme", "beta")).unwrap();
    catalog.ingest_entities("Product", vec![make_product("B1", "kernel scheduler notes")]).unwrap();
    let conn = catalog.conn_arc();
    let rows = conn.execute("MATCH (c:Product_Chunk) RETURN c._project, c._uuid").unwrap();
    eprintln!("chunks: {:?}", rows.rows.iter().map(|r| format!("{:?}", r.get(0))).collect::<Vec<_>>());
    conn.execute(r#"CALL PROJECT_GRAPH_CYPHER('g_probe', 'MATCH (n:Product_Chunk) WHERE n._project = \'beta\' RETURN n')"#).unwrap();
    let emb = "[0.1, 0.2, 0.3, 0.4]";
    let r = conn.execute(&format!("CALL QUERY_VECTOR_INDEX('g_probe', 'Product_Chunk_vec', {emb}, 10) RETURN node._project, node._uuid, distance")).unwrap();
    let projs: Vec<String> = r.rows.iter().map(|row| format!("{:?}", row.get(0))).collect();
    eprintln!("projected query → {projs:?}");
    let r2 = conn.execute(&format!("CALL QUERY_VECTOR_INDEX('Product_Chunk', 'Product_Chunk_vec', {emb}, 10) RETURN node._project")).unwrap();
    eprintln!("plain query → {:?}", r2.rows.iter().map(|row| format!("{:?}", row.get(0))).collect::<Vec<_>>());
    assert!(
        projs.iter().any(|p| p.contains("alpha")),
        "kuzu respecte maintenant la projection ({projs:?}) : retirer scope_post_filter et ce canari"
    );
}

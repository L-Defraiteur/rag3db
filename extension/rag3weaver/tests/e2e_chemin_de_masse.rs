//! **Le chemin de masse d'une première ingestion.**
//!
//! Sur une table vide, les lignes partent par `COPY … FROM` un CSV, et les
//! chunks arrivent à l'insertion avec leurs vecteurs — au lieu d'un MERGE
//! ligne à ligne suivi d'un SET par vecteur (6 septembre 2026). Ces tests
//! fixent deux contrats : ce que le CSV du moteur garde (chaînes à
//! guillemets, sauts de ligne, chaîne vide, NULL, vecteurs), et ce qu'une
//! première ingestion laisse en base — identique à ce que laissait le chemin
//! de toujours, qui reprend dès la seconde.
//!
//! Run with: ./run_e2e.sh --test e2e_chemin_de_masse

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::FieldType;
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::dialect::{Rag3dbDialect, SchemaDialect};
use rag3weaver::disponibilite::RegimeEcriture;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn load_extensions(conn: &dyn DbConnection) {
    let root = rag3db_root();
    let ext = format!("{root}/extension/vector/build/libvector.rag3db_extension");
    assert!(std::path::Path::new(&ext).exists(), "extension vector absente : {ext} — ./run_e2e.sh --build-only");
    conn.execute(&format!("LOAD EXTENSION '{ext}'")).expect("LOAD EXTENSION vector");
}

fn product_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("description".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    fields.insert("price".into(), SimpleFieldDef { field_type: FieldType::Double, ..Default::default() });
    // L'identité par le nom : modifier la description met à jour, ne crée pas.
    EntityConfig { fields, signals: SearchSignals::HYBRID, hashsafe: Some(vec!["name".into()]), ..Default::default() }
}

fn product(name: &str, description: &str, price: f64) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("description".into(), CypherValue::String(description.into()));
    data.insert("price".into(), CypherValue::Float(price));
    data
}

fn catalogue(dim: usize) -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let config = CatalogConfig { name: Some("chemin-de-masse".into()), embedding_dim: dim, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Product", product_config()).unwrap();
    catalog.regime_d_ecriture(RegimeEcriture::ParLot);
    catalog
}

fn compte(catalog: &Catalog, cypher: &str) -> i64 {
    catalog.execute_raw(cypher).unwrap().rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).unwrap_or(-1)
}

/// **Ce que le CSV du moteur garde.** C'est le format qu'écrit
/// `InsertRecordNode` en mode COPY : pas d'en-tête, guillemet doublé, chaîne
/// vide entre guillemets, NULL par un mot convenu, vecteur `"[a,b,c]"`.
#[test]
#[ignore]
fn le_csv_du_moteur_garde_les_chaines_les_vides_et_les_vecteurs() {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    conn.execute("CREATE NODE TABLE T(_uuid STRING, texte STRING, vide STRING, n INT64, f DOUBLE, b BOOLEAN, vec FLOAT[3], PRIMARY KEY(_uuid))").unwrap();

    let chemin = std::env::temp_dir().join(format!("rag3weaver-test-masse-{}.csv", std::process::id()));
    let texte = "ligne 1, avec virgule\nligne 2 \"citée\" et \\ barre\r\nligne 3";
    let cellule = format!("\"{}\"", texte.replace('"', "\"\""));
    std::fs::write(
        &chemin,
        format!("a,{cellule},\"\",42,1.5,true,\"[0.25,-1,3.5]\"\nb,simple,{n},7,2.5,false,\"[1,2,3]\"\n", n = rag3weaver::dialect::CSV_NULL),
    )
    .unwrap();
    let copie = Rag3dbDialect.copy_nodes_from_csv("T", &["_uuid", "texte", "vide", "n", "f", "b", "vec"], &chemin.to_string_lossy()).unwrap();
    conn.execute(&copie).expect("COPY T");
    let _ = std::fs::remove_file(&chemin);

    let lu = conn.execute("MATCH (t:T) RETURN t._uuid, t.texte, t.vide, t.n, t.f, t.b, t.vec ORDER BY t._uuid").unwrap();
    assert_eq!(lu.rows.len(), 2);
    let a = &lu.rows[0];
    assert_eq!(a[1].as_str(), Some(texte), "la chaîne à guillemets et sauts de ligne revient telle quelle");
    assert_eq!(a[2].as_str(), Some(""), "une chaîne vide entre guillemets reste une chaîne, pas un NULL");
    assert_eq!(a[3].as_i64(), Some(42));
    assert_eq!(a[4], CypherValue::Float(1.5));
    assert_eq!(a[5], CypherValue::Bool(true));
    let vec_a: Vec<f64> = match &a[6] { CypherValue::List(v) => v.iter().filter_map(|x| if let CypherValue::Float(f) = x { Some(*f) } else { None }).collect(), autre => panic!("vecteur attendu, lu {autre:?}") };
    assert_eq!(vec_a, vec![0.25, -1.0, 3.5]);
    let b = &lu.rows[1];
    assert_eq!(b[1].as_str(), Some("simple"));
    assert!(b[2].is_null(), "le mot convenu est NULL");
    assert_eq!(b[3].as_i64(), Some(7));
}

/// **Une première ingestion laisse la même base que le chemin de toujours**,
/// et le chemin de toujours reprend dès que la table n'est plus vide.
#[test]
#[ignore]
fn une_premiere_ingestion_passe_par_la_masse_et_la_seconde_par_le_merge() {
    let mut catalog = catalogue(4);
    let texte_difficile = "Un couteau \"forgé\", à lame haute teneur en carbone,\navec une virgule, un retour à la ligne\r\net une barre \\ oblique.";
    let lot = vec![
        product("Rust Book", "A comprehensive guide to Rust programming: ownership, lifetimes, concurrency.", 49.99),
        product("Python Cookbook", "Recipes for mastering Python with data science and automation.", 39.99),
        product("Couteau", texte_difficile, 129.99),
        // Un doublon dans le lot : la dernière occurrence gagne, COPY ne heurte pas.
        product("Rust Book", "A comprehensive guide to Rust programming: ownership, lifetimes, concurrency.", 49.99),
    ];
    let r = catalog.ingest_entities("Product", lot).unwrap();
    assert_eq!(r.failed, 0, "{:?}", r.warnings);
    assert!(
        !r.warnings.iter().any(|w| w.contains("chargement en masse refusé")),
        "le chemin de masse ne doit pas retomber sur le MERGE : {:?}",
        r.warnings
    );

    assert_eq!(compte(&catalog, "MATCH (p:Product) RETURN count(p)"), 3);
    let chunks = compte(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    assert!(chunks >= 3, "chunks={chunks}");
    assert_eq!(compte(&catalog, "MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]->(:Product) RETURN count(c)"), chunks, "chaque chunk est lié à son parent");
    assert_eq!(compte(&catalog, "MATCH (c:Product_Chunk) WHERE c._embed_hash <> '' AND c._embed_hash = c._text_hash RETURN count(c)"), chunks, "chaque chunk porte son marqueur dense");
    assert_eq!(compte(&catalog, "MATCH (c:Product_Chunk) WHERE size(c.embedding) = 4 RETURN count(c)"), chunks, "chaque chunk porte son vecteur");
    assert_eq!(compte(&catalog, "MATCH (p:Product) WHERE p._chunked_hash = p._content_hash RETURN count(p)"), 3, "chaque parent est marqué découpé");
    let lu = catalog.execute_raw("MATCH (p:Product {name: 'Couteau'}) RETURN p.description").unwrap();
    assert_eq!(lu.rows[0][0].as_str(), Some(texte_difficile), "le texte à guillemets et sauts de ligne revient tel quel");

    // Le plein texte (lucivy, par décalage relu après le COPY) et le vecteur.
    let bm25 = catalog.search("Product", "Rust programming", SearchOptions { consistency: Consistency::Immediate, signals: Some(SearchSignals::BM25), ..Default::default() }).unwrap();
    assert!(!bm25.results.is_empty(), "BM25 trouve après une première ingestion");
    let vecteur = catalog.search("Product", "Rust programming", SearchOptions { consistency: Consistency::Immediate, signals: Some(SearchSignals::SEMANTIC), ..Default::default() }).unwrap();
    assert!(!vecteur.results.is_empty(), "le vecteur trouve après une première ingestion");

    // Seconde ingestion, table non vide : le MERGE, avec un modifié et un nouveau.
    let r2 = catalog
        .ingest_entities(
            "Product",
            vec![
                product("Rust Book", "A comprehensive guide to Rust programming: ownership, lifetimes, concurrency.", 49.99),
                product("Couteau", "Un couteau de cuisine, tout simplement.", 99.0),
                product("Wok", "A carbon steel wok for high heat cooking.", 59.0),
            ],
        )
        .unwrap();
    assert_eq!(r2.failed, 0, "{:?}", r2.warnings);
    assert_eq!(r2.unchanged, 1, "le Rust Book identique est sauté");
    assert_eq!(compte(&catalog, "MATCH (p:Product) RETURN count(p)"), 4);
    let chunks2 = compte(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    assert_eq!(compte(&catalog, "MATCH (c:Product_Chunk) WHERE c._embed_hash = c._text_hash AND size(c.embedding) = 4 RETURN count(c)"), chunks2, "tous les chunks embarqués, après le MERGE aussi");
    let lu = catalog.execute_raw("MATCH (p:Product {name: 'Couteau'}) RETURN p.description, p.price").unwrap();
    assert_eq!(lu.rows[0][0].as_str(), Some("Un couteau de cuisine, tout simplement."));
    assert_eq!(lu.rows[0][1], CypherValue::Float(99.0));
    let wok = catalog.search("Product", "wok", SearchOptions { consistency: Consistency::Immediate, signals: Some(SearchSignals::BM25), ..Default::default() }).unwrap();
    assert!(!wok.results.is_empty(), "le nouveau venu se trouve");
}

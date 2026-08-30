//! E2E : le même catalogue, sur PostgreSQL + pgvector.
//!
//! Pourquoi cette suite existe : `PostgresDialect` fait 944 lignes, compile, et
//! n'avait **aucun test contre une vraie base**. Une boucle étrange qui ne sert
//! que kuzu ne sert pas les projets réels — et un dialecte qu'on n'a jamais
//! exécuté est une supposition, pas un backend.
//!
//! La base :
//!
//! ```sh
//! docker run -d --name rag3weaver-pg \
//!   -e POSTGRES_USER=rag3weaver -e POSTGRES_PASSWORD=rag3weaver \
//!   -e POSTGRES_DB=rag3weaver_test -p 5433:5432 pgvector/pgvector:pg17
//! ```
//!
//! Puis : `cargo test --features postgres --test e2e_postgres -- --nocapture`
//!
//! **La suite ne se saute pas en silence.** Si la base n'est pas là, chaque
//! test échoue en disant comment la démarrer : un « 0 passed » vert est un saut
//! déguisé, et c'est précisément ce qu'on essaie de ne plus faire.

#![cfg(feature = "postgres")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rag3weaver::config::FieldType;
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::dialect::PostgresDialect;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::postgres_connection::PostgresConnection;
use rag3weaver::postgres_search_backend::PostgresSearchBackend;
use rag3weaver::search::SearchSignals;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, SimpleFieldDef};

// ─── Le socle ────────────────────────────────────────────────────────────────

/// Chaîne de connexion : surchargeable, avec le conteneur ci-dessus par défaut.
fn conn_str() -> String {
    std::env::var("RAG3WEAVER_PG").unwrap_or_else(|_| {
        "host=localhost port=5433 user=rag3weaver password=rag3weaver \
         dbname=rag3weaver_test"
            .to_string()
    })
}

/// Le runtime et sa garde.
///
/// `PostgresConnection::execute` est **synchrone** et appelle
/// `Handle::current().block_on` : il lui faut donc un contexte tokio *sans*
/// être dans une tâche. C'est exactement ce que donne `rt.enter()` tenu par un
/// fil de test ordinaire.
///
/// L'ordre de déclaration compte : `rt` d'abord, la garde ensuite, pour que la
/// garde tombe avant le runtime — l'inverse panique.
struct Contexte {
    _garde: tokio::runtime::EnterGuard<'static>,
    rt: &'static tokio::runtime::Runtime,
}

impl Contexte {
    fn ouvrir() -> (Self, Arc<PostgresConnection>) {
        // Fuite volontaire : la garde emprunte le runtime, et un test n'a pas
        // besoin de le rendre. C'est un test, pas un serveur.
        let rt: &'static tokio::runtime::Runtime = Box::leak(Box::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("runtime tokio"),
        ));

        let conn = rt.block_on(PostgresConnection::new(&conn_str())).unwrap_or_else(|e| {
            panic!(
                "PostgreSQL injoignable ({e}).\n\
                 Démarre-le :\n  docker run -d --name rag3weaver-pg \\\n\
                 \x20   -e POSTGRES_USER=rag3weaver -e POSTGRES_PASSWORD=rag3weaver \\\n\
                 \x20   -e POSTGRES_DB=rag3weaver_test -p 5433:5432 pgvector/pgvector:pg17\n\
                 Ou pointe ailleurs avec RAG3WEAVER_PG=..."
            )
        });

        let garde = rt.enter();
        (Contexte { _garde: garde, rt }, Arc::new(conn))
    }
}

/// Repartir d'une base vide.
///
/// Deux schémas à raser : `public`, où vivent les tables d'entités, et
/// `rag3weaver`, celui que `internal_schema()` réclame pour la méta et les
/// blobs.
fn table_rase(conn: &dyn DbConnection) {
    for stmt in [
        "DROP SCHEMA IF EXISTS public CASCADE",
        "DROP SCHEMA IF EXISTS rag3weaver CASCADE",
        "CREATE SCHEMA public",
        "CREATE SCHEMA IF NOT EXISTS rag3weaver",
    ] {
        conn.execute(stmt)
            .unwrap_or_else(|e| panic!("table rase « {stmt} » : {e}"));
    }
}

fn config_vide(dim: usize) -> CatalogConfig {
    CatalogConfig {
        name: Some("postgres-e2e".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: dim,
        ..Default::default()
    }
}

fn config_produit() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "name".into(),
        SimpleFieldDef {
            field_type: FieldType::String,
            is_title: true,
            ..Default::default()
        },
    );
    fields.insert(
        "description".into(),
        SimpleFieldDef {
            field_type: FieldType::Text,
            is_content: true,
            ..Default::default()
        },
    );
    fields.insert(
        "price".into(),
        SimpleFieldDef {
            field_type: FieldType::Double,
            ..Default::default()
        },
    );
    EntityConfig {
        fields,
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }
}

fn produit(name: &str, description: &str, price: f64) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("name".into(), CypherValue::String(name.into()));
    d.insert("description".into(), CypherValue::String(description.into()));
    d.insert("price".into(), CypherValue::Float(price));
    d
}

/// Le catalogue branché sur postgres : dialecte, backend de recherche, et le
/// magasin de blobs qui va avec — les trois pièces qu'un backend doit fournir.
fn catalogue(dim: usize) -> (Contexte, Catalog) {
    let (ctx, conn) = Contexte::ouvrir();
    table_rase(conn.as_ref());

    let boxed: Box<dyn DbConnection> = Box::new(
        ctx.rt
            .block_on(PostgresConnection::new(&conn_str()))
            .expect("seconde connexion"),
    );

    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config_vide(dim));
    let partagee = catalog.conn_arc();
    catalog.set_dialect(Arc::new(PostgresDialect));
    catalog.set_search_backend(Arc::new(PostgresSearchBackend::new(partagee.clone())));
    catalog.set_blob_store(Arc::new(
        rag3weaver::postgres_blob_store::PostgresBlobStore::new(partagee),
    ));
    catalog.initialize().expect("initialize sur postgres");
    (ctx, catalog)
}

// ═══ 1. Le schéma se pose ════════════════════════════════════════════════════

#[test]
fn le_schema_se_pose() {
    let (_ctx, mut catalog) = catalogue(8);
    catalog
        .register_entity("Product", config_produit())
        .expect("register_entity");

    // `current_schema()` et pas `'public'` : les tables non qualifiées vont
    // dans le premier schéma du `search_path`, et `search_path` commence par
    // `"$user"`. Avec un rôle nommé comme le schéma interne, tout atterrit là.
    // Un test qui interroge `public` en dur ne verrait rien — et conclurait à
    // tort que rien n'a été créé.
    let tables = catalog
        .execute_raw(
            "SELECT table_name::text FROM information_schema.tables \
             WHERE table_schema = current_schema() ORDER BY table_name",
        )
        .expect("liste des tables");
    let noms: Vec<String> = tables
        .rows
        .iter()
        .filter_map(|r| r[0].as_str().map(|s| s.to_string()))
        .collect();
    eprintln!("tables : {noms:?}");

    assert!(noms.iter().any(|n| n == "product"), "table Product absente : {noms:?}");
    assert!(
        noms.iter().any(|n| n.contains("chunk")),
        "table de chunks absente : {noms:?}"
    );
}

// ═══ 2. L'ingestion écrit vraiment ═══════════════════════════════════════════

#[test]
fn l_ingestion_ecrit_des_lignes() {
    let (_ctx, mut catalog) = catalogue(8);
    catalog.register_entity("Product", config_produit()).unwrap();

    let res = catalog
        .ingest_entities(
            "Product",
            vec![
                produit(
                    "Rust Book",
                    "A comprehensive guide to Rust: ownership, lifetimes, concurrency.",
                    49.99,
                ),
                produit(
                    "Python Cookbook",
                    "Recipes for Python: data science, web development, automation.",
                    39.99,
                ),
                produit(
                    "French Chef Knife",
                    "Professional kitchen knife forged from high-carbon stainless steel.",
                    129.99,
                ),
            ],
        )
        .expect("ingest_entities");

    eprintln!("ingest : traités={} échoués={}", res.processed, res.failed);
    assert_eq!(res.failed, 0, "aucune ingestion ne doit échouer");

    let n = catalog
        .execute_raw("SELECT count(*) FROM product")
        .expect("count")
        .rows[0][0]
        .as_i64()
        .unwrap();
    assert_eq!(n, 3, "trois produits attendus");

    let chunks = catalog
        .execute_raw("SELECT count(*) FROM product_chunk")
        .expect("count chunks")
        .rows[0][0]
        .as_i64()
        .unwrap();
    assert!(chunks >= 3, "au moins un chunk par produit : {chunks}");
    eprintln!("✓ {n} produits, {chunks} chunks");
}

// ═══ 3. pgvector répond ══════════════════════════════════════════════════════

#[test]
fn le_vecteur_classe() {
    let (_ctx, mut catalog) = catalogue(8);
    catalog.register_entity("Product", config_produit()).unwrap();
    catalog
        .ingest_entities(
            "Product",
            vec![
                produit("Rust Book", "Ownership, lifetimes and concurrency in Rust.", 49.99),
                produit("French Chef Knife", "A forged kitchen knife for slicing.", 129.99),
            ],
        )
        .unwrap();

    let backend = catalog.search_backend().expect("backend de recherche");
    let requete = MockEmbedder::new(8);
    use rag3weaver::embedder::Embedder;
    let vecteur = requete.embed(&["Ownership, lifetimes and concurrency in Rust.".to_string()]).unwrap();

    let hits = backend
        .vector_search("product_chunk", "", &vecteur[0], 5)
        .expect("vector_search pgvector");
    eprintln!("hits : {}", hits.len());
    for h in &hits {
        eprintln!("  {} → {:.4}", h.uuid, h.score);
    }
    assert!(!hits.is_empty(), "pgvector ne renvoie rien");
}

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
// `HashEmbedder` et pas `MockEmbedder` : le second rend des vecteurs **nuls**,
// et la distance cosinus d'un vecteur nul est `NaN` — pgvector trie alors sur
// du NaN et son index HNSW ne rend rien. Un montage qui « marche » avec des
// zéros ne prouve rien du chemin vectoriel.
use rag3weaver::embedder::HashEmbedder;
use rag3weaver::postgres_connection::PostgresConnection;
use rag3weaver::postgres_search_backend::PostgresSearchBackend;
use rag3weaver::search::{Consistency, ResultMode, SearchOptions, SearchSignals};
use rag3weaver::scope::Scope;
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

/// **Une base, un test à la fois.**
///
/// Tous les tests partagent le même PostgreSQL et chacun rase le schéma au
/// démarrage : lancés en parallèle, ils se détruisent mutuellement — six sur
/// sept échouaient, et pour une raison qui n'avait rien à voir avec le code
/// testé. Un `--test-threads=1` dans la ligne de commande corrigeait le
/// symptôme et laissait le piège en place pour le suivant.
///
/// Le verrou est donc **dans la suite**, pas dans son invocation. Il est tenu
/// pendant tout le test, et `is_poisoned` est ignoré : si un test panique en le
/// tenant, les suivants doivent quand même pouvoir raser et repartir.
static VERROU_BASE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Le catalogue branché sur postgres : dialecte, backend de recherche, et le
/// magasin de blobs qui va avec — les trois pièces qu'un backend doit fournir.
fn catalogue(dim: usize) -> (std::sync::MutexGuard<'static, ()>, Contexte, Catalog) {
    let garde = VERROU_BASE.lock().unwrap_or_else(|e| e.into_inner());
    let (ctx, conn) = Contexte::ouvrir();
    table_rase(conn.as_ref());

    let boxed: Box<dyn DbConnection> = Box::new(
        ctx.rt
            .block_on(PostgresConnection::new(&conn_str()))
            .expect("seconde connexion"),
    );

    let mut catalog = Catalog::new(boxed, Box::new(HashEmbedder::new(dim)), config_vide(dim));
    let partagee = catalog.conn_arc();
    catalog.set_dialect(Arc::new(PostgresDialect));
    catalog.set_search_backend(Arc::new(PostgresSearchBackend::new(partagee.clone())));
    catalog.set_blob_store(Arc::new(
        rag3weaver::postgres_blob_store::PostgresBlobStore::new(partagee),
    ));
    catalog.initialize().expect("initialize sur postgres");
    (garde, ctx, catalog)
}

// ═══ 1. Le schéma se pose ════════════════════════════════════════════════════

#[test]
fn le_schema_se_pose() {
    let (_garde, _ctx, mut catalog) = catalogue(8);
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
    let (_garde, _ctx, mut catalog) = catalogue(8);
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
    let (_garde, _ctx, mut catalog) = catalogue(8);
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
    let requete = HashEmbedder::new(8);
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

// ═══ 4. Le plein texte trouve ════════════════════════════════════════════════
//
// Le pari à vérifier : lucivy vit **à côté** de la base, ses index passent par
// le magasin de blobs, donc le plein texte devrait être indifférent au backend.
// C'est exactement le genre de « devrait » que cette soirée a puni trois fois.

#[test]
fn le_plein_texte_trouve() {
    let (_garde, _ctx, mut catalog) = catalogue(8);
    catalog.register_entity("Product", config_produit()).unwrap();
    catalog
        .ingest_entities(
            "Product",
            vec![
                produit(
                    "Rust Book",
                    "A comprehensive guide to the Rust programming language: ownership, \
                     lifetimes and concurrency.",
                    49.99,
                ),
                produit(
                    "Python Cookbook",
                    "Recipes for the Python programming language: data science, web \
                     development, automation.",
                    39.99,
                ),
                produit(
                    "French Chef Knife",
                    "A professional kitchen knife forged from high-carbon stainless steel.",
                    129.99,
                ),
            ],
        )
        .unwrap();

    let reponse = catalog
        .search(
            "Product",
            "programming language",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        .expect("recherche BM25 sur postgres");

    eprintln!(
        "BM25 : {} résultats, bm25_count={}",
        reponse.results.len(),
        reponse.meta.bm25_count
    );
    for a in &reponse.meta.warnings {
        eprintln!("  avertissement : {a}");
    }
    for r in &reponse.results {
        eprintln!("  {:.4} — {:?}", r.score, r.data.as_ref().and_then(|d| d.get("name")));
    }

    // L'extrait vient du chunk lui-même : le trigramme cherche dans les
    // chunks, donc chaque résultat en porte un, sans aucun appariement de spans.
    for r in &reponse.results {
        let c = r.chunk.as_ref().expect("un résultat sans chunk : l'extrait est perdu");
        eprintln!("  extrait [{}..{}] : {}", c.start_char, c.end_char, c.text);
        assert!(!c.text.is_empty(), "chunk vide");
    }

    assert!(reponse.meta.bm25_count > 0, "le plein texte natif n'a rien rendu");
    assert!(!reponse.results.is_empty(), "aucun résultat plein texte");
    // Le couteau ne parle pas de langage de programmation : s'il sort premier,
    // c'est que la résolution des décalages rend n'importe quelle ligne.
    let premier = reponse.results[0]
        .data
        .as_ref()
        .and_then(|d| d.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    assert!(
        premier.contains("Rust") || premier.contains("Python"),
        "premier résultat inattendu : {premier}"
    );
}

// ═══ 5. Les deux signaux fusionnent ══════════════════════════════════════════

#[test]
fn l_hybride_fusionne() {
    let (_garde, _ctx, mut catalog) = catalogue(8);
    catalog.register_entity("Product", config_produit()).unwrap();
    catalog
        .ingest_entities(
            "Product",
            vec![
                produit("Rust Book", "Ownership, lifetimes and concurrency in Rust.", 49.99),
                produit("Python Cookbook", "Data science and automation in Python.", 39.99),
                produit("French Chef Knife", "A forged kitchen knife for slicing.", 129.99),
            ],
        )
        .unwrap();

    let reponse = catalog
        .search(
            "Product",
            "Ownership, lifetimes and concurrency in Rust.",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::HYBRID),
                result_mode: ResultMode::SourceResolved,
                ..Default::default()
            },
        )
        .expect("recherche hybride sur postgres");

    eprintln!(
        "hybride : {} résultats, bm25={} vecteur={}",
        reponse.results.len(),
        reponse.meta.bm25_count,
        reponse.meta.vector_count
    );
    for a in &reponse.meta.warnings {
        eprintln!("  avertissement : {a}");
    }

    assert!(reponse.meta.bm25_count > 0, "signal BM25 muet");
    assert!(reponse.meta.vector_count > 0, "signal vectoriel muet");
    assert!(!reponse.results.is_empty(), "la fusion ne rend rien");

    // `SourceResolved` remonte du chunk à son entité : c'est le chemin qui
    // joint la relation sur `to_uuid` — celui qui n'avait aucun index avant ce
    // soir, et qui rendrait silencieusement vide si la jointure échouait.
    let premier = reponse.results[0]
        .data
        .as_ref()
        .and_then(|d| d.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(premier, "Rust Book", "la remontée chunk → entité s'est perdue");
}

// ═══ 6. Les relations tiennent ═══════════════════════════════════════════════

/// Une entité minimale, pour être l'autre bout d'une relation.
fn config_variante() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "label".into(),
        SimpleFieldDef {
            field_type: FieldType::String,
            is_title: true,
            // `is_content` aussi : une entité sans champ de contenu est refusée
            // à l'enregistrement, faute de quoi indexer. Le titre fait les deux.
            is_content: true,
            ..Default::default()
        },
    );
    EntityConfig { fields, signals: SearchSignals::BM25, ..Default::default() }
}

fn variante(label: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("label".into(), CypherValue::String(label.into()));
    d
}

#[test]
fn les_relations_tiennent() {
    let (_garde, _ctx, mut catalog) = catalogue(8);
    catalog.register_entity("Product", config_produit()).unwrap();
    catalog.register_entity("Variant", config_variante()).unwrap();
    catalog
        .register_relation("HAS_VARIANT", "Product", "Variant")
        .expect("register_relation sur postgres");

    let livre = catalog
        .create("Product", produit("Rust Book", "Ownership and lifetimes.", 49.99))
        .unwrap();
    let poche = catalog.create("Variant", variante("Rust Book — poche")).unwrap();
    let relie = catalog.create("Variant", variante("Rust Book — relié")).unwrap();
    catalog
        .link("HAS_VARIANT", livre.clone(), poche, BTreeMap::new())
        .unwrap();
    catalog.link("HAS_VARIANT", livre, relie, BTreeMap::new()).unwrap();

    let vidage = catalog.drain();
    assert_eq!(vidage.failed, 0, "le vidage a échoué : {vidage:?}");

    // La table de relation est une vraie table sur postgres, avec sa clé
    // primaire composite. On vérifie les deux arêtes, et qu'elles partent bien
    // du même produit — c'est la jointure sur `to_uuid` qui n'avait aucun index
    // avant aujourd'hui.
    let n = catalog
        .execute_raw("SELECT count(*) FROM has_variant")
        .expect("count relations")
        .rows[0][0]
        .as_i64()
        .unwrap();
    assert_eq!(n, 2, "deux arêtes attendues");

    let jointure = catalog
        .execute_raw(
            "SELECT p.name, v.label FROM has_variant r \
             JOIN product p ON p._uuid = r.from_uuid \
             JOIN variant v ON v._uuid = r.to_uuid \
             ORDER BY v.label",
        )
        .expect("jointure des deux bouts");
    let paires: Vec<(String, String)> = jointure
        .rows
        .iter()
        .map(|r| {
            (
                r[0].as_str().unwrap_or("").to_string(),
                r[1].as_str().unwrap_or("").to_string(),
            )
        })
        .collect();
    eprintln!("arêtes : {paires:?}");
    assert_eq!(paires.len(), 2);
    assert!(paires.iter().all(|(p, _)| p == "Rust Book"), "{paires:?}");
}

// ═══ 7. Les cellules se séparent ═════════════════════════════════════════════

#[test]
fn les_cellules_se_separent() {
    let (_garde, _ctx, mut catalog) = catalogue(8);
    catalog.register_entity("Product", config_produit()).unwrap();

    let cellule_a = Scope { org: "acme".into(), project: "boutique".into() };
    let cellule_b = Scope { org: "globex".into(), project: "entrepot".into() };

    catalog.set_scope(cellule_a.clone()).expect("cellule A");
    catalog
        .ingest_entities(
            "Product",
            vec![produit("Rust Book", "Ownership, lifetimes and concurrency.", 49.99)],
        )
        .unwrap();

    catalog.set_scope(cellule_b.clone()).expect("cellule B");
    catalog
        .ingest_entities(
            "Product",
            vec![produit("Python Cookbook", "Ownership, lifetimes and concurrency.", 39.99)],
        )
        .unwrap();

    // Les deux descriptions sont **identiques** : si le cloisonnement fuit, la
    // recherche dans une cellule remonte le produit de l'autre avec le même
    // score, et rien ne le signalera.
    let reponse = catalog
        .search(
            "Product",
            "Ownership, lifetimes and concurrency.",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        .expect("recherche dans la cellule B");

    let noms: Vec<String> = reponse
        .results
        .iter()
        .filter_map(|r| r.data.as_ref()?.get("name")?.as_str().map(|s| s.to_string()))
        .collect();
    eprintln!("cellule B voit : {noms:?}");

    assert!(
        noms.contains(&"Python Cookbook".to_string()),
        "la cellule B ne voit pas son propre produit : {noms:?}"
    );
    assert!(
        !noms.contains(&"Rust Book".to_string()),
        "FUITE ENTRE CELLULES : la cellule B voit le produit de la cellule A — {noms:?}"
    );

    // **Et le même cloisonnement sur le chemin vectoriel**, qui passe par un
    // autre code : `vector_search_filtered` reçoit un `WHERE` construit par le
    // catalogue avec l'alias `n.`, alors que la requête postgres n'aliase pas
    // sa table. Si c'est faux, ça se verra ici — soit par une erreur SQL, soit
    // par une fuite.
    let hybride = catalog
        .search(
            "Product",
            "Ownership, lifetimes and concurrency.",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::HYBRID),
                ..Default::default()
            },
        )
        .expect("recherche hybride cloisonnée sur postgres");

    let noms_h: Vec<String> = hybride
        .results
        .iter()
        .filter_map(|r| r.data.as_ref()?.get("name")?.as_str().map(|s| s.to_string()))
        .collect();
    eprintln!(
        "hybride cellule B voit : {noms_h:?}  (bm25={} vecteur={})",
        hybride.meta.bm25_count, hybride.meta.vector_count
    );
    assert!(
        !noms_h.contains(&"Rust Book".to_string()),
        "FUITE ENTRE CELLULES sur le chemin vectoriel — {noms_h:?}"
    );
}

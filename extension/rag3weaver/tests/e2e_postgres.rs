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
    catalogue_avec(dim, rag3weaver::search_backend::MoteurTexte::Auto)
}

/// Le même montage, moteur de texte imposé.
fn catalogue_avec(
    dim: usize,
    moteur: rag3weaver::search_backend::MoteurTexte,
) -> (std::sync::MutexGuard<'static, ()>, Contexte, Catalog) {
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
    // **Avant `initialize`** : c'est à l'ingestion que la décision coûte,
    // puisqu'elle détermine si un index lucivy s'écrit sur disque.
    catalog.set_moteur_texte(moteur);
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

// ═══ 8. Les accents ne coupent pas la recherche ══════════════════════════════

#[test]
fn les_accents_ne_coupent_pas() {
    let (_garde, _ctx, mut catalog) = catalogue(8);
    catalog.register_entity("Product", config_produit()).unwrap();
    catalog
        .ingest_entities(
            "Product",
            vec![
                produit("Café serré", "Un café torréfié à Naples, préparé très serré.", 3.50),
                produit("Marteau", "Un outil de charpentier en acier forgé.", 19.90),
            ],
        )
        .unwrap();

    // Sans normalisation, « cafe torrefie » ne partage aucun trigramme utile
    // avec « café torréfié » — un utilisateur francophone qui tape sans
    // accents ne trouve rien, ce qui est la moitié des requêtes réelles.
    for requete in ["cafe torrefie", "café torréfié", "CAFE"] {
        let reponse = catalog
            .search(
                "Product",
                requete,
                SearchOptions {
                    consistency: Consistency::Immediate,
                    signals: Some(SearchSignals::BM25),
                    ..Default::default()
                },
            )
            .unwrap_or_else(|e| panic!("recherche « {requete} » : {e}"));
        let noms: Vec<String> = reponse
            .results
            .iter()
            .filter_map(|r| r.data.as_ref()?.get("name")?.as_str().map(|s| s.to_string()))
            .collect();
        eprintln!("« {requete} » → {noms:?}");
        assert!(
            noms.contains(&"Café serré".to_string()),
            "« {requete} » ne trouve pas « Café serré » : {noms:?}"
        );
        assert!(
            !noms.contains(&"Marteau".to_string()),
            "« {requete} » remonte le marteau : {noms:?}"
        );
    }
}

// ═══ 9. Le filtre utilisateur tient sur les deux chemins ═════════════════════

/// **Un filtre ignoré est pire qu'un filtre refusé.**
///
/// Le domaine de travail (« cherche, mais seulement dans ça ») descend par deux
/// routes différentes : des offsets `allowed_ids` sur le chemin lucivy, un
/// `WHERE` compilé sur le chemin vectoriel. Aucune des deux n'a été éprouvée
/// sur postgres, et le chemin texte **natif** n'en reçoit aucune.
///
/// Les trois produits partagent la même description, donc le texte les remonte
/// tous les trois : seul le filtre peut faire la différence. S'il ne descend
/// pas, le test le voit ; s'il descend mal, il échoue bruyamment. C'est
/// exactement ce qu'on veut des deux côtés.
#[test]
fn le_filtre_utilisateur_tient() {
    use rag3weaver::filter::{FilterOp, FilterValue};

    let (_garde, _ctx, mut catalog) = catalogue(8);
    catalog.register_entity("Product", config_produit()).unwrap();

    let commun = "Ownership, lifetimes and concurrency.";
    catalog
        .ingest_entities(
            "Product",
            vec![
                produit("Livre bon marche", commun, 10.0),
                produit("Livre moyen", commun, 30.0),
                produit("Livre cher", commun, 90.0),
            ],
        )
        .unwrap();

    let mut filtres = HashMap::new();
    filtres.insert(
        "price".to_string(),
        FilterValue::Ops(vec![FilterOp::Lt(CypherValue::Float(20.0))]),
    );

    // Les deux chemins sont éprouvés **dans la même passe** : s'arrêter au
    // premier échec cacherait l'état du second, et c'est précisément ce qu'on
    // veut savoir ici.
    let mut manques: Vec<String> = Vec::new();
    for signal in [SearchSignals::BM25, SearchSignals::VECTOR] {
        let reponse = match catalog.search(
            "Product",
            commun,
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(signal),
                filters: filtres.clone(),
                ..Default::default()
            },
        ) {
            Ok(r) => r,
            Err(e) => {
                manques.push(format!("{signal:?} : la recherche filtrée échoue — {e}"));
                continue;
            }
        };

        let noms: Vec<String> = reponse
            .results
            .iter()
            .filter_map(|r| r.data.as_ref()?.get("name")?.as_str().map(|s| s.to_string()))
            .collect();
        eprintln!("{signal:?} sous filtre price<20 : {noms:?}");

        if !noms.contains(&"Livre bon marche".to_string()) {
            manques.push(format!(
                "{signal:?} : le seul produit qui satisfait le filtre manque — {noms:?}"
            ));
        }
        if noms.contains(&"Livre cher".to_string()) || noms.contains(&"Livre moyen".to_string()) {
            manques.push(format!("{signal:?} : le filtre ne descend pas — {noms:?}"));
        }
    }
    assert!(manques.is_empty(), "filtre utilisateur :\n  {}", manques.join("\n  "));
}

// ═══ 10. Le banc : où vit la frontière entre le vrai et le bruit ═════════════

/// **Avant de poser un seuil, le mesurer.**
///
/// Ragkit en a un (0,7) mais calibré sur des **noms de médicaments** — des noms
/// propres courts, où deux chaînes proches désignent presque toujours la même
/// molécule. Notre corpus est fait de descriptions ; rien ne dit que le nombre
/// voyage, et le poser sans regarder serait refaire au jugé ce qu'on reproche
/// aux poids 0,35 / 0,65.
///
/// **Ce que la première version de ce banc a appris, et qui vaut d'être écrit :
/// un banc dont toutes les requêtes reprennent les mots exacts de leur cible ne
/// mesure rien.** Tout y valait 1,0000 — trigramme et Jaro rendent tous deux 1
/// sur une correspondance verbatim — et le bruit lointain rendait zéro résultat,
/// parce que `<%` a un plancher de similarité. Séparation parfaite, information
/// nulle. La frontière ne vit pas là.
///
/// Elle vit dans quatre familles, et ce sont elles qu'on mesure :
///
/// | famille | ce qu'on attend |
/// |---|---|
/// | exacte | les mots de la cible, tels quels |
/// | dégradée | fautes de frappe, mot tronqué — ce qu'un humain tape vraiment |
/// | bruit proche | des mots **du corpus**, mais une combinaison qui ne désigne rien |
/// | bruit lointain | rien en commun avec le corpus |
///
/// C'est le **bruit proche** qui décide : s'il monte au-dessus des requêtes
/// dégradées, aucun seuil ne sépare, et il faut marquer plutôt que filtrer.
#[test]
fn ou_vit_la_frontiere_entre_le_vrai_et_le_bruit() {
    let (_garde, _ctx, mut catalog) = catalogue(8);
    catalog.register_entity("Product", config_produit()).unwrap();

    // Cinq domaines, aucun mot commun entre eux : sans ça une requête d'un
    // domaine répond par accident sur un autre, et la mesure ne vaut rien.
    let corpus: Vec<(&str, &str)> = vec![
        ("Clavecin", "clavecin baroque sautereau registre"),
        ("Xylophone", "xylophone lames palissandre resonateur"),
        ("Crescendo", "crescendo nuance orchestre partition"),
        ("Arpege", "arpege accord egrene doigte"),
        ("Sextant", "sextant navigation astronomique horizon"),
        ("Estuaire", "estuaire maree salinite envasement"),
        ("Tourbillon", "tourbillon courant marin remous"),
        ("Meridien", "meridien longitude greenwich cartographie"),
        ("Obsidienne", "obsidienne verre volcanique conchoidale"),
        ("Basalte", "basalte coulee prismatique refroidissement"),
        ("Gneiss", "gneiss metamorphique foliation migmatite"),
        ("Kaolin", "kaolin argile blanche porcelaine"),
        ("Stipule", "stipule feuille appendice petiole"),
        ("Rhizome", "rhizome souterrain bourgeon vivace"),
        ("Samare", "samare fruit aile dissemination"),
        ("Cambium", "cambium assise generatrice liber"),
        ("Echappement", "echappement ancre balancier spiral"),
        ("Cage", "cage rotative gravite chronometrie"),
        ("Quantieme", "quantieme perpetuel calendrier bissextile"),
        ("Remontoir", "remontoir couronne barillet armage"),
    ];
    catalog
        .ingest_entities(
            "Product",
            corpus.iter().map(|(n, d)| produit(n, d, 10.0)).collect(),
        )
        .unwrap();

    let cherche = |c: &mut Catalog, q: &str| -> (f64, Option<String>) {
        let r = c
            .search(
                "Product",
                q,
                SearchOptions {
                    consistency: Consistency::Immediate,
                    signals: Some(SearchSignals::BM25),
                    limit: 5,
                    ..Default::default()
                },
            )
            .unwrap_or_else(|e| panic!("recherche « {q} » : {e}"));
        let p = r.results.first();
        (
            p.map(|x| x.score).unwrap_or(0.0),
            p.and_then(|x| x.data.as_ref()?.get("name")?.as_str().map(|s| s.to_string())),
        )
    };

    let mesure = |c: &mut Catalog, titre: &str, qs: &[(&str, Option<&str>)]| -> (f64, f64) {
        eprintln!("\n── {titre} ──");
        let (mut mini, mut maxi) = (f64::MAX, 0.0f64);
        for (q, attendu) in qs {
            let (score, nom) = cherche(c, q);
            let verdict = match (attendu, nom.as_deref()) {
                (Some(a), Some(n)) if a == &n => "✓".to_string(),
                (Some(a), n) => format!("✗ (attendu {a}, eu {})", n.unwrap_or("rien")),
                (None, None) => "— rien, et c'est juste".to_string(),
                (None, Some(n)) => format!("⚠ rend {n} alors qu'il n'y a rien à rendre"),
            };
            eprintln!("  {q:<30} {score:.4}  {verdict}");
            mini = mini.min(score);
            maxi = maxi.max(score);
        }
        (mini, maxi)
    };

    // Le plancher de rappel de `pg_trgm`, mesuré aux deux valeurs. Par défaut
    // `word_similarity_threshold` vaut 0,6 : c'est **lui** qui décide ce que la
    // base rapporte, donc lui qui décide ce que Jaro peut ordonner. Un rappel
    // trop serré ne se voit pas comme une erreur — il se voit comme « aucun
    // résultat », ce qui ressemble à « ça n'existe pas ».
    let plancher = |c: &Catalog, v: f64| {
        c.conn()
            .execute(&format!("SET pg_trgm.word_similarity_threshold = {v}"))
            .unwrap_or_else(|e| panic!("plancher {v} : {e}"));
    };

    // Le backend pose lui-même 0,3 au premier appel : pour montrer la falaise,
    // il faut donc **remettre** le défaut de PostgreSQL explicitement, après
    // qu'il ait parlé une fois. C'est ce que fait la recherche à blanc.
    let _ = cherche(&mut catalog, "amorce");
    eprintln!("\n═══ plancher au défaut de PostgreSQL (0,6) ══════════════════");
    plancher(&catalog, 0.6);

    let (exact_min, _) = mesure(&mut catalog, "exactes", &[
        ("sautereau baroque", Some("Clavecin")),
        ("lames palissandre", Some("Xylophone")),
        ("navigation astronomique", Some("Sextant")),
        ("verre volcanique", Some("Obsidienne")),
        ("ancre balancier", Some("Echappement")),
    ]);

    // Ce qu'un humain tape vraiment : une lettre en trop, une qui manque, un
    // mot au singulier, un mot seul.
    let (degrade_min, _) = mesure(&mut catalog, "dégradées (fautes, troncatures)", &[
        ("sauterau baroqe", Some("Clavecin")),
        ("palissandre", Some("Xylophone")),
        ("navigaton astronomiqe", Some("Sextant")),
        ("volcanic", Some("Obsidienne")),
        ("balancier ancres", Some("Echappement")),
        ("metamorphique", Some("Gneiss")),
    ]);

    // **La famille qui décide.** Des mots du corpus, mais une combinaison qui
    // ne désigne aucun produit. C'est la confusion réelle — pas « zzzz ».
    let (_, proche_max) = mesure(&mut catalog, "bruit PROCHE (mots du corpus, sens absent)", &[
        ("palissade baroque", None),
        ("horizon volcanique", None),
        ("couronne metamorphique", None),
        ("feuille prismatique", None),
        ("balancier salinite", None),
    ]);

    let (_, loin_max) = mesure(&mut catalog, "bruit lointain", &[
        ("hydravion supersonique", None),
        ("blockchain consensus byzantin", None),
        ("zzzz qqqq wwww", None),
    ]);

    eprintln!("\n  exactes ≥ {exact_min:.4}   dégradées ≥ {degrade_min:.4}");
    eprintln!("  bruit proche ≤ {proche_max:.4}   bruit lointain ≤ {loin_max:.4}");
    if proche_max < degrade_min {
        eprintln!(
            "  → SÉPARÉS : un seuil vit dans ]{proche_max:.4} ; {degrade_min:.4}[\n"
        );
    } else {
        eprintln!(
            "  → RECOUVREMENT de {:.4} — le pire vrai passe sous le meilleur bruit. \
             Aucun seuil ne sépare : il faut MARQUER, pas filtrer.\n",
            proche_max - degrade_min
        );
    }

    // ── La même mesure, plancher de rappel abaissé ──────────────────────────
    //
    // C'est l'expérience qui compte : le dessin à deux étages veut que la base
    // rapporte **large** et que Jaro tranche. Avec le plancher par défaut, la
    // base ne rapporte pas large — elle ferme la porte avant.
    eprintln!("\n═══ plancher de rappel abaissé à 0,3 ═══════════════════════");
    plancher(&catalog, 0.3);

    let (exact_min_b, _) = mesure(&mut catalog, "exactes", &[
        ("sautereau baroque", Some("Clavecin")),
        ("lames palissandre", Some("Xylophone")),
        ("navigation astronomique", Some("Sextant")),
        ("verre volcanique", Some("Obsidienne")),
        ("ancre balancier", Some("Echappement")),
    ]);
    let (degrade_min_b, _) = mesure(&mut catalog, "dégradées (fautes, troncatures)", &[
        ("sauterau baroqe", Some("Clavecin")),
        ("palissandre", Some("Xylophone")),
        ("navigaton astronomiqe", Some("Sextant")),
        ("volcanic", Some("Obsidienne")),
        ("balancier ancres", Some("Echappement")),
        ("metamorphique", Some("Gneiss")),
    ]);
    let (_, proche_max_b) = mesure(&mut catalog, "bruit PROCHE", &[
        ("palissade baroque", None),
        ("horizon volcanique", None),
        ("couronne metamorphique", None),
        ("feuille prismatique", None),
        ("balancier salinite", None),
    ]);
    let (_, loin_max_b) = mesure(&mut catalog, "bruit lointain", &[
        ("hydravion supersonique", None),
        ("blockchain consensus byzantin", None),
        ("zzzz qqqq wwww", None),
    ]);

    eprintln!("\n  exactes ≥ {exact_min_b:.4}   dégradées ≥ {degrade_min_b:.4}");
    eprintln!("  bruit proche ≤ {proche_max_b:.4}   bruit lointain ≤ {loin_max_b:.4}");
    if proche_max_b < degrade_min_b {
        eprintln!("  → SÉPARÉS : un seuil vit dans ]{proche_max_b:.4} ; {degrade_min_b:.4}[\n");
    } else {
        eprintln!(
            "  → RECOUVREMENT de {:.4} : marquer, pas filtrer\n",
            proche_max_b - degrade_min_b
        );
    }

    // ── Ce que le banc affirme ──────────────────────────────────────────────
    assert!(exact_min > 0.0, "une requête exacte doit rendre quelque chose");
    assert!(exact_min_b > 0.0, "abaisser le plancher ne doit rien casser");

    // La falaise : au plancher de PostgreSQL, une requête à deux fautes ne
    // rapporte **rien**. C'est ce qui justifie `PLANCHER_RAPPEL`.
    assert_eq!(
        degrade_min, 0.0,
        "au plancher 0,6, « sauterau baroqe » devait ne rien rapporter — si ce \
         n'est plus le cas, `PLANCHER_RAPPEL` n'a plus de raison d'être"
    );

    // Et la frontière : une fois le rappel ouvert, le bruit proche reste sous
    // les requêtes dégradées. C'est ce qui rend un seuil possible.
    assert!(
        proche_max_b < degrade_min_b,
        "le bruit proche ({proche_max_b:.4}) doit rester sous la pire vraie \
         requête ({degrade_min_b:.4}) — sinon aucun seuil ne sépare et il faut \
         revoir `SEUIL_CONFIANCE`"
    );
    // **Ce banc mesure le classement, pas la confiance.** Les scores rendus ici
    // sont ceux de la fusion (0,20 trigramme / 0,80 Jaro) : relatifs, et d'une
    // autre nature sur lucivy. Le seuil, lui, vit sur du Jaro pur et se vérifie
    // dans `les_poids_du_combo_se_mesurent`. Ce qui compte ici est qu'un
    // intervalle **existe** — sans lui, aucun seuil n'aurait de sens.

    // **Et le marquage se dit.** Une coïncidence lexicale doit s'annoncer comme
    // telle, sinon elle se présente exactement comme une réponse.
    let douteux = catalog
        .search(
            "Product",
            "couronne metamorphique",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        .expect("recherche douteuse");
    eprintln!("  avertissements : {:?}", douteux.meta.warnings);
    assert!(
        douteux.meta.warnings.iter().any(|w| w.contains("rien de probant")),
        "une réponse sous le seuil doit se marquer — avertissements : {:?}",
        douteux.meta.warnings
    );

    // Et une vraie réponse ne se fait pas marquer pour rien.
    let net = catalog
        .search(
            "Product",
            "sautereau baroque",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        .expect("recherche nette");
    assert!(
        !net.meta.warnings.iter().any(|w| w.contains("rien de probant")),
        "une requête exacte ne doit pas être marquée : {:?}",
        net.meta.warnings
    );
}

// ═══ 11. Les poids du combo, mesurés au lieu d'être supposés ═════════════════

/// **0,35 trigramme / 0,65 Jaro étaient posés au jugé.** Ce banc les mesure.
///
/// Il n'a besoin d'aucun réglage en production : il demande au backend ses
/// candidats **bruts** (`text_search` rend le score trigramme *et* le texte),
/// calcule Jaro lui-même, et balaie les combinaisons en Rust. C'est la
/// **formule** qu'on mesure, pas la tuyauterie — donc rien à exposer, rien à
/// laisser derrière.
///
/// Le critère est celui du banc précédent : la **marge** entre la pire requête
/// dégradée (qui doit passer) et le meilleur bruit proche (qui ne doit pas).
/// Une marge large veut dire qu'un seuil y tient sans trembler.
#[test]
fn les_poids_du_combo_se_mesurent() {
    let (_garde, _ctx, mut catalog) = catalogue(8);
    catalog.register_entity("Product", config_produit()).unwrap();

    let corpus: Vec<(&str, &str)> = vec![
        ("Clavecin", "clavecin baroque sautereau registre"),
        ("Xylophone", "xylophone lames palissandre resonateur"),
        ("Crescendo", "crescendo nuance orchestre partition"),
        ("Arpege", "arpege accord egrene doigte"),
        ("Sextant", "sextant navigation astronomique horizon"),
        ("Estuaire", "estuaire maree salinite envasement"),
        ("Tourbillon", "tourbillon courant marin remous"),
        ("Meridien", "meridien longitude greenwich cartographie"),
        ("Obsidienne", "obsidienne verre volcanique conchoidale"),
        ("Basalte", "basalte coulee prismatique refroidissement"),
        ("Gneiss", "gneiss metamorphique foliation migmatite"),
        ("Kaolin", "kaolin argile blanche porcelaine"),
        ("Stipule", "stipule feuille appendice petiole"),
        ("Rhizome", "rhizome souterrain bourgeon vivace"),
        ("Samare", "samare fruit aile dissemination"),
        ("Cambium", "cambium assise generatrice liber"),
        ("Echappement", "echappement ancre balancier spiral"),
        ("Cage", "cage rotative gravite chronometrie"),
        ("Quantieme", "quantieme perpetuel calendrier bissextile"),
        ("Remontoir", "remontoir couronne barillet armage"),
    ];
    catalog
        .ingest_entities(
            "Product",
            corpus.iter().map(|(n, d)| produit(n, d, 10.0)).collect(),
        )
        .unwrap();

    let backend = catalog.search_backend().expect("le backend postgres");

    // Le meilleur score d'une requête, pour un partage de poids donné.
    let meilleur = |q: &str, poids_trgm: f64| -> f64 {
        let hits = backend
            .text_search("Product_Chunk", &["_text".to_string()], q, 50, None, None, None, &[])
            .expect("le backend sert le plein texte")
            .unwrap_or_else(|e| panic!("text_search « {q} » : {e}"));
        hits.iter()
            .map(|h| {
                let fin = rag3weaver::jaro::meilleur_par_mot(q, &h.texte);
                poids_trgm * h.score + (1.0 - poids_trgm) * fin
            })
            .fold(0.0f64, f64::max)
    };

    let degradees = [
        "sauterau baroqe",
        "palissandre",
        "navigaton astronomiqe",
        "volcanic",
        "balancier ancres",
        "metamorphique",
    ];
    let bruit_proche = [
        "palissade baroque",
        "horizon volcanique",
        "couronne metamorphique",
        "feuille prismatique",
        "balancier salinite",
    ];

    eprintln!("\n  poids trgm/jaro   pire vraie   meilleur bruit   marge");
    let mut meilleure_marge = (f64::MIN, 0.0f64);
    for pas in 0..=10 {
        let pt = pas as f64 / 10.0;
        let pire_vraie = degradees
            .iter()
            .map(|q| meilleur(q, pt))
            .fold(f64::MAX, f64::min);
        let pire_bruit = bruit_proche
            .iter()
            .map(|q| meilleur(q, pt))
            .fold(0.0f64, f64::max);
        let marge = pire_vraie - pire_bruit;
        let marque = if pt == 0.35 { "  ← aujourd'hui" } else { "" };
        eprintln!(
            "  {:.2} / {:.2}        {pire_vraie:.4}       {pire_bruit:.4}      {marge:+.4}{marque}",
            pt,
            1.0 - pt
        );
        if marge > meilleure_marge.0 {
            meilleure_marge = (marge, pt);
        }
    }
    eprintln!(
        "\n  marge maximale {:+.4} à {:.2} trigramme / {:.2} Jaro\n",
        meilleure_marge.0,
        meilleure_marge.1,
        1.0 - meilleure_marge.1
    );

    // Ce que le banc affirme : il existe un partage qui sépare. Le nombre se
    // lit dans la sortie et se fixe dans `search.rs` — pas ici, sinon le banc
    // deviendrait le juge de sa propre réponse.
    assert!(
        meilleure_marge.0 > 0.0,
        "aucun partage de poids ne sépare le vrai du bruit proche"
    );

    // **Et le seuil de confiance, qui vit sur Jaro pur.** Le score de classement
    // et le score de confiance ne sont pas le même nombre : le premier est
    // relatif et change de nature selon le backend (BM25 non borné sur lucivy),
    // le second doit être absolu et comparable partout. C'est la ligne
    // 0,00 / 1,00 du tableau ci-dessus qui le fixe.
    let jaro_vraie = degradees.iter().map(|q| meilleur(q, 0.0)).fold(f64::MAX, f64::min);
    let jaro_bruit = bruit_proche.iter().map(|q| meilleur(q, 0.0)).fold(0.0f64, f64::max);
    let seuil = rag3weaver::search::SEUIL_CONFIANCE;
    eprintln!(
        "  en Jaro pur : pire vraie {jaro_vraie:.4}, meilleur bruit {jaro_bruit:.4} \
         — seuil posé à {seuil:.2}\n"
    );
    assert!(
        jaro_bruit < seuil && seuil < jaro_vraie,
        "SEUIL_CONFIANCE ({seuil:.2}) doit partager ]{jaro_bruit:.4} ; {jaro_vraie:.4}[ \
         — s'il est déplacé sans remesurer, c'est ici que ça se dit"
    );
}

// ═══ 12. Les trois moteurs de texte marchent, chacun à sa sauce ══════════════

/// **Le chemin du retour vers lucivy, prouvé plutôt que supposé.**
///
/// `MoteurTexte::{Auto, Lucivy, Natif}` est une **option**, pas un
/// remplacement : le trigramme est le défaut aujourd'hui parce que lucivy pèse
/// trop sur disque, et lucivy redeviendra le défaut quand elle sera allégée.
/// Encore faut-il que ce retour marche.
///
/// Or `set_moteur_texte` n'avait **aucun appelant** : les trois valeurs
/// existaient sur le papier. Une option jamais empruntée est une option qui se
/// dégrade sans bruit — c'est le défaut qu'on a passé la journée à débusquer,
/// appliqué à une garantie de compatibilité.
///
/// Ce test emprunte les deux chemins forcés sur **la même base**, avec le même
/// corpus, et vérifie qu'ils trouvent tous deux — pas qu'ils trouvent la même
/// chose : chacun classe à sa sauce, c'est le but.
#[test]
fn les_trois_moteurs_de_texte_marchent() {
    use rag3weaver::search_backend::MoteurTexte;

    let produits = || {
        vec![
            produit("Clavecin", "clavecin baroque sautereau registre", 10.0),
            produit("Sextant", "sextant navigation astronomique horizon", 20.0),
            produit("Obsidienne", "obsidienne verre volcanique conchoidale", 30.0),
        ]
    };

    // ── Le trigramme, forcé ─────────────────────────────────────────────────
    let noms_natif = {
        let (_garde, _ctx, mut catalog) = catalogue_avec(8, MoteurTexte::Natif);
        assert!(catalog.plein_texte_natif(), "Natif doit forcer le backend");
        catalog.register_entity("Product", config_produit()).unwrap();
        catalog.ingest_entities("Product", produits()).unwrap();
        let r = catalog
            .search(
                "Product",
                "navigation astronomique",
                SearchOptions {
                    consistency: Consistency::Immediate,
                    signals: Some(SearchSignals::BM25),
                    ..Default::default()
                },
            )
            .expect("recherche par le trigramme");
        r.results
            .iter()
            .filter_map(|x| x.data.as_ref()?.get("name")?.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>()
    };
    eprintln!("[natif]  {noms_natif:?}");
    assert!(
        noms_natif.contains(&"Sextant".to_string()),
        "le trigramme doit trouver : {noms_natif:?}"
    );

    // ── lucivy, forcée, sur le même backend PostgreSQL ──────────────────────
    //
    // C'est le cas qui n'avait jamais tourné : l'index lucivy s'écrit dans le
    // `PostgresBlobStore`, et la recherche passe par `search_bm25_chunked` avec
    // ses handles — deux organes qui ne se rencontraient nulle part.
    let noms_lucivy = {
        let (_garde, _ctx, mut catalog) = catalogue_avec(8, MoteurTexte::Lucivy);
        assert!(
            !catalog.plein_texte_natif(),
            "Lucivy doit forcer lucivy même quand le backend sait faire"
        );
        catalog.register_entity("Product", config_produit()).unwrap();
        catalog.ingest_entities("Product", produits()).unwrap();
        let r = catalog
            .search(
                "Product",
                "navigation astronomique",
                SearchOptions {
                    consistency: Consistency::Immediate,
                    signals: Some(SearchSignals::BM25),
                    ..Default::default()
                },
            )
            .expect("recherche par lucivy sur postgres");
        eprintln!("[lucivy] avertissements : {:?}", r.meta.warnings);
        r.results
            .iter()
            .filter_map(|x| x.data.as_ref()?.get("name")?.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>()
    };
    eprintln!("[lucivy] {noms_lucivy:?}");
    assert!(
        noms_lucivy.contains(&"Sextant".to_string()),
        "lucivy doit trouver sur postgres — c'est le chemin du retour : {noms_lucivy:?}"
    );
}

// ═══ 13. La reprise après incident, sur PostgreSQL ═══════════════════════════

/// **Ce qui manquait pour que PostgreSQL soit un backend et pas une
/// démonstration.**
///
/// `initialize()` disait déjà ce qui manquait — « aucun magasin de checkpoints :
/// le seul disponible parle Cypher […] la reprise après incident est
/// indisponible » — et continuait. Une ingestion morte en route ne pouvait pas
/// reprendre. Sur kuzu si, sur PostgreSQL non.
///
/// La suite de conformité est **la même** que celle jouée contre le magasin
/// Cypher : deux implémentations d'un même trait tenues à la main divergent, et
/// la garde n'est pas la bonne volonté.
#[test]
fn la_reprise_apres_incident_tient_sur_postgres() {
    let (_garde, ctx, _catalog) = catalogue(8);
    let conn: Arc<dyn DbConnection> = Arc::new(
        ctx.rt
            .block_on(PostgresConnection::new(&conn_str()))
            .expect("connexion pour le magasin de checkpoints"),
    );
    let store = rag3weaver::dataflow::checkpoint_store::PostgresCheckpointStore::new(conn);
    rag3weaver::dataflow::checkpoint_store::verifier_conformite(&store, "postgres")
        .unwrap_or_else(|e| panic!("{e}"));
}

/// Et le catalogue le **monte tout seul** : une pièce écrite mais jamais
/// appelée est une pièce qui se dégrade sans bruit — c'est le défaut qu'on a
/// passé la journée à débusquer.
#[test]
fn le_catalogue_monte_le_magasin_de_checkpoints() {
    let (_garde, _ctx, mut catalog) = catalogue(8);
    assert!(
        catalog.has_checkpoint_store(),
        "PostgreSQL doit avoir sa reprise après incident sans qu'on la lui pose à la main"
    );

    // **Et il s'en sert.** Un magasin monté mais jamais écrit ne vaut pas mieux
    // qu'un magasin absent : c'est le défaut de la journée. Une ingestion
    // réelle doit laisser sa trace.
    catalog.register_entity("Product", config_produit()).unwrap();
    catalog
        .ingest_entities("Product", vec![produit("Clavecin", "clavecin baroque", 10.0)])
        .unwrap();

    let n = catalog
        .conn()
        .execute("SELECT count(*) FROM rag3weaver._dataflow_execution")
        .expect("la table des exécutions doit exister")
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    assert!(
        n > 0,
        "une ingestion doit laisser un checkpoint — sinon la reprise n'a rien à reprendre"
    );

    // Et rien d'inachevé ne traîne après une ingestion réussie.
    let inachevees = catalog
        .conn()
        .execute(
            "SELECT count(*) FROM rag3weaver._dataflow_execution \
             WHERE status IN ('running', 'failed')",
        )
        .expect("compte des inachevées")
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .unwrap_or(-1);
    assert_eq!(
        inachevees, 0,
        "une ingestion réussie ne doit laisser aucune exécution en cours ni échouée"
    );
}

// ═══ 14. La marque d'eau : Strict traverse la frontière du processus ═════════

/// **Ce que `Consistency::Strict` promettait sans pouvoir le tenir.**
///
/// `Strict` veut dire « vide la file avant de chercher ». La file vit dans
/// `Catalog::pending`, **en mémoire** : un lecteur d'un autre processus a son
/// propre catalogue, dont la file est vide. Il demandait `Strict` et obtenait
/// `Immediate`, sans que rien ne le dise.
///
/// Le verrou de fichier n'a jamais protégé de ça — il rendait l'accès
/// concurrent *impossible*, pas *ordonné*. Ça devient visible maintenant qu'on
/// peut le franchir.
///
/// Deux catalogues sur la même base tiennent lieu de deux processus : ils ont
/// des files séparées, ce qui est exactement la propriété en cause.
#[test]
fn la_marque_deau_traverse_la_frontiere() {
    let (_garde, ctx, mut ecrivain) = catalogue(8);
    ecrivain.register_entity("Product", config_produit()).unwrap();

    // Le lecteur : un second catalogue, sa propre file — donc vide.
    let boxed: Box<dyn DbConnection> = Box::new(
        ctx.rt
            .block_on(PostgresConnection::new(&conn_str()))
            .expect("connexion du lecteur"),
    );
    let mut lecteur = Catalog::new(boxed, Box::new(HashEmbedder::new(8)), config_vide(8));
    let partagee = lecteur.conn_arc();
    lecteur.set_dialect(Arc::new(PostgresDialect));
    lecteur.set_search_backend(Arc::new(PostgresSearchBackend::new(partagee.clone())));
    lecteur.set_blob_store(Arc::new(
        rag3weaver::postgres_blob_store::PostgresBlobStore::new(partagee),
    ));
    lecteur.initialize().expect("initialize du lecteur");
    lecteur.register_entity("Product", config_produit()).unwrap();

    // 1. Rien en attente : `Strict` aboutit sans un mot.
    let mut avertissements: Vec<String> = Vec::new();
    assert!(
        lecteur.attendre_les_ecritures(1_000, &mut avertissements),
        "base au repos : l'attente doit aboutir — {avertissements:?}"
    );
    assert!(avertissements.is_empty(), "rien à signaler : {avertissements:?}");

    // 2. L'écrivain met en file **sans vider**. C'est le cas qui mentait :
    //    le lecteur n'a aucun moyen de le savoir depuis sa propre file.
    ecrivain
        .create("Product", produit("Clavecin", "clavecin baroque sautereau", 10.0))
        .expect("mise en file");
    assert!(ecrivain.has_pending(), "le montage suppose une file non vidée");

    let mut avertissements: Vec<String> = Vec::new();
    let abouti = lecteur.attendre_les_ecritures(200, &mut avertissements);
    eprintln!("[marque] attente sous travail : abouti={abouti}, {avertissements:?}");
    assert!(
        !abouti,
        "un écrivain a du travail non publié : l'attente ne doit PAS aboutir"
    );
    assert!(
        avertissements.iter().any(|w| w.contains("cohérence stricte non tenue")),
        "et elle doit le dire — {avertissements:?}"
    );

    // 3. L'écrivain vide : la marque s'efface, et l'attente aboutit de nouveau.
    ecrivain.drain();
    assert!(!ecrivain.has_pending());
    let mut avertissements: Vec<String> = Vec::new();
    assert!(
        lecteur.attendre_les_ecritures(1_000, &mut avertissements),
        "après le drain, l'attente doit aboutir — {avertissements:?}"
    );
    assert!(avertissements.is_empty(), "et sans rien signaler : {avertissements:?}");
}

/// Et la promesse remonte **jusqu'à l'appelant** : une recherche `Strict` faite
/// pendant qu'un autre écrivain a du travail non publié le dit dans sa méta.
///
/// Sans cette ligne, la garantie se dégraderait exactement comme avant — en
/// silence.
#[test]
fn une_recherche_stricte_dit_ce_quelle_ne_peut_pas_tenir() {
    let (_garde, ctx, mut ecrivain) = catalogue(8);
    ecrivain.register_entity("Product", config_produit()).unwrap();
    ecrivain
        .ingest_entities("Product", vec![produit("Sextant", "sextant navigation", 10.0)])
        .unwrap();

    let boxed: Box<dyn DbConnection> = Box::new(
        ctx.rt
            .block_on(PostgresConnection::new(&conn_str()))
            .expect("connexion du lecteur"),
    );
    let mut lecteur = Catalog::new(boxed, Box::new(HashEmbedder::new(8)), config_vide(8));
    let partagee = lecteur.conn_arc();
    lecteur.set_dialect(Arc::new(PostgresDialect));
    lecteur.set_search_backend(Arc::new(PostgresSearchBackend::new(partagee.clone())));
    lecteur.set_blob_store(Arc::new(
        rag3weaver::postgres_blob_store::PostgresBlobStore::new(partagee),
    ));
    lecteur.initialize().expect("initialize du lecteur");
    lecteur.register_entity("Product", config_produit()).unwrap();

    // L'écrivain met en file sans vider.
    ecrivain
        .create("Product", produit("Clavecin", "clavecin baroque", 20.0))
        .expect("mise en file");

    let reponse = lecteur
        .search(
            "Product",
            "navigation",
            SearchOptions {
                consistency: Consistency::Strict,
                signals: Some(SearchSignals::BM25),
                timeout_ms: 200,
                ..Default::default()
            },
        )
        .expect("recherche stricte");
    eprintln!("[marque] avertissements de la recherche : {:?}", reponse.meta.warnings);
    assert!(
        reponse
            .meta
            .warnings
            .iter()
            .any(|w| w.contains("cohérence stricte non tenue")),
        "une recherche Strict qui ne peut pas l'être doit le dire — {:?}",
        reponse.meta.warnings
    );
}

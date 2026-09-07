//! E2E integration tests: Simple Entity pipeline (register → ingest → search).
//!
//! Tests the full pipeline WITHOUT knowledge bases: register_entity, ingest_entities,
//! and search() with real FTS/vector/sparse indexes.
//!
//! Run with: ./run_e2e.sh simple_entity

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
#[cfg(feature = "burn-embedder")]
use std::sync::Arc;

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
#[cfg(feature = "burn-embedder")]
use rag3weaver::embedder::Embedder;
use rag3weaver::search::{Consistency, ResultMode, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, CatalogEvent, EntityConfig, Rag3dbConnection, SimpleFieldDef, UpdateStatus};

mod common;

#[cfg(feature = "burn-embedder")]
use common::burn::{BGE_M3, MINILM};
#[cfg(feature = "burn-embedder")]
use rag3weaver::embedder::SparseEmbedder;
use rag3weaver::disponibilite::RegimeEcriture;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Root path of the rag3db source tree (two levels up from extension/rag3weaver/).
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

/// Load required extensions (vector, lucivy_fts, sparse_vector) into a native connection.
fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let root = rag3db_root();
    let extensions = [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
    ];
    for (name, ext_path) in &extensions {
        if !std::path::Path::new(ext_path).exists() {
            panic!(
                "Extension '{name}' not found at: {ext_path}\n\
                 Run ./run_e2e.sh --build-only first."
            );
        }
        let result = conn.execute(&format!("LOAD EXTENSION '{ext_path}'"));
        match result {
            Ok(_) => eprintln!("✓ Loaded {name}"),
            Err(e) => panic!("Failed to load {name} from {ext_path}: {e}"),
        }
    }
}

/// Minimal CatalogConfig with no entities and no KBs.
/// Entities will be registered dynamically via register_entity().
fn make_empty_config(dim: usize) -> CatalogConfig {
    CatalogConfig {
        name: Some("simple-entity-test".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: dim,
        ..Default::default()
    }
}

/// EntityConfig for a "Product" entity with title, description, details fields.
fn make_product_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert(
        "name".into(),
        SimpleFieldDef {
            field_type: FieldType::String,
            is_title: true,
            is_content: false,
            ..Default::default()
        },
    );
    fields.insert(
        "description".into(),
        SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        },
    );
    fields.insert(
        "details".into(),
        SimpleFieldDef {
            field_type: FieldType::Text,
            is_title: false,
            is_content: true,
            ..Default::default()
        },
    );
    fields.insert(
        "price".into(),
        SimpleFieldDef {
            field_type: FieldType::Double,
            is_title: false,
            is_content: false,
            ..Default::default()
        },
    );

    EntityConfig {
        fields,
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }
}

fn make_product(
    name: &str,
    description: &str,
    details: &str,
    price: f64,
) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("description".into(), CypherValue::String(description.into()));
    data.insert("details".into(), CypherValue::String(details.into()));
    data.insert("price".into(), CypherValue::Float(price));
    data
}

/// Create catalog, load extensions, initialize, register Product entity.
fn setup_simple_catalog(embedder_dim: usize) -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(embedder_dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(embedder_dim)), config);
    catalog.initialize().unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.regime_d_ecriture(RegimeEcriture::ParLot);
    catalog
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 1 — Register + Ingest basics
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn simple_register_and_ingest() {
    let mut catalog = setup_simple_catalog(4);

    // Subscribe to events for debug
    let mut rx = catalog.subscribe();

    let products = vec![
        make_product(
            "Rust Book",
            "A comprehensive guide to Rust programming language covering ownership, lifetimes, and concurrency.",
            "Covers systems programming, memory safety, and zero-cost abstractions.",
            49.99,
        ),
        make_product(
            "Python Cookbook",
            "Recipes for mastering Python with focus on data science, web development, and automation.",
            "Includes pandas, numpy, flask, and asyncio examples.",
            39.99,
        ),
        make_product(
            "French Chef Knife",
            "Professional kitchen knife forged from high-carbon stainless steel.",
            "Perfect for slicing, dicing, and mincing. Used in French cuisine worldwide.",
            129.99,
        ),
    ];

    let result = catalog.ingest_entities("Product", products).unwrap();
    eprintln!("ingest: processed={}, failed={}", result.processed, result.failed);

    // Drain events
    while let Ok(event) = rx.try_recv() {
        match &event {
            CatalogEvent::Error { context, message } => {
                eprintln!("  [EVENT ERROR] {context}: {message}");
            }
            _ => eprintln!("  [EVENT] {event:?}"),
        }
    }

    assert!(result.processed >= 3, "at least 3 inserts: {}", result.processed);
    assert_eq!(result.failed, 0);

    // Verify entity count
    let count = catalog
        .execute_raw("MATCH (p:Product) RETURN count(p) AS cnt")
        
        .unwrap();
    let cnt = count.rows[0][0].as_i64().unwrap();
    assert_eq!(cnt, 3, "should have 3 products");

    // Verify chunks created
    let chunks = catalog
        .execute_raw("MATCH (c:Product_Chunk) RETURN count(c) AS cnt")
        
        .unwrap();
    let chunk_cnt = chunks.rows[0][0].as_i64().unwrap();
    assert!(chunk_cnt >= 3, "should have at least 3 chunks: {chunk_cnt}");
    eprintln!("✓ {cnt} products, {chunk_cnt} chunks");

    // Debug: show rel tables
    let tables = catalog
        .execute_raw("CALL show_tables() RETURN *")
        
        .unwrap();
    eprintln!("--- Tables ---");
    for row in &tables.rows {
        eprintln!("  {:?}", row);
    }

    // Debug: try both directions for CHUNKED_FROM
    let fwd = catalog
        .execute_raw("MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]->(p:Product) RETURN count(c) AS cnt")
        
        .unwrap();
    let fwd_cnt = fwd.rows[0][0].as_i64().unwrap();
    eprintln!("CHUNKED_FROM fwd (chunk→product): {fwd_cnt}");

    let rev = catalog
        .execute_raw("MATCH (p:Product)-[:Product_CHUNKED_FROM]->(c:Product_Chunk) RETURN count(c) AS cnt")
        
        .unwrap();
    let rev_cnt = rev.rows[0][0].as_i64().unwrap();
    eprintln!("CHUNKED_FROM rev (product→chunk): {rev_cnt}");

    // Undirected
    let undir = catalog
        .execute_raw("MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]-(p:Product) RETURN count(c) AS cnt")
        
        .unwrap();
    let undir_cnt = undir.rows[0][0].as_i64().unwrap();
    eprintln!("CHUNKED_FROM undirected: {undir_cnt}");

    let rel_cnt = std::cmp::max(fwd_cnt, rev_cnt);
    assert_eq!(rel_cnt, chunk_cnt, "every chunk should have a CHUNKED_FROM relation");
    eprintln!("✓ {rel_cnt} CHUNKED_FROM relations");
}

#[test]
#[ignore]
fn simple_ingest_unknown_entity_fails() {
    let mut catalog = setup_simple_catalog(4);
    let result = catalog.ingest_entities("Unknown", vec![]);
    assert!(result.is_err(), "should fail for unknown entity");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 2 — BM25 search (FTS only, no embeddings needed)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn simple_bm25_search_finds_results() {
    let mut catalog = setup_simple_catalog(4);

    let products = vec![
        make_product(
            "Rust Book",
            "A comprehensive guide to Rust programming language covering ownership, lifetimes, and concurrency.",
            "Covers systems programming, memory safety, and zero-cost abstractions.",
            49.99,
        ),
        make_product(
            "Python Cookbook",
            "Recipes for mastering Python with focus on data science, web development, and automation.",
            "Includes pandas, numpy, flask, and asyncio examples.",
            39.99,
        ),
        make_product(
            "French Chef Knife",
            "Professional kitchen knife forged from high-carbon stainless steel.",
            "Perfect for slicing, dicing, and mincing. Used in French cuisine worldwide.",
            129.99,
        ),
    ];

    catalog.ingest_entities("Product", products).unwrap();

    let response = catalog
        .search(
            "Product",
            "programming language",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!(
        "BM25 search: {} results, bm25_count={}",
        response.results.len(),
        response.meta.bm25_count
    );
    assert!(!response.results.is_empty(), "should find results for 'programming language'");
    assert!(response.meta.bm25_count > 0, "bm25_count should be > 0");
    assert_eq!(response.meta.target, "Product", "meta.target should be 'Product'");

    // Top result should be a programming-related product
    let top = &response.results[0];
    eprintln!("Top result: score={}, entity={:?}", top.score, top.entity);
    if let Some(data) = &top.data {
        eprintln!("  data keys: {:?}", data.keys().collect::<Vec<_>>());
    }
}

/// **Le chemin par défaut de la lecture indexe-t-il ce qu'il pose ?**
///
/// Toutes les recherches de ce fichier passaient par `ingest_entities` et
/// `Consistency::Immediate`. Personne n'empruntait la combinaison qui est
/// pourtant le défaut du produit : `create()` — qui met en file — suivi d'une
/// recherche en `Eventual`, laquelle appelle `flush_insertions` pour poser les
/// entités avant de chercher.
///
/// Ce chemin-là ne appelait pas `open_fts_handles_for` et n'enregistrait pas
/// `fts_handles` : les entités étaient **consommées** — `mem::take`, elles ne
/// repassent pas au drain — et jamais indexées en plein texte. Une recherche
/// rendait zéro, sans erreur, pour des lignes bien présentes en base.
///
/// Le test tient les deux bouts : la ligne existe, **et** on la trouve.
#[test]
#[ignore]
fn une_recherche_eventual_indexe_ce_qu_elle_pose() {
    let mut catalog = setup_simple_catalog(4);

    for produit in [
        make_product(
            "Rust Book",
            "A comprehensive guide to Rust programming language covering ownership.",
            "Systems programming, memory safety, zero-cost abstractions.",
            49.99,
        ),
        make_product(
            "French Chef Knife",
            "Professional kitchen knife forged from high-carbon stainless steel.",
            "Perfect for slicing and dicing.",
            129.99,
        ),
    ] {
        catalog.create("Product", produit).expect("mise en file");
    }

    // `Eventual` est le défaut, et c'est lui qui appelle `flush_insertions`.
    let reponse = catalog
        .search(
            "Product",
            "programming language",
            SearchOptions {
                consistency: Consistency::Eventual,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        .expect("la recherche ne doit pas échouer");

    eprintln!(
        "[eventual] {} résultats, bm25={}, partiel={}, en file={}",
        reponse.results.len(),
        reponse.meta.bm25_count,
        reponse.meta.partial,
        reponse.meta.pending_count,
    );
    for a in &reponse.meta.warnings {
        eprintln!("[eventual] avertissement : {a}");
    }

    // **L'assertion qui compte.** Avant le 6 septembre 2026, ce compte était
    // zéro : `flush_insertions` posait les lignes sans ouvrir de handle FTS ni
    // enregistrer `fts_handles`, et les entités — consommées par `mem::take` —
    // ne repassaient jamais au drain. La recherche rendait « rien trouvé » pour
    // une ligne bien présente en base, sans une erreur.
    assert!(
        !reponse.results.is_empty(),
        "une entité posée par flush_insertions doit être trouvable en plein texte \
         dans la foulée — zéro résultat ici veut dire que l'indexation a été sautée \
         en silence"
    );
    assert!(reponse.meta.bm25_count > 0, "le signal plein texte doit avoir répondu");

    // Et la ligne est bien en base : les deux bouts, pas seulement l'index.
    let compte = catalog.count("Product").expect("compte");
    assert_eq!(compte, 2, "les deux produits sont posés");

    // L'invariant de la méta, quel que soit ce qui reste. Ici `Product` ne
    // participe à aucune base de connaissances : `create()` n'enfile qu'un
    // enregistrement d'entité, sans relation ni agrégat, donc `flush_insertions`
    // vide tout et il ne reste rien. C'est ce que la première version de ce test
    // avait supposé faux — elle attendait « partiel » en recopiant un cas de
    // bibliothèque qui, lui, a une KB.
    assert_eq!(
        reponse.meta.partial,
        reponse.meta.pending_count > 0,
        "« partiel » dit exactement s'il reste du travail, ni plus ni moins"
    );

    // Le drain d'après ne doit rien changer à ce qu'on trouve.
    catalog.drain();
    let apres = catalog
        .search(
            "Product",
            "programming language",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        .expect("la recherche ne doit pas échouer");
    assert_eq!(
        apres.results.len(),
        reponse.results.len(),
        "le drain ne réindexe pas ce qui l'était déjà, et n'en perd pas"
    );
}

/// **Le verbe de lot qui rend moins, et qui le dit.**
///
/// Le cas de l'écrivain qu'il faut chaperonner : il ingère sans arrêt, et on ne
/// veut pas une passe GPU par item. `ingest_entities_jusqu_a(…, RECHERCHE_TEXTE)`
/// pose les lignes, découpe, indexe en plein texte — et laisse l'embarquement
/// dû dans la base.
///
/// Ce qui distingue ça d'un mensonge : le `FlushResult` **porte sa portée**.
/// L'appelant sait qu'il a `data + textsearch` et pas `dense`, au lieu de
/// croire qu'il a tout.
#[test]
#[ignore]
fn le_lot_peut_rendre_moins_a_condition_de_le_dire() {
    use rag3weaver::disponibilite::Disponibilites as D;

    let mut catalog = setup_simple_catalog(4);
    let res = catalog
        .ingest_entities_jusqu_a(
            "Product",
            vec![make_product(
                "Rust Book",
                "A comprehensive guide to Rust programming language covering ownership.",
                "Systems programming, memory safety.",
                49.99,
            )],
            D::RECHERCHE_TEXTE,
        )
        .expect("ingestion partielle");

    // **L'acquittement dit sa portée.** C'est ce qui rend l'omission honnête.
    assert_eq!(
        res.rendu_pret,
        Some(D::RECHERCHE_TEXTE),
        "un verbe qui rend moins doit dire quoi, sinon c'est le mensonge d'hier"
    );

    // Le plein texte trouve, sans qu'aucun GPU ait tourné.
    let bm25 = catalog
        .search("Product", "programming language", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        .expect("recherche");
    assert!(!bm25.results.is_empty(), "le plein texte est prêt");

    // Le dense est dû, et le zéro vectoriel le dit.
    let dense = catalog
        .search("Product", "programming language", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::VECTOR),
            ..Default::default()
        })
        .expect("recherche");
    eprintln!("[lot] avertissements : {:?}", dense.meta.warnings);
    assert!(
        dense.meta.warnings.iter().any(|a| a.contains("pas encore été embarqués")),
        "la dette laissée par le lot doit se dire : {:?}", dense.meta.warnings
    );

    // Et le rattrapage la solde — c'est le tick, appelé à la demande.
    let repris = catalog.embarquer_le_retard(D::TOUT, 512, None).expect("rattrapage");
    assert!(repris > 0, "le rattrapage doit retrouver la dette dans la base");

    let dense = catalog
        .search("Product", "programming language", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::VECTOR),
            ..Default::default()
        })
        .expect("recherche");
    assert!(
        dense.meta.warnings.iter().all(|a| !a.contains("pas encore été embarqués")),
        "plus rien de dû : {:?}", dense.meta.warnings
    );

    // Et le verbe complet, lui, n'a rien changé : il rend toujours tout.
    let res = catalog
        .ingest_entities("Product", vec![make_product(
            "Python Cookbook", "Recipes for data science.", "pandas, numpy.", 39.99,
        )])
        .expect("ingestion complète");
    assert_eq!(
        res.rendu_pret,
        Some(D::TOUT),
        "le contrat historique d'ingest_entities ne bouge pas"
    );
}

/// **La coupe, sur le chemin où elle existe.**
///
/// Le graphe de drain ne découpe pas les entités simples — leurs chunks
/// viennent d'`ingest_entities`, qui garde son contrat complet. Il porte
/// l'embarquement pour deux choses : les lignes d'index KB, et les **chunks
/// réécrits** après une mise à jour. C'est ce second chemin qu'on éprouve ici.
///
/// Exiger `data + textsearch` réécrit les chunks et les indexe en plein texte
/// **sans passer par le GPU** ; leur embarquement devient une dette dans la
/// base. Exiger `dense` ensuite déclenche le rattrapage, qui la retrouve par
/// une requête — rien n'a été gardé en mémoire.
///
/// Les assertions portent sur la **dette**, pas sur la qualité vectorielle :
/// l'embarqueur de ce fichier est un factice à quatre dimensions, et faire
/// dépendre un test de ses similarités serait le rendre faux et fragile.
#[test]
#[ignore]
fn la_coupe_reecrit_sans_le_gpu_puis_rattrape() {
    use rag3weaver::disponibilite::Disponibilites as D;

    let mut catalog = setup_simple_catalog(4);
    catalog
        .ingest_entities("Product", vec![make_product(
            "Rust Book",
            "A comprehensive guide to Rust programming language covering ownership.",
            "Systems programming, memory safety.",
            49.99,
        )])
        .expect("ingestion complète");

    // Tout est embarqué : `ingest_entities` ne coupe rien.
    let apres_ingestion = catalog
        .search("Product", "programming", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::VECTOR),
            ..Default::default()
        })
        .expect("recherche");
    assert!(
        apres_ingestion.meta.warnings.iter().all(|a| !a.contains("pas encore été embarqués")),
        "aucune dette après une ingestion complète : {:?}", apres_ingestion.meta.warnings
    );

    // ── La mise à jour, puis la coupe ──────────────────────────────────
    let uuid = {
        let res = catalog
            .search("Product", "Rust", SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            })
            .expect("recherche");
        res.results.first().map(|r| r.uuid.clone()).expect("le produit posé")
    };
    let mut maj = BTreeMap::new();
    maj.insert(
        "description".to_string(),
        CypherValue::String(
            "Une refonte complète du texte, avec des mots entièrement nouveaux : \
             marmotte, clavecin, cartographie."
                .to_string(),
        ),
    );
    catalog.update("Product", &uuid, maj).expect("mise en file");

    let mut w = Vec::new();
    let (reste, _) = catalog.appliquer_la_consigne(D::RECHERCHE_TEXTE, false, 5_000, &mut w);
    assert_eq!(reste, 0, "la file est vidée jusqu'au plein texte : {w:?}");

    // Le plein texte trouve le nouveau texte, sans qu'aucun GPU ait tourné.
    let bm25 = catalog
        .search("Product", "clavecin", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        .expect("recherche");
    assert!(
        !bm25.results.is_empty(),
        "le plein texte doit voir les chunks réécrits sans étage GPU"
    );

    // Et la dette dense est là, **et elle se dit**.
    let dense = catalog
        .search("Product", "clavecin", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::VECTOR),
            ..Default::default()
        })
        .expect("recherche");
    eprintln!("[coupe] avertissements après la coupe : {:?}", dense.meta.warnings);
    assert!(
        dense.meta.warnings.iter().any(|a| a.contains("pas encore été embarqués")),
        "la coupe laisse une dette, et un zéro vectoriel doit dire que c'en est \
         une — pas une absence : {:?}", dense.meta.warnings
    );

    // ── Exiger le dense rattrape ───────────────────────────────────────
    // Par la consigne, pas par un appel direct : c'est le contrat qu'on
    // éprouve — « exiger dense » doit rattraper tout seul.
    let mut w = Vec::new();
    catalog.appliquer_la_consigne(D::TOUT, false, 5_000, &mut w);

    let dense = catalog
        .search("Product", "clavecin", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::VECTOR),
            ..Default::default()
        })
        .expect("recherche");
    eprintln!("[coupe] avertissements après rattrapage : {:?}", dense.meta.warnings);
    assert!(
        dense.meta.warnings.iter().all(|a| !a.contains("pas encore été embarqués")),
        "la passe de rattrapage doit avoir soldé la dette — sinon exiger « dense » \
         est une promesse non tenue : {:?}", dense.meta.warnings
    );
}

#[test]
#[ignore]
fn simple_bm25_no_results_for_nonsense() {
    let mut catalog = setup_simple_catalog(4);

    catalog
        .ingest_entities(
            "Product",
            vec![make_product("Test", "some description here", "some details", 10.0)],
        )
        
        .unwrap();

    let response = catalog
        .search(
            "Product",
            "xyzzy zyxwv qwerty",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        
        .unwrap();

    assert_eq!(response.results.len(), 0, "nonsense query should return 0 results");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 3 — Vector search with MiniLM embedder
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn simple_vector_minilm_search() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().unwrap();

    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::SEMANTIC;
    catalog.register_entity("Product", product_config).unwrap();

    let products = vec![
        make_product(
            "Rust Book",
            "A comprehensive guide to Rust programming language covering ownership, lifetimes, and concurrency.",
            "Covers systems programming, memory safety, and zero-cost abstractions.",
            49.99,
        ),
        make_product(
            "French Cuisine Guide",
            "La cuisine française est mondialement reconnue pour ses sauces et pâtisseries.",
            "Techniques de cuisson, recettes traditionnelles et gastronomie moderne.",
            34.99,
        ),
        make_product(
            "ML Textbook",
            "Deep learning uses neural networks with many layers for pattern recognition.",
            "Transformers and attention mechanisms have revolutionized NLP.",
            59.99,
        ),
    ];

    let mut rx = catalog.subscribe();
    catalog.ingest_entities("Product", products).unwrap();

    // Drain events
    while let Ok(event) = rx.try_recv() {
        match &event {
            CatalogEvent::Error { context, message } => {
                eprintln!("  [EVENT ERROR] {context}: {message}");
            }
            _ => eprintln!("  [EVENT] {event:?}"),
        }
    }

    // Debug: DB state
    let products_cnt = catalog.execute_raw("MATCH (p:Product) RETURN count(p)").unwrap();
    eprintln!("[MiniLM] Products: {:?}", products_cnt.rows);
    let chunks = catalog.execute_raw("MATCH (c:Product_Chunk) RETURN count(c)").unwrap();
    eprintln!("[MiniLM] Chunks: {:?}", chunks.rows);
    // La colonne et le marqueur sont ceux du modèle courant, résolus par le
    // catalogue — plus `embedding` / `_embed_hash` en dur (7 septembre 2026).
    let stockage = catalog.vector_storage("Product_Chunk").unwrap();
    let embs = catalog.execute_raw(&format!(
        "MATCH (c:Product_Chunk) RETURN c._uuid, c._text, size(c.{}) AS dim, c.{} LIMIT 5",
        stockage.column, stockage.marker
    )).unwrap();
    for row in &embs.rows {
        eprintln!("[MiniLM] Chunk: {:?}", row);
    }

    // Debug: check CHUNKED_FROM direction
    let fwd = catalog.execute_raw("MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]->(p:Product) RETURN count(c)").unwrap();
    let rev = catalog.execute_raw("MATCH (p:Product)-[:Product_CHUNKED_FROM]->(c:Product_Chunk) RETURN count(c)").unwrap();
    eprintln!("[MiniLM] CHUNKED_FROM fwd={:?} rev={:?}", fwd.rows, rev.rows);

    // Search for programming → should find Rust Book
    let response = catalog
        .search(
            "Product",
            "systems programming and memory safety",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::SEMANTIC),
                diagnostics: true,
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!(
        "[MiniLM] vector search: {} results, vector_count={}",
        response.results.len(),
        response.meta.vector_count
    );
    if let Some(diag) = &response.meta.diagnostics {
        eprintln!("[MiniLM] diagnostics: {:?}", diag);
    }

    // Drain search events
    while let Ok(event) = rx.try_recv() {
        match &event {
            CatalogEvent::Error { context, message } => {
                eprintln!("  [SEARCH EVENT ERROR] {context}: {message}");
            }
            _ => eprintln!("  [SEARCH EVENT] {event:?}"),
        }
    }

    assert!(!response.results.is_empty(), "should find results");
    assert!(response.meta.vector_count > 0, "vector_count should be > 0");
    assert_eq!(response.meta.target, "Product");

    let top = &response.results[0];
    eprintln!("[MiniLM] top: score={}, entity={:?}", top.score, top.entity);
    if let Some(chunk) = &top.chunk {
        let snippet: String = chunk.text.chars().take(60).collect();
        eprintln!("[MiniLM] chunk text: '{snippet}...'");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 4 — Hybrid search (BM25 + Vector) with BGE-M3
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn simple_hybrid_bgem3_search() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let dim = embedder.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(embedder);
    catalog.initialize().unwrap();

    catalog.register_entity("Product", make_product_config()).unwrap();

    let products = vec![
        make_product(
            "Rust Book",
            "A comprehensive guide to Rust programming language covering ownership, lifetimes, and concurrency.",
            "Covers systems programming, memory safety, and zero-cost abstractions.",
            49.99,
        ),
        make_product(
            "French Cuisine Guide",
            "La cuisine française est mondialement reconnue pour ses sauces et pâtisseries.",
            "Techniques de cuisson, recettes traditionnelles et gastronomie moderne.",
            34.99,
        ),
        make_product(
            "ML Textbook",
            "Deep learning uses neural networks with many layers for pattern recognition.",
            "Transformers and attention mechanisms have revolutionized NLP.",
            59.99,
        ),
    ];

    catalog.ingest_entities("Product", products).unwrap();

    // Hybrid search → should use both BM25 and vector
    let response = catalog
        .search(
            "Product",
            "programming language",
            SearchOptions {
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!(
        "[BGE-M3 hybrid] {} results, bm25={}, vector={}, fused={}",
        response.results.len(),
        response.meta.bm25_count,
        response.meta.vector_count,
        response.meta.fused_count
    );
    assert!(!response.results.is_empty(), "hybrid should find results");
    assert_eq!(response.meta.target, "Product");
    // In hybrid mode, we expect at least one signal to fire
    assert!(
        response.meta.bm25_count > 0 || response.meta.vector_count > 0,
        "at least one signal should produce results"
    );
}

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn simple_sparse_bgem3_search() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let sparse: Arc<dyn SparseEmbedder> = BGE_M3.clone();
    let dim = embedder.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(embedder);
    catalog.set_sparse_embedder(sparse);
    catalog.initialize().unwrap();

    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::HYBRID | SearchSignals::SPARSE;
    catalog.register_entity("Product", product_config).unwrap();

    let products = vec![
        make_product(
            "Rust Book",
            "A comprehensive guide to Rust programming language covering ownership, lifetimes, and concurrency.",
            "Covers systems programming, memory safety, and zero-cost abstractions.",
            49.99,
        ),
        make_product(
            "ML Textbook",
            "Deep learning uses neural networks with many layers for pattern recognition.",
            "Transformers and attention mechanisms have revolutionized NLP.",
            59.99,
        ),
    ];

    catalog.ingest_entities("Product", products).unwrap();

    let response = catalog
        .search(
            "Product",
            "programming",
            SearchOptions {
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!(
        "[BGE-M3 sparse] {} results, bm25={}, vector={}, sparse={}",
        response.results.len(),
        response.meta.bm25_count,
        response.meta.vector_count,
        response.meta.sparse_count
    );
    assert!(!response.results.is_empty(), "sparse hybrid should find results");
    assert_eq!(response.meta.target, "Product");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 5 — BM25 highlights + chunk resolution (long multi-field content)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn simple_bm25_highlights_resolve_to_correct_chunks() {
    let mut catalog = setup_simple_catalog(4);

    // Long content across 2 fields to force multiple chunks.
    // "description" field: ~600 chars about Rust, mentions "borrow checker" only here.
    // "details" field: ~600 chars about deployment, mentions "kubernetes" only here.
    // The word "performance" appears in both fields.
    let products = vec![make_product(
        "Rust Systems Guide",
        // description (~600 chars)
        "Rust is a systems programming language that guarantees memory safety without \
         a garbage collector. The borrow checker enforces ownership rules at compile \
         time, preventing data races and dangling pointers. Rust's type system is one \
         of the most expressive available, with algebraic data types, pattern matching, \
         and trait-based generics. The language achieves C-level performance while \
         maintaining safety guarantees that would normally require a managed runtime. \
         Fearless concurrency is a key feature — threads share data safely through \
         ownership transfer and synchronized references. The standard library provides \
         async/await for non-blocking IO, channels for message passing, and atomic \
         primitives for lock-free algorithms.",
        // details (~600 chars)
        "Deploying Rust applications in production requires understanding the build \
         system and toolchain. Cargo manages dependencies, builds, and testing with \
         a single unified tool. Cross-compilation targets include ARM, WASM, and \
         embedded platforms. Container images can be extremely small since Rust \
         produces static binaries with no runtime dependencies. Kubernetes orchestration \
         works seamlessly with Rust microservices thanks to low memory footprint and \
         fast startup times. Observability is achieved through the tracing ecosystem \
         which provides structured logging, distributed tracing, and metrics collection. \
         Performance profiling uses tools like perf, flamegraph, and criterion for \
         benchmarking.",
        79.99,
    )];

    catalog.ingest_entities("Product", products).unwrap();

    // Debug: show chunks with offsets
    let chunks_debug = catalog
        .execute_raw(
            "MATCH (c:Product_Chunk)-[:Product_CHUNKED_FROM]->(p:Product) \
             RETURN c._uuid, c._parent_field, c._content_offset, c._start_char, c._end_char, \
                    substring(c._text, 0, 60) AS snippet \
             ORDER BY c._content_offset, c._start_char"
        )
        
        .unwrap();
    eprintln!("\n--- Chunks ---");
    for row in &chunks_debug.rows {
        let uuid = row[0].as_str().unwrap_or("?");
        let field = row[1].as_str().unwrap_or("?");
        let offset = row[2].as_i64().unwrap_or(-1);
        let start = row[3].as_i64().unwrap_or(-1);
        let end = row[4].as_i64().unwrap_or(-1);
        let snippet = row[5].as_str().unwrap_or("?");
        eprintln!("  [{field}] offset={offset} chars=[{start}..{end}] uuid={} text='{snippet}...'",
            &uuid[..8.min(uuid.len())]);
    }

    // 1. Search "borrow checker" — only in description field
    let response = catalog
        .search(
            "Product",
            "borrow checker",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                diagnostics: true,
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!("\n--- Search 'borrow checker' ---");
    eprintln!("results={}, bm25_count={}", response.results.len(), response.meta.bm25_count);
    if let Some(ref diag) = response.meta.diagnostics {
        for (i, hit) in diag.bm25_hits.iter().enumerate() {
            eprintln!("  bm25_hit[{i}]: parent={}, score={:.4}", &hit.parent_uuid[..8.min(hit.parent_uuid.len())], hit.score);
            eprintln!("    hl_raw={}", hit.highlights_raw);
            eprintln!("    hl_parsed={:?}", hit.highlights_parsed);
            eprintln!("    chunks_available={}, chunks_matched={}", hit.chunks_available, hit.chunks_matched);
            for co in &hit.chunk_overlaps {
                eprintln!("    chunk {}: offset={}, [{},{}], global=[{},{}], overlap={}",
                    &co.chunk_uuid[..8.min(co.chunk_uuid.len())],
                    co.content_offset, co.start_char, co.end_char,
                    co.global_start, co.global_end, co.overlap);
            }
        }
    }
    assert!(!response.results.is_empty(), "'borrow checker' should match");
    if let Some(chunk) = &response.results[0].chunk {
        eprintln!("  resolved chunk: uuid={}, text='{}'",
            &chunk.uuid[..8.min(chunk.uuid.len())],
            &chunk.text.chars().take(80).collect::<String>());
        assert!(
            chunk.text.contains("borrow checker") || chunk.text.contains("borrow"),
            "chunk should contain 'borrow checker', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    }

    // 2. Search "kubernetes" — only in details field
    let response2 = catalog
        .search(
            "Product",
            "kubernetes",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                diagnostics: true,
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!("\n--- Search 'kubernetes' ---");
    eprintln!("results={}, bm25_count={}", response2.results.len(), response2.meta.bm25_count);
    if let Some(ref diag) = response2.meta.diagnostics {
        for (i, hit) in diag.bm25_hits.iter().enumerate() {
            eprintln!("  bm25_hit[{i}]: hl_raw={}", hit.highlights_raw);
            eprintln!("    hl_parsed={:?}", hit.highlights_parsed);
            eprintln!("    chunks_available={}, chunks_matched={}", hit.chunks_available, hit.chunks_matched);
            for co in &hit.chunk_overlaps {
                eprintln!("    chunk {}: offset={}, overlap={}",
                    &co.chunk_uuid[..8.min(co.chunk_uuid.len())],
                    co.content_offset, co.overlap);
            }
        }
    }
    assert!(!response2.results.is_empty(), "'kubernetes' should match");
    if let Some(chunk) = &response2.results[0].chunk {
        eprintln!("  resolved chunk: text='{}'",
            &chunk.text.chars().take(80).collect::<String>());
        assert!(
            chunk.text.contains("Kubernetes") || chunk.text.contains("kubernetes"),
            "chunk should contain 'kubernetes', got: '{}'",
            &chunk.text[..80.min(chunk.text.len())]
        );
    }

    // 3. Search "performance" — in both fields, check Detailed mode returns multiple chunks
    let response3 = catalog
        .search(
            "Product",
            "performance",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                result_mode: ResultMode::Detailed,
                diagnostics: true,
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!("\n--- Search 'performance' (Detailed) ---");
    eprintln!("results={}, bm25_count={}", response3.results.len(), response3.meta.bm25_count);
    for (i, r) in response3.results.iter().enumerate() {
        eprintln!("  result[{i}]: score={:.4}, chunks={:?}",
            r.score, r.chunks.as_ref().map(|c| c.len()));
        if let Some(chunks) = &r.chunks {
            for (j, ac) in chunks.iter().enumerate() {
                let snippet: String = ac.text.chars().take(60).collect();
                eprintln!("    chunk[{j}]: source_field={} text='{snippet}...'",
                    ac.source_field);
            }
        }
    }
    if let Some(ref diag) = response3.meta.diagnostics {
        for hit in &diag.bm25_hits {
            eprintln!("  diag: hl_parsed={:?} chunks_matched={}", hit.highlights_parsed, hit.chunks_matched);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 6 — Multiple ingestions + incremental
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn simple_multiple_ingestions() {
    let mut catalog = setup_simple_catalog(4);

    // First batch
    let batch1 = vec![
        make_product("Product A", "First batch item alpha", "Details about alpha", 10.0),
        make_product("Product B", "First batch item beta", "Details about beta", 20.0),
    ];
    let r1 = catalog.ingest_entities("Product", batch1).unwrap();
    assert_eq!(r1.failed, 0);

    // Second batch
    let batch2 = vec![
        make_product("Product C", "Second batch item gamma", "Details about gamma", 30.0),
    ];
    let r2 = catalog.ingest_entities("Product", batch2).unwrap();
    assert_eq!(r2.failed, 0);

    // Total count should be 3
    let count = catalog
        .execute_raw("MATCH (p:Product) RETURN count(p) AS cnt")
        
        .unwrap();
    let cnt = count.rows[0][0].as_i64().unwrap();
    assert_eq!(cnt, 3, "should have 3 products after 2 batches");

    // BM25 search should find across both batches
    let response = catalog
        .search(
            "Product",
            "batch item",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        )
        
        .unwrap();

    eprintln!("multi-ingest search: {} results", response.results.len());
    assert!(
        response.results.len() >= 2,
        "should find items from both batches: got {}",
        response.results.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 7 — CRUD: delete, update, batch operations
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: execute a Cypher count query and return the single i64 result.
fn query_count(catalog: &Catalog, cypher: &str) -> i64 {
    let result = catalog.execute_raw(cypher).unwrap();
    result
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// Helper: get all UUIDs of Product entities.
fn get_product_uuids(catalog: &Catalog) -> Vec<String> {
    let result = catalog
        .execute_raw("MATCH (p:Product) RETURN p._uuid ORDER BY p.name")
        
        .unwrap();
    result
        .rows
        .iter()
        .filter_map(|r| r.first().and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect()
}

#[test]
#[ignore]
fn simple_delete_removes_chunks() {
    let mut catalog = setup_simple_catalog(4);

    let products = vec![
        make_product("Alpha Widget", "Advanced alpha technology for computing", "Alpha details here", 10.0),
        make_product("Beta Gadget", "Beta engineering and manufacturing process", "Beta details here", 20.0),
        make_product("Gamma Tool", "Gamma precision instruments for research", "Gamma details here", 30.0),
    ];
    catalog.ingest_entities("Product", products).unwrap();

    let product_count = query_count(&catalog, "MATCH (p:Product) RETURN count(p)");
    assert_eq!(product_count, 3);
    let total_chunks = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    assert!(total_chunks >= 3, "should have chunks: {total_chunks}");
    eprintln!("Before delete: {product_count} products, {total_chunks} chunks");

    // Get UUID of the first product
    let uuids = get_product_uuids(&catalog);
    let delete_uuid = &uuids[0];
    eprintln!("Deleting product: {delete_uuid}");

    // Count chunks for this product before delete
    let chunks_before = query_count(
        &catalog,
        &format!("MATCH (c:Product_Chunk {{_parent_uuid: '{delete_uuid}'}}) RETURN count(c)"),
    )
    ;
    assert!(chunks_before >= 1, "product should have chunks: {chunks_before}");

    // Delete via catalog API
    catalog.delete("Product", delete_uuid).unwrap();
    let flush = catalog.drain();
    assert_eq!(flush.delete_results.len(), 1, "drain should have one delete result");
    let del_result = &flush.delete_results[0];
    eprintln!("delete result: chunks_deleted={}", del_result.chunks_deleted);
    assert!(del_result.chunks_deleted >= 1, "should report deleted chunks");

    // Verify entity gone
    let product_count_after = query_count(&catalog, "MATCH (p:Product) RETURN count(p)");
    assert_eq!(product_count_after, 2);

    // Verify chunks gone for deleted product
    let chunks_after = query_count(
        &catalog,
        &format!("MATCH (c:Product_Chunk {{_parent_uuid: '{delete_uuid}'}}) RETURN count(c)"),
    )
    ;
    assert_eq!(chunks_after, 0, "deleted product's chunks should be gone");

    // Remaining products still have chunks
    let remaining_chunks = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    assert!(remaining_chunks > 0, "other products should still have chunks");
    eprintln!("After delete: {product_count_after} products, {remaining_chunks} chunks");

    // BM25: deleted product not findable
    let response = catalog
        .search("Product", "alpha technology", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert_eq!(response.results.len(), 0, "deleted product should not be searchable");

    // BM25: remaining products still findable
    let response2 = catalog
        .search("Product", "beta engineering", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(!response2.results.is_empty(), "remaining product should still be searchable");
}

#[test]
#[ignore]
fn simple_update_refreshes_chunks() {
    let mut catalog = setup_simple_catalog(4);

    let products = vec![make_product(
        "Rust Book",
        "A comprehensive guide to Rust programming language",
        "Covers ownership, lifetimes, and concurrency patterns",
        49.99,
    )];
    catalog.ingest_entities("Product", products).unwrap();

    let uuids = get_product_uuids(&catalog);
    let uuid = &uuids[0];

    // Verify initial search
    let response = catalog
        .search("Product", "programming", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(!response.results.is_empty(), "should find 'programming' before update");

    // Update to completely different content
    let new_data = make_product(
        "Python Cookbook",
        "Recipes for mastering Python data science and web development",
        "Includes pandas, numpy, flask and asyncio examples",
        39.99,
    );
    catalog.update("Product", uuid, new_data).unwrap();
    let flush = catalog.drain();
    assert_eq!(flush.update_results.len(), 1, "drain should have one update result");
    let result = &flush.update_results[0];
    eprintln!(
        "update: status={:?}, reembedded={}, chunks_deleted={}, chunks_created={}",
        result.status, result.reembedded, result.chunks_deleted, result.chunks_created
    );
    assert!(matches!(result.status, UpdateStatus::Updated));
    assert!(result.reembedded, "should have re-embedded");

    // Old content should not be findable
    let response_old = catalog
        .search("Product", "programming", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert_eq!(
        response_old.results.len(),
        0,
        "old content 'programming' should not be findable after update"
    );

    // New content should be findable
    let response_new = catalog
        .search("Product", "data science", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(
        !response_new.results.is_empty(),
        "new content 'data science' should be findable after update"
    );
}

/// Régression : une mise à jour partielle (un seul champ) ne doit pas faire
/// disparaître les autres champs texte de l'index FTS — `add_document` n'est
/// pas un merge, la ré-indexation doit relire la ligne entière.
#[test]
#[ignore]
fn simple_partial_update_keeps_other_fields_indexed() {
    let mut catalog = setup_simple_catalog(4);
    catalog
        .ingest_entities(
            "Product",
            vec![make_product(
                "Rust Book",
                "A comprehensive guide to Rust programming language",
                "Covers ownership, lifetimes, and concurrency patterns",
                49.99,
            )],
        )
        .unwrap();
    let uuid = get_product_uuids(&catalog).remove(0);

    fn bm25(catalog: &mut Catalog, q: &str) -> usize {
        catalog
            .search("Product", q, SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            })
            .unwrap()
            .results
            .len()
    }
    assert!(bm25(&mut catalog, "lifetimes") > 0, "`details` indexé avant la mise à jour");

    // Ne touche que `description`.
    let mut partial = BTreeMap::new();
    partial.insert(
        "description".to_string(),
        CypherValue::String("Recipes for mastering Python data science".into()),
    );
    catalog.update("Product", &uuid, partial).unwrap();
    let flush = catalog.drain();
    assert_eq!(flush.failed, 0, "drain: {} échec(s)", flush.failed);

    assert!(bm25(&mut catalog, "Python") > 0, "la nouvelle description est indexée");
    assert_eq!(bm25(&mut catalog, "comprehensive"), 0, "l'ancienne description ne l'est plus");
    assert!(
        bm25(&mut catalog, "lifetimes") > 0,
        "`details`, non modifié, doit rester dans l'index après une mise à jour partielle"
    );
}

#[test]
#[ignore]
fn simple_update_unchanged_no_rechunk() {
    let mut catalog = setup_simple_catalog(4);

    let products = vec![make_product(
        "Widget",
        "A useful widget for everyday tasks",
        "Details about the widget",
        10.0,
    )];
    catalog.ingest_entities("Product", products).unwrap();

    let uuids = get_product_uuids(&catalog);
    let uuid = &uuids[0];
    let chunks_before = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");

    // Update only price (non-content field) — content fields unchanged
    let same_content_data = make_product(
        "Widget",
        "A useful widget for everyday tasks",
        "Details about the widget",
        99.99, // only price changed
    );
    catalog.update("Product", uuid, same_content_data).unwrap();
    let flush = catalog.drain();
    assert_eq!(flush.update_results.len(), 1, "drain should have one update result");
    let result = &flush.update_results[0];
    eprintln!(
        "update unchanged: status={:?}, reembedded={}, chunks_deleted={}, chunks_created={}",
        result.status, result.reembedded, result.chunks_deleted, result.chunks_created
    );
    assert!(
        matches!(result.status, UpdateStatus::Unchanged),
        "should be Unchanged when only non-content field changes"
    );
    assert!(!result.reembedded, "should not re-embed");
    assert_eq!(result.chunks_deleted, 0);
    assert_eq!(result.chunks_created, 0);

    // Chunk count unchanged
    let chunks_after = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    assert_eq!(chunks_before, chunks_after, "chunk count should be unchanged");
}

#[test]
#[ignore]
fn simple_batch_delete_multiple() {
    let mut catalog = setup_simple_catalog(4);

    // **Trois vocabulaires disjoints.** Le montage précédent donnait la même
    // phrase aux trois — « … description content here » — et le test
    // n'affirmait donc rien : chercher « alpha description » après avoir
    // supprimé Alpha remontait Beta sur le mot `description`, et l'échec se
    // lisait comme un index corrompu. Un mot propre à chacun, et l'assertion
    // porte enfin sur la suppression.
    let products = vec![
        make_product("Alpha", "xylophone crescendo", "arpege", 10.0),
        make_product("Beta", "tourbillon marin", "estuaire", 20.0),
        make_product("Gamma", "sextant nocturne", "meridien", 30.0),
    ];
    catalog.ingest_entities("Product", products).unwrap();

    let uuids = get_product_uuids(&catalog);
    assert_eq!(uuids.len(), 3);
    let total_chunks_before = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    eprintln!("Before batch_delete: 3 products, {total_chunks_before} chunks");

    // Delete first and third
    let to_delete = vec![uuids[0].clone(), uuids[2].clone()];
    for uuid in &to_delete {
        catalog.delete("Product", uuid).unwrap();
    }
    let flush = catalog.drain();
    assert_eq!(flush.delete_results.len(), 2);
    for r in &flush.delete_results {
        eprintln!("  deleted {}: chunks_deleted={}", &r.uuid[..8], r.chunks_deleted);
        assert!(r.chunks_deleted >= 1, "each deleted product should have had chunks");
    }

    // Only Beta remains
    let product_count = query_count(&catalog, "MATCH (p:Product) RETURN count(p)");
    assert_eq!(product_count, 1);

    let remaining_uuids = get_product_uuids(&catalog);
    assert_eq!(remaining_uuids.len(), 1);
    assert_eq!(remaining_uuids[0], uuids[1], "Beta should remain");

    // Chunks only for Beta
    let remaining_chunks = query_count(&catalog, "MATCH (c:Product_Chunk) RETURN count(c)");
    let beta_chunks = query_count(
        &catalog,
        &format!(
            "MATCH (c:Product_Chunk {{_parent_uuid: '{}'}}) RETURN count(c)",
            uuids[1]
        ),
    )
    ;
    assert_eq!(remaining_chunks, beta_chunks, "all remaining chunks should belong to Beta");

    // BM25: Beta still searchable
    let response = catalog
        .search("Product", "tourbillon", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(!response.results.is_empty(), "Beta should still be searchable");

    // BM25: Alpha not searchable
    let response2 = catalog
        .search("Product", "xylophone", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    let noms: Vec<String> = response2
        .results
        .iter()
        .filter_map(|r| r.data.as_ref()?.get("name")?.as_str().map(|s| s.to_string()))
        .collect();
    assert_eq!(
        response2.results.len(), 0,
        "« xylophone » n'appartenait qu'à Alpha, supprimé — remonté : {noms:?}"
    );
}

#[test]
#[ignore]
fn simple_batch_update_multiple() {
    let mut catalog = setup_simple_catalog(4);

    // Même règle qu'au-dessus : trois vocabulaires disjoints, et aucun mot du
    // contenu qui reparaisse dans le nom. Sans quoi « Alpha original » remonte
    // Alpha **par son titre**, et le test ne dit rien de son ancien contenu.
    let products = vec![
        make_product("Alpha", "xylophone crescendo", "arpege", 10.0),
        make_product("Beta", "tourbillon marin", "estuaire", 20.0),
        make_product("Gamma", "sextant nocturne", "meridien", 30.0),
    ];
    catalog.ingest_entities("Product", products).unwrap();

    let uuids = get_product_uuids(&catalog);
    assert_eq!(uuids.len(), 3);

    // batch_update: change Alpha + Gamma content, Beta only price
    let updates = vec![
        (
            uuids[0].clone(),
            make_product("Alpha", "clavecin baroque", "cadence", 10.0),
        ),
        (
            uuids[1].clone(),
            make_product("Beta", "tourbillon marin", "estuaire", 99.99), // le prix seul
        ),
        (
            uuids[2].clone(),
            make_product("Gamma", "obsidienne fumee", "basalte", 30.0),
        ),
    ];
    for (uuid, data) in updates {
        catalog.update("Product", &uuid, data).unwrap();
    }
    let flush = catalog.drain();
    let results = &flush.update_results;
    assert_eq!(results.len(), 3);

    eprintln!("batch_update results:");
    for r in results {
        eprintln!(
            "  {}: status={:?}, reembedded={}, chunks_deleted={}, chunks_created={}",
            &r.uuid[..8],
            r.status,
            r.reembedded,
            r.chunks_deleted,
            r.chunks_created
        );
    }

    // Alpha: Updated + reembedded
    assert!(matches!(results[0].status, UpdateStatus::Updated));
    assert!(results[0].reembedded);

    // Beta: Unchanged (only price changed, not content)
    assert!(matches!(results[1].status, UpdateStatus::Unchanged));
    assert!(!results[1].reembedded);

    // Gamma: Updated + reembedded
    assert!(matches!(results[2].status, UpdateStatus::Updated));
    assert!(results[2].reembedded);

    // **Et les chunks comptent pour de vrai.** Ces deux champs étaient des
    // zéros écrits en dur dans `UpdateRecordNode` : le rechunkage a lieu en
    // aval, le nœud ne pouvait pas les connaître, et personne ne l'avait
    // remarqué parce qu'aucun test ne les regardait. `Catalog::drain` les
    // recolle maintenant depuis le service `chunk_counts`.
    for (rang, nom) in [(0usize, "Alpha"), (2, "Gamma")] {
        assert!(
            results[rang].chunks_deleted >= 1,
            "{nom} a été réécrit : ses anciens chunks devaient être supprimés, \
             chunks_deleted={}",
            results[rang].chunks_deleted
        );
        assert!(
            results[rang].chunks_created >= 1,
            "{nom} a été réécrit : de nouveaux chunks devaient naître, \
             chunks_created={}",
            results[rang].chunks_created
        );
    }
    assert_eq!(
        (results[1].chunks_deleted, results[1].chunks_created),
        (0, 0),
        "Beta n'a changé que de prix : aucun chunk ne devait bouger"
    );

    // BM25: old Alpha content not findable
    let response = catalog
        .search("Product", "xylophone", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    let noms: Vec<String> = response
        .results
        .iter()
        .filter_map(|r| r.data.as_ref()?.get("name")?.as_str().map(|s| s.to_string()))
        .collect();
    assert_eq!(
        response.results.len(), 0,
        "« xylophone » était l'ancien contenu d'Alpha, réécrit — remonté : {noms:?}"
    );

    // BM25: new Alpha content findable
    let response2 = catalog
        .search("Product", "clavecin", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(!response2.results.is_empty(), "new Alpha content should be findable");

    // BM25: Beta still findable with original content
    let response3 = catalog
        .search("Product", "tourbillon", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        
        .unwrap();
    assert!(!response3.results.is_empty(), "Beta original content should still be findable");
}


// ═══════════════════════════════════════════════════════════════════════════
// L'état et ses transitions (doc 07 §4)
// ═══════════════════════════════════════════════════════════════════════════

fn ticket_config() -> EntityConfig {
    use rag3weaver::config::{Lifecycle, Transition};
    let mut fields = HashMap::new();
    fields.insert("title".to_string(), SimpleFieldDef {
        field_type: FieldType::String, is_title: true, ..Default::default()
    });
    fields.insert("body".to_string(), SimpleFieldDef {
        field_type: FieldType::Text, is_content: true, ..Default::default()
    });
    fields.insert("status".to_string(), SimpleFieldDef {
        field_type: FieldType::String, ..Default::default()
    });
    let t = |name: &str, from: &str, to: &str| Transition {
        name: name.into(), from: from.into(), to: to.into(),
    };
    EntityConfig {
        fields,
        signals: SearchSignals::BM25,
        hashsafe: Some(vec!["title".into()]),
        lifecycle: Some(Lifecycle {
            field: "status".into(),
            initial: "open".into(),
            transitions: vec![t("start", "open", "in_progress"), t("close", "in_progress", "closed")],
        }),
        ..Default::default()
    }
}

fn ticket_status(catalog: &Catalog) -> String {
    catalog
        .execute_raw("MATCH (t:Ticket) RETURN t.status")
        .unwrap()
        .rows
        .first()
        .and_then(|r| r.first().and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_default()
}

/// **Une déclaration vérifiée mais non appliquée est un piège.**
///
/// `Lifecycle` était contrôlé à l'enregistrement et n'empêchait rien à
/// l'écriture. Ce test exerce la garde là où elle vit — au drain, seul endroit
/// où l'**ancien** état est connu.
#[test]
#[ignore]
fn une_transition_non_declaree_ne_passe_pas() {
    let mut catalog = setup_simple_catalog(4);
    catalog.register_entity("Ticket", ticket_config()).unwrap();

    let mut data = BTreeMap::new();
    data.insert("title".to_string(), CypherValue::String("Le masque HNSW".into()));
    data.insert("body".to_string(), CypherValue::String("Le balayage ignorait le masque".into()));
    data.insert("status".to_string(), CypherValue::String("open".into()));
    catalog.ingest_entities("Ticket", vec![data]).unwrap();
    let uuid = catalog
        .execute_raw("MATCH (t:Ticket) RETURN t._uuid")
        .unwrap()
        .rows[0][0]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(ticket_status(&catalog), "open");

    // Déclarée : elle passe.
    let mut vers = BTreeMap::new();
    vers.insert("status".to_string(), CypherValue::String("in_progress".into()));
    catalog.update("Ticket", &uuid, vers).unwrap();
    let flush = catalog.drain();
    assert_eq!(flush.failed, 0, "transition déclarée : {flush:?}");
    assert_eq!(ticket_status(&catalog), "in_progress");

    // Non déclarée : `in_progress -> open` n'existe pas. Ce qui compte est
    // moins le compteur d'échecs que **l'état, qui ne doit pas avoir bougé**.
    let mut retour = BTreeMap::new();
    retour.insert("status".to_string(), CypherValue::String("open".into()));
    catalog.update("Ticket", &uuid, retour).unwrap();
    let flush = catalog.drain();
    assert_eq!(ticket_status(&catalog), "in_progress", "la transition interdite ne doit rien écrire");
    // L'état d'abord — un compteur peut être juste pendant que la donnée est
    // fausse — mais le refus doit aussi **se voir** : un drain qui rejette en
    // silence laisserait croire à l'appelant que sa mise à jour a eu lieu.
    assert_eq!((flush.processed, flush.failed), (0, 1), "le refus doit se voir : {flush:?}");

    // Un champ ordinaire n'est pas concerné : ce n'est pas une transition.
    let mut corps = BTreeMap::new();
    corps.insert("body".to_string(), CypherValue::String("Une ligne suffisait".into()));
    catalog.update("Ticket", &uuid, corps).unwrap();
    let flush = catalog.drain();
    assert_eq!(flush.failed, 0, "une mise à jour hors état passe : {flush:?}");
    assert_eq!(ticket_status(&catalog), "in_progress");

    // Écrire l'état qu'on a déjà n'est pas un passage.
    let mut idem = BTreeMap::new();
    idem.insert("status".to_string(), CypherValue::String("in_progress".into()));
    catalog.update("Ticket", &uuid, idem).unwrap();
    let flush = catalog.drain();
    assert_eq!(flush.failed, 0, "état inchangé : {flush:?}");
    assert_eq!(ticket_status(&catalog), "in_progress");
}


/// **La dette de découpage vit dans la base** (C5). Une mise à jour posée au
/// niveau donnée pose ses champs et laisse ses chunks tels quels ;
/// `_chunked_hash` dit qu'ils sont en retard. Exiger le plein texte les
/// redécoupe — par une requête, rien n'a été gardé en mémoire — et la liste
/// des chunks reflète alors le nouveau contenu.
///
/// On compte les chunks : le texte de départ tient en un, celui de la mise à
/// jour en plusieurs. Tant que la dette n'est pas soldée, le compte ne bouge
/// pas ; après, il grandit.
#[test]
#[ignore]
fn une_mise_a_jour_au_niveau_donnee_laisse_une_dette_de_decoupage_qui_se_solde() {
    use rag3weaver::disponibilite::Disponibilites as D;

    let mut catalog = setup_simple_catalog(4);
    catalog
        .ingest_entities("Product", vec![make_product(
            "Rust Book", "Un guide court.", "Bref.", 49.99,
        )])
        .expect("ingestion");
    let uuid = catalog
        .search("Product", "guide", SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(SearchSignals::BM25),
            ..Default::default()
        })
        .expect("recherche")
        .results.first().map(|r| r.uuid.clone()).expect("le produit posé");
    // `count` ne connaît que les entités déclarées : la table de chunks se
    // compte par la connexion.
    let compter_les_chunks = |c: &Catalog| -> usize {
        c.conn_arc()
            .execute("MATCH (n:Product_Chunk) RETURN count(n) AS n")
            .expect("compte des chunks")
            .rows
            .first()
            .and_then(|l| l.first())
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize
    };
    let chunks_avant = compter_les_chunks(&catalog);
    assert!(chunks_avant >= 1);

    // Une description longue : plusieurs chunks, une fois découpée.
    let long = "clavecin ".repeat(600);
    let mut maj = BTreeMap::new();
    maj.insert("description".to_string(), CypherValue::String(long));
    let res = catalog.update_jusqu_a("Product", &uuid, maj, D::DONNEE).expect("mise à jour");
    assert_eq!(res.rendu_pret, Some(D::DONNEE), "la donnée, exactement : {res:?}");
    assert_eq!(
        compter_les_chunks(&catalog),
        chunks_avant,
        "au niveau donnée, les chunks ne bougent pas : ils sont en dette"
    );

    // Le plein texte exigé solde la dette de découpage.
    let mut w = Vec::new();
    catalog.appliquer_la_consigne_pour("Product", D::RECHERCHE_TEXTE, false, 5_000, &mut w);
    let chunks_apres = compter_les_chunks(&catalog);
    eprintln!("[découpage] avant={chunks_avant} après={chunks_apres} avertissements={w:?}");
    assert!(
        chunks_apres > chunks_avant,
        "la dette de découpage doit être soldée : {chunks_avant} → {chunks_apres} ({w:?})"
    );

    // Et une seconde passe n'a plus rien à redécouper.
    let encore = catalog.rattraper_le_decoupage(None, 512, false).expect("rattrapage");
    assert_eq!(encore, 0, "plus rien en retard");
}

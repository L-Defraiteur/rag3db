//! E2E integration tests: Generic search nodes (composable pipelines).
//!
//! Builds search pipelines from generic nodes (SearchSourceNode, VectorSearchNode,
//! BM25SearchNode, SparseSearchNode, FuseResultsNode, ResolveParentNode) and compares
//! results with `Catalog::rechercher` to validate equivalence.
//!
//! Run with: ./run_e2e.sh generic_search

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use std::sync::Mutex;

use rag3weaver::config::FieldType;
use rag3weaver::connection::CypherValue;
use rag3weaver::dataflow::{
    BM25SearchNode, DataflowGraph, DataflowRuntime, KBQuerySourceNode,
    ResolveParentNode, SearchSourceNode, ServiceRegistry, SparseSearchNode,
};
use rag3weaver::dataflow::QueryPayload;
#[cfg(feature = "burn-embedder")]
use rag3weaver::dataflow::{ExecutionStatus, FuseResultsNode, RerankNode, VectorSearchNode};
#[cfg(feature = "burn-embedder")]
use rag3weaver::reranker::Reranker;
use rag3weaver::search::BM25Mode;
use rag3weaver::embedder::{DualEmbedder, Embedder, MockEmbedder, SparseEmbedder};
use rag3weaver::search::{Consistency, SearchMeta, SearchOptions, SearchResult, SearchSignals};
use rag3weaver::search_strategy::UnifiedResult;
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, Rag3dbConnection, SimpleFieldDef};
use rag3weaver::disponibilite::RegimeEcriture;

#[allow(unused_imports)] // employé sous certaines features seulement
use crate::common;

#[cfg(feature = "burn-embedder")]
use common::burn::{BGE_M3, MINILM};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let extensions = [
        ("vector", crate::common::vector_extension()),
    ];
    for (name, ext_path) in &extensions {
        if !std::path::Path::new(ext_path).exists() {
            panic!("Extension '{name}' not found at: {ext_path}\nRun ./run_e2e.sh --build-only first.");
        }
        conn.execute(&format!("LOAD EXTENSION '{ext_path}'"))
            
            .unwrap_or_else(|e| panic!("Failed to load {name}: {e}"));
        eprintln!("✓ Loaded {name}");
    }
}

fn make_empty_config(dim: usize) -> CatalogConfig {
    CatalogConfig {
        name: Some("generic-search-test".into()),
        entities: HashMap::new(),
        relations: HashMap::new(),
        embedding_dim: dim,
        ..Default::default()
    }
}

fn make_product_config() -> EntityConfig {
    let mut fields = HashMap::new();
    fields.insert("name".into(), SimpleFieldDef {
        field_type: FieldType::String,
        is_title: true,
        is_content: false,
        ..Default::default()
    });
    fields.insert("description".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        is_title: false,
        is_content: true,
        ..Default::default()
    });
    fields.insert("details".into(), SimpleFieldDef {
        field_type: FieldType::Text,
        is_title: false,
        is_content: true,
        ..Default::default()
    });
    fields.insert("price".into(), SimpleFieldDef {
        field_type: FieldType::Double,
        is_title: false,
        is_content: false,
        ..Default::default()
    });
    EntityConfig {
        fields,
        signals: SearchSignals::HYBRID,
        ..Default::default()
    }
}

fn make_product(name: &str, description: &str, details: &str, price: f64) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("description".into(), CypherValue::String(description.into()));
    data.insert("details".into(), CypherValue::String(details.into()));
    data.insert("price".into(), CypherValue::Float(price));
    data
}

fn test_products() -> Vec<BTreeMap<String, CypherValue>> {
    vec![
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
    ]
}

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

/// Extract Vec<UnifiedResult> from pipeline output.
fn extract_results(
    output: &rag3weaver::dataflow::DataflowOutput,
    node: &str,
) -> Vec<UnifiedResult> {
    match output.get(node, "results").and_then(|v| v.downcast::<Vec<UnifiedResult>>()) {
        Some(r) => r.clone(),
        None => panic!("expected Vec<UnifiedResult> on '{node}.results', got: {:?}", output.get(node, "results")),
    }
}

/// Build services for generic search pipeline.

/// Enveloppe le catalogue le temps d'un appel au lanceur, puis le rend.
fn cherche(
    catalog: Catalog,
    cible: &str,
    requete: &str,
    options: SearchOptions,
) -> (Catalog, rag3weaver::search::SearchResponse) {
    let partage = std::sync::Arc::new(std::sync::Mutex::new(catalog));
    let reponse = Catalog::rechercher(&partage, cible, requete, options).unwrap();
    let catalog = std::sync::Arc::try_unwrap(partage)
        .ok()
        .expect("le graphe ne retient pas le catalogue")
        .into_inner()
        .unwrap();
    (catalog, reponse)
}

fn build_services(
    catalog: Catalog,
    embedder: Arc<dyn Embedder>,
    sparse_embedder: Option<Arc<dyn SparseEmbedder>>,
    dual_embedder: Option<Arc<dyn DualEmbedder>>,
) -> (ServiceRegistry, Arc<Mutex<Catalog>>) {
    let mut services = ServiceRegistry::new();
    // **Une seule source pour la liste.** Ce montage la reconstruisait pièce
    // par pièce, et c'est comme ça qu'il a perdu le dialecte le jour où
    // `BM25SearchNode` s'est mis à en avoir besoin. Le catalogue la connaît :
    // connexion, dialecte, cellule, index FTS et sparse, embarqueurs.
    catalog.register_search_services(&mut services);
    let catalog_arc = Arc::new(Mutex::new(catalog));
    services.register("catalog", catalog_arc.clone());

    // Ce que l'appelant choisit, lui : un embarqueur autre que celui du
    // catalogue, pour éprouver un montage précis.
    services.register::<Arc<dyn Embedder>>("embedder", embedder);
    if let Some(sparse) = sparse_embedder {
        services.register::<Arc<dyn SparseEmbedder>>("sparse_embedder", sparse);
    }
    if let Some(dual) = dual_embedder {
        services.register::<Arc<dyn DualEmbedder>>("dual_embedder", dual);
    }

    (services, catalog_arc)
}

/// Compare pipeline results (UnifiedResult) vs catalog results (SearchResult): same UUIDs (order-independent).
fn assert_same_uuids(pipeline: &[UnifiedResult], catalog: &[SearchResult], label: &str) {
    let mut pipe_uuids: Vec<&str> = pipeline.iter().map(|r| r.uuid.as_str()).collect();
    let mut cat_uuids: Vec<&str> = catalog.iter().map(|r| r.uuid.as_str()).collect();
    pipe_uuids.sort();
    cat_uuids.sort();
    assert_eq!(
        pipe_uuids, cat_uuids,
        "[{label}] UUIDs differ.\n  pipeline: {pipe_uuids:?}\n  catalog:  {cat_uuids:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// BM25 pipeline
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn generic_bm25_pipeline_matches_catalog() {
    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();

    // 1. le lanceur en référence
    let (catalog, cat_response) = cherche(
            catalog,
            "Product",
            "programming language",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::BM25),
                ..Default::default()
            },
        );
    eprintln!(
        "[BM25 catalog] {} results, bm25_count={}",
        cat_response.results.len(),
        cat_response.meta.bm25_count
    );

    // 2. Generic pipeline
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (services, _cat_arc) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming language",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("bm25", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    eprintln!("[BM25 pipeline] {} results", pipe_results.len());
    for (i, r) in pipe_results.iter().enumerate() {
        eprintln!("  [{i}] uuid={}, score={:.4}, entity={:?}", &r.uuid[..8.min(r.uuid.len())], r.score, r.entity);
    }

    // 3. Compare
    assert!(!pipe_results.is_empty(), "pipeline should find results");
    assert_same_uuids(&pipe_results, &cat_response.results, "BM25");

    // Top result should match
    assert_eq!(
        pipe_results[0].uuid, cat_response.results[0].uuid,
        "top result UUID should match"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// La consigne de cohérence, sur le chemin composable
// ═══════════════════════════════════════════════════════════════════════════════

/// **Le chemin que les agents empruntent applique-t-il la consigne ?**
///
/// Jusqu'au 6 septembre 2026, non. `Consistency` vivait dans le corps de
/// `Catalog::search`, que ce graphe **n'empruntait pas** : l'outil `search` offert
/// aux agents passe par `SearchSourceNode`, qui ne traversait aucune des trois
/// branches. `Consistency::Strict` n'était d'ailleurs construit nulle part dans
/// `src/`, et la marque d'eau d'ingestion — bâtie et éprouvée sur deux processus
/// réels — n'avait aucun appelant en production.
///
/// Le test met le moteur dans l'état exact où ça se voit : des entités **en
/// file**, jamais posées. Avant, le graphe cherchait dans une base vide et
/// rendait zéro, sans une erreur. Maintenant `SearchSourceNode` applique
/// `Eventual`, pose les entités, et les trouve.
#[test]
#[ignore]
fn le_graphe_applique_la_consigne_de_coherence() {
    let mut catalog = setup_simple_catalog(4);

    // `create()` met en file et ne pose rien : c'est tout l'intérêt.
    for produit in test_products() {
        catalog.create("Product", produit).expect("mise en file");
    }
    assert!(catalog.has_pending(), "rien n'est posé avant la recherche");

    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (services, cat_arc) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming language",
        SearchOptions { consistency: Consistency::Eventual, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("bm25", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let resultats = extract_results(&output, "resolve");

    eprintln!("[consigne] {} résultats depuis une file non vidée", resultats.len());
    assert!(
        !resultats.is_empty(),
        "le graphe doit appliquer la consigne et poser les entités avant de \
         chercher — zéro résultat ici veut dire qu'il a cherché dans une base \
         vide sans le dire"
    );

    // Le port `meta` du nœud source existe pour que ça se **dise**. Il porte la
    // consigne appliquée et ce qui reste en file.
    let meta = output
        .get("source", "meta")
        .and_then(|v| v.downcast::<SearchMeta>())
        .cloned()
        .expect("SearchSourceNode doit rendre une méta");
    eprintln!(
        "[consigne] méta : consistency={:?} partiel={} en file={} avertissements={:?}",
        meta.consistency, meta.partial, meta.pending_count, meta.warnings
    );
    assert_eq!(meta.consistency, Consistency::Eventual);
    assert_eq!(
        meta.partial,
        meta.pending_count > 0,
        "« partiel » dit exactement s'il reste du travail"
    );

    // Et la file a bien été vidée de ses entités par le nœud, pas par nous.
    let restant = cat_arc.lock().unwrap().pending_work().entities.len();
    assert_eq!(restant, 0, "les entités ont été posées par la consigne");
}

/// **Le troisième signal dit aussi pourquoi il se tait.**
///
/// `Catalog::search` expliquait un zéro dense *et* un zéro sparse par la dette
/// d'embarquement ; sur ce chemin, seul `VectorSearchNode` le faisait. Un
/// agent qui cherchait en sparse sur des chunks posés sans passe GPU concluait
/// « ça n'existe pas ». C'est le demi-portage relevé par la réconciliation du
/// 6 septembre 2026, et la raison pour laquelle `Catalog::search` devait
/// devenir ce gabarit — fait le 18 septembre : le monolithe est parti.
///
/// La coupe est faite par le verbe de lot qui dit sa portée ; le rattrapage
/// solde la dette ; entre les deux, le nœud sparse rend zéro **en le disant**.
#[test]
#[ignore]
fn le_noeud_sparse_explique_son_silence() {
    use rag3weaver::disponibilite::Disponibilites as D;
    use rag3weaver::embedder::MockSparseEmbedder;

    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let sparse: Arc<dyn SparseEmbedder> = Arc::new(MockSparseEmbedder);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), make_empty_config(4));
    catalog.set_sparse_embedder(sparse.clone());
    catalog.initialize().unwrap();
    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::BM25 | SearchSignals::SPARSE;
    catalog.register_entity("Product", product_config).unwrap();

    // Posé et indexé en plein texte, sans passe GPU : la dette sparse est là.
    let res = catalog
        .ingest_entities_jusqu_a("Product", test_products(), D::RECHERCHE_TEXTE)
        .expect("ingestion sans GPU");
    assert_eq!(res.rendu_pret, Some(D::RECHERCHE_TEXTE));

    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let cat_arc = Arc::new(Mutex::new(catalog));
    // Le même montage que `build_services`, mais rejouable : le catalogue est
    // interrogé deux fois, avant et après le rattrapage.
    let services_de = |cat: &Arc<Mutex<Catalog>>| {
        let mut services = ServiceRegistry::new();
        cat.lock().unwrap().register_search_services(&mut services);
        services.register("catalog", cat.clone());
        services.register::<Arc<dyn Embedder>>("embedder", embedder.clone());
        services.register::<Arc<dyn SparseEmbedder>>("sparse_embedder", sparse.clone());
        services
    };

    let chercher = |services| {
        let mut graph = DataflowGraph::new();
        graph.add_node(Box::new(SearchSourceNode::new(
            "source", "Product", "programming",
            SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
        ))).unwrap();
        graph.add_node(Box::new(SparseSearchNode::new("sparse", 10))).unwrap();
        graph.connect("source", "query", "sparse", "query").unwrap();
        let runtime = DataflowRuntime::with_services(100, services);
        let output = runtime.execute(&mut graph).unwrap();
        output
            .get("sparse", "meta")
            .and_then(|v| v.downcast::<SearchMeta>())
            .cloned()
            .expect("SparseSearchNode doit rendre une méta")
    };

    let meta = chercher(services_de(&cat_arc));
    eprintln!("[sparse muet] sparse_count={} avertissements={:?}", meta.sparse_count, meta.warnings);
    assert_eq!(meta.sparse_count, 0, "rien n'est embarqué en sparse : zéro, forcément");
    assert!(
        meta.warnings.iter().any(|a| a.contains("pas encore été embarqués")),
        "le zéro doit dire qu'il est une dette, pas une absence : {:?}", meta.warnings
    );

    // Le rattrapage solde la dette ; le même nœud ne s'en plaint plus.
    let repris = cat_arc.lock().unwrap().embarquer_le_retard(D::SPARSE, 512, None).expect("rattrapage");
    assert!(repris > 0, "la dette doit être retrouvée dans la base");

    let meta = chercher(services_de(&cat_arc));
    eprintln!("[sparse rattrapé] sparse_count={} avertissements={:?}", meta.sparse_count, meta.warnings);
    assert!(
        meta.warnings.iter().all(|a| !a.contains("pas encore été embarqués")),
        "plus rien de dû après le rattrapage : {:?}", meta.warnings
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Vector pipeline
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_vector_pipeline_matches_catalog() {
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
    catalog.ingest_entities("Product", test_products()).unwrap();

    // 1. le lanceur en référence
    let (catalog, cat_response) = cherche(
            catalog,
            "Product",
            "systems programming and memory safety",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::SEMANTIC),
                ..Default::default()
            },
        );
    eprintln!(
        "[Vector catalog] {} results, vector_count={}",
        cat_response.results.len(),
        cat_response.meta.vector_count
    );

    // 2. Generic pipeline
    let embedder: Arc<dyn Embedder> = MINILM.clone();
    let (services, _) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "systems programming and memory safety",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(VectorSearchNode::new("vector", 10))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "vector", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("vector", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    eprintln!("[Vector pipeline] {} results", pipe_results.len());
    for (i, r) in pipe_results.iter().enumerate() {
        eprintln!("  [{i}] uuid={}, score={:.4}", &r.uuid[..8.min(r.uuid.len())], r.score);
    }

    // 3. Compare
    assert!(!pipe_results.is_empty(), "pipeline should find results");
    assert_same_uuids(&pipe_results, &cat_response.results, "Vector");
    assert_eq!(pipe_results[0].uuid, cat_response.results[0].uuid, "top result should match");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Hybrid pipeline (BM25 + Vector → Fuse)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_hybrid_pipeline_matches_catalog() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();

    // 1. le lanceur en référence (HYBRID = BM25 + SEMANTIC)
    let (catalog, cat_response) = cherche(
            catalog,
            "Product",
            "programming language",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::HYBRID),
                ..Default::default()
            },
        );
    eprintln!(
        "[Hybrid catalog] {} results, bm25={}, vector={}, fused={}",
        cat_response.results.len(),
        cat_response.meta.bm25_count,
        cat_response.meta.vector_count,
        cat_response.meta.fused_count
    );

    // 2. Generic pipeline
    let embedder: Arc<dyn Embedder> = MINILM.clone();
    let (services, _) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming language",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(VectorSearchNode::new("vector", 10))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10))).unwrap();
    graph.add_node(Box::new(FuseResultsNode::new("fuse"))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "vector", "query").unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("vector", "results", "fuse", "vector").unwrap();
    graph.connect("bm25", "results", "fuse", "bm25").unwrap();
    graph.connect("fuse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    eprintln!("[Hybrid pipeline] {} results", pipe_results.len());
    for (i, r) in pipe_results.iter().enumerate() {
        eprintln!("  [{i}] uuid={}, score={:.4}", &r.uuid[..8.min(r.uuid.len())], r.score);
    }

    // 3. Compare
    assert!(!pipe_results.is_empty(), "pipeline should find results");
    assert_same_uuids(&pipe_results, &cat_response.results, "Hybrid");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Sparse pipeline (SparseSearchNode seul)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_sparse_pipeline_matches_catalog() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let sparse: Arc<dyn SparseEmbedder> = BGE_M3.clone();
    let dual: Arc<dyn DualEmbedder> = BGE_M3.clone();
    let dim = embedder.dim();

    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(embedder.clone());
    catalog.set_sparse_embedder(sparse.clone());
    catalog.set_dual_embedder(dual.clone());
    catalog.initialize().unwrap();

    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::SPARSE;
    catalog.register_entity("Product", product_config).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();

    // 1. le lanceur en référence
    let (catalog, cat_response) = cherche(
            catalog,
            "Product",
            "programming",
            SearchOptions {
                consistency: Consistency::Immediate,
                signals: Some(SearchSignals::SPARSE),
                ..Default::default()
            },
        );
    eprintln!(
        "[Sparse catalog] {} results, sparse_count={}",
        cat_response.results.len(),
        cat_response.meta.sparse_count
    );

    // 2. Generic pipeline
    let (services, _) = build_services(
        catalog,
        embedder,
        Some(sparse),
        Some(dual),
    );

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(SparseSearchNode::new("sparse", 10))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "sparse", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("sparse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    eprintln!("[Sparse pipeline] {} results", pipe_results.len());
    for (i, r) in pipe_results.iter().enumerate() {
        eprintln!("  [{i}] uuid={}, score={:.4}", &r.uuid[..8.min(r.uuid.len())], r.score);
    }

    // 3. Compare
    assert!(!pipe_results.is_empty(), "pipeline should find results");
    assert_same_uuids(&pipe_results, &cat_response.results, "Sparse");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Full hybrid pipeline (BM25 + Vector + Sparse → Fuse)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_full_hybrid_pipeline_matches_catalog() {
    let embedder: Arc<dyn Embedder> = BGE_M3.clone();
    let sparse: Arc<dyn SparseEmbedder> = BGE_M3.clone();
    let dual: Arc<dyn DualEmbedder> = BGE_M3.clone();
    let dim = embedder.dim();

    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(embedder.clone());
    catalog.set_sparse_embedder(sparse.clone());
    catalog.set_dual_embedder(dual.clone());
    catalog.initialize().unwrap();

    let mut product_config = make_product_config();
    product_config.signals = SearchSignals::HYBRID | SearchSignals::SPARSE;
    catalog.register_entity("Product", product_config).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();

    // 1. le lanceur en référence (HYBRID + SPARSE = all 3 signals)
    let (catalog, cat_response) = cherche(
            catalog,
            "Product",
            "programming",
            SearchOptions {
                consistency: Consistency::Immediate,
                ..Default::default()
            },
        );
    eprintln!(
        "[Full hybrid catalog] {} results, bm25={}, vector={}, sparse={}, fused={}",
        cat_response.results.len(),
        cat_response.meta.bm25_count,
        cat_response.meta.vector_count,
        cat_response.meta.sparse_count,
        cat_response.meta.fused_count
    );

    // 2. Generic pipeline: all 3 signal nodes → FuseResultsNode
    let (services, _) = build_services(
        catalog,
        embedder,
        Some(sparse),
        Some(dual),
    );

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(VectorSearchNode::new("vector", 10))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10))).unwrap();
    graph.add_node(Box::new(SparseSearchNode::new("sparse", 10))).unwrap();
    graph.add_node(Box::new(FuseResultsNode::new("fuse"))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "vector", "query").unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "sparse", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("vector", "results", "fuse", "vector").unwrap();
    graph.connect("bm25", "results", "fuse", "bm25").unwrap();
    graph.connect("sparse", "results", "fuse", "sparse").unwrap();
    graph.connect("fuse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    eprintln!("[Full hybrid pipeline] {} results", pipe_results.len());
    for (i, r) in pipe_results.iter().enumerate() {
        eprintln!("  [{i}] uuid={}, score={:.4}", &r.uuid[..8.min(r.uuid.len())], r.score);
    }

    // 3. Compare
    assert!(!pipe_results.is_empty(), "pipeline should find results");
    assert_same_uuids(&pipe_results, &cat_response.results, "Full hybrid");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Edge cases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn generic_bm25_pipeline_no_results() {
    let mut catalog = setup_simple_catalog(4);
    catalog
        .ingest_entities("Product", vec![
            make_product("Test", "some description here", "some details", 10.0),
        ])
        
        .unwrap();

    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (services, _) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "xyzzy zyxwv qwerty",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("bm25", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    assert_eq!(pipe_results.len(), 0, "nonsense query should return 0 results");
}

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_vector_pipeline_with_limit() {
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
    catalog.ingest_entities("Product", test_products()).unwrap();

    let embedder: Arc<dyn Embedder> = MINILM.clone();
    let (services, _) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(VectorSearchNode::new("vector", 1))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "vector", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("vector", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let pipe_results = extract_results(&output, "resolve");

    assert_eq!(pipe_results.len(), 1, "limit=1 should return exactly 1 result");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Report verification
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_hybrid_pipeline_with_report() {
    let dim = MINILM.dim();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let config = make_empty_config(dim);
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(dim)), config);
    catalog.set_embedder(MINILM.clone());
    catalog.initialize().unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();

    let embedder: Arc<dyn Embedder> = MINILM.clone();
    let (services, _) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "programming language",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(VectorSearchNode::new("vector", 10))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10))).unwrap();
    graph.add_node(Box::new(FuseResultsNode::new("fuse"))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "vector", "query").unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("vector", "results", "fuse", "vector").unwrap();
    graph.connect("bm25", "results", "fuse", "bm25").unwrap();
    graph.connect("fuse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let (output, report) = runtime.execute_with_report(&mut graph).unwrap();

    // Verify report structure
    assert!(
        matches!(report.status, ExecutionStatus::Completed),
        "status should be Completed, got: {:?}",
        report.status
    );
    assert_eq!(report.nodes.len(), 5, "should have 5 nodes in report");
    eprintln!("Report: status={:?}, duration={}ms", report.status, report.total_duration_ms);
    for node in &report.nodes {
        eprintln!("  node: {}, duration={}ms, status={:?}", node.name, node.duration_ms, node.status);
    }
    for edge in &report.edges {
        eprintln!(
            "  edge: {}:{} → {}:{} = {}",
            edge.from_node, edge.from_port, edge.to_node, edge.to_port, edge.value_summary
        );
    }

    // Verify edges count: 6 connections
    assert_eq!(report.edges.len(), 6, "should have 6 edges");

    // Verify results exist
    let pipe_results = extract_results(&output, "resolve");
    assert!(!pipe_results.is_empty(), "pipeline should produce results");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Fusion N-aire étiquetée : deux branches BM25 sur deux champs, port `signals`
// ═══════════════════════════════════════════════════════════════════════════════

/// Deux branches BM25 (`description` et `details`) en fan-in sur `fuse.signals`.
/// La requête « Rust pandas » (mode split) fait matcher le Rust Book sur la
/// description et le Python Cookbook sur les détails : les poids décident de
/// l'ordre. C'est le « boost de champ » sans pondération dans le moteur.
#[cfg(feature = "burn-embedder")]
fn run_two_field_fusion(weights: &[(&str, f64)]) -> Vec<UnifiedResult> {
    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (services, _cat) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "Rust pandas",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(
        BM25SearchNode::new("desc", 10).with_fields(vec!["description".into()]).with_mode(BM25Mode::ContainsSplit),
    )).unwrap();
    graph.add_node(Box::new(
        BM25SearchNode::new("det", 10).with_fields(vec!["details".into()]).with_mode(BM25Mode::ContainsSplit),
    )).unwrap();
    let mut fuse = FuseResultsNode::new("fuse");
    for (label, w) in weights {
        fuse = fuse.with_weight(*label, *w);
    }
    graph.add_node(Box::new(fuse)).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "desc", "query").unwrap();
    graph.connect("source", "query", "det", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("desc", "results", "fuse", "signals").unwrap();
    graph.connect("det", "results", "fuse", "signals").unwrap();
    graph.connect("fuse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let results = extract_results(&output, "resolve");
    for (i, r) in results.iter().enumerate() {
        let name = r.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str()).unwrap_or("?");
        eprintln!("  [{i}] {name} score={:.4} signal={:?}", r.score, r.signal);
    }
    results
}

#[cfg(feature = "burn-embedder")]
fn name_of(r: &UnifiedResult) -> String {
    r.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str()).unwrap_or("?").to_string()
}

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_two_field_branches_weights_decide_order() {
    eprintln!("[desc:1 det:0]");
    let desc_first = run_two_field_fusion(&[("desc", 1.0), ("det", 0.0)]);
    eprintln!("[desc:0 det:1]");
    let det_first = run_two_field_fusion(&[("desc", 0.0), ("det", 1.0)]);

    assert_eq!(desc_first.len(), 2, "one hit per branch");
    assert_eq!(name_of(&desc_first[0]), "Rust Book", "description branch wins");
    assert_eq!(name_of(&det_first[0]), "Python Cookbook", "details branch wins");
    // La fusion garde la provenance : un résultat sort en disant quelle
    // branche l'a trouvé, pas le nom du nœud qui les a mêlées (27 août 2026).
    assert!(desc_first.iter().all(|r| matches!(r.signal.as_deref(), Some("desc") | Some("det") | Some("desc+det"))), "{:?}", desc_first.iter().map(|r| r.signal.clone()).collect::<Vec<_>>());
}

/// `fields` doit nommer un champ indexé : l'erreur est explicite, pas un
/// résultat vide silencieux.
#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_bm25_unknown_field_is_an_error() {
    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (services, _cat) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "Rust",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10).with_fields(vec!["price".into()]))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let err = runtime.execute(&mut graph).unwrap_err();
    assert!(err.contains("'price' is not indexed"), "{err}");
}

// ═══════════════════════════════════════════════════════════════════════════════
// RerankNode : remplacement (après fusion) et mélange (en boost dans la fusion)
// ═══════════════════════════════════════════════════════════════════════════════

/// Graphe BM25 (split) → [rerank] → resolve, sur « Rust Python French » qui
/// retrouve les trois produits. `reranker` = None donne l'ordre BM25 de
/// référence ; sinon un reranker à préférence contrôlée.
#[cfg(feature = "burn-embedder")]
fn run_bm25_then_rerank(reranker: Option<Arc<dyn Reranker>>, candidates: usize) -> Vec<UnifiedResult> {
    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (mut services, _cat) = build_services(catalog, embedder, None, None);
    if let Some(rk) = reranker {
        services.register::<Arc<dyn Reranker>>("reranker", rk);
    }

    let mut graph = DataflowGraph::new();
    // **Le pool a un plancher** — au moins `limit + offset` — pour que la
    // pagination ne coupe pas dans des résultats que le cross-encoder n'a pas
    // vus. Ici on éprouve le pool lui-même : la page vaut le pool.
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "Rust Python French",
        SearchOptions { consistency: Consistency::Immediate, limit: candidates, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10).with_mode(BM25Mode::ContainsSplit))).unwrap();
    graph.add_node(Box::new(RerankNode::new("rerank").with_candidates(candidates))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "rerank", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("bm25", "results", "rerank", "results").unwrap();
    graph.connect("rerank", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    extract_results(&output, "resolve")
}

/// Reranker qui ne veut qu'une chose : `favourite` en tête.
#[cfg(feature = "burn-embedder")]
fn prefers(favourite: &str) -> Arc<dyn Reranker> {
    let fav = favourite.to_lowercase();
    Arc::new(rag3weaver::reranker::CallbackReranker::new("prefers", move |_q, passages| {
        Ok(passages.iter().map(|p| if p.to_lowercase().contains(&fav) { 1.0 } else { 0.0 }).collect())
    }))
}

/// Mot du passage qui identifie chaque produit (le chunk retrouvé porte la
/// description ou les détails, pas le nom).
#[cfg(feature = "burn-embedder")]
fn passage_key(name: &str) -> &'static str {
    match name {
        "Rust Book" => "rust",
        "Python Cookbook" => "python",
        "French Chef Knife" => "french",
        other => panic!("unexpected product {other}"),
    }
}

#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_rerank_replaces_head_and_keeps_tail() {
    // Sans reranker : avertissement, ordre BM25 conservé — c'est la référence.
    let reference = run_bm25_then_rerank(None, 2);
    let ref_names: Vec<String> = reference.iter().map(name_of).collect();
    eprintln!("[bm25 order] {ref_names:?}");
    assert_eq!(reference.len(), 3, "each word matches one product");

    // Le reranker préfère le DEUXIÈME de la référence : dans un pool de 2 il
    // passe premier ; le troisième, hors pool, ne bouge pas même si on le
    // préférait.
    let reranked = run_bm25_then_rerank(Some(prefers(passage_key(&ref_names[1]))), 2);
    let names: Vec<String> = reranked.iter().map(name_of).collect();
    eprintln!("[reranked]   {names:?}");
    assert_eq!(names[0], ref_names[1], "preferred candidate moves to the top");
    assert_eq!(names[1], ref_names[0]);
    assert_eq!(names[2], ref_names[2], "tail untouched");
    assert!((reranked[0].score - 1.0).abs() < 1e-6 && reranked[1].score.abs() < 1e-6, "head scores are the reranker's");
    assert!(reranked.iter().all(|r| r.signal.as_deref() == Some("rerank")));

    // Préférer le troisième ne change rien : il est hors pool.
    let untouched = run_bm25_then_rerank(Some(prefers(passage_key(&ref_names[2]))), 2);
    assert_eq!(untouched.iter().map(name_of).collect::<Vec<_>>(), ref_names, "out-of-pool preference is ignored");
}

/// Le même reranker branché en `boost` dans une fusion : il **module** l'ordre
/// BM25 au lieu de le remplacer. Le dernier de la référence, boosté, passe
/// devant ; les scores restent ceux de la fusion, modulés.
#[cfg(feature = "burn-embedder")]
#[test]
#[ignore]
fn generic_rerank_as_boost_signal_inside_fusion() {
    let reference = run_bm25_then_rerank(None, 3);
    let ref_names: Vec<String> = reference.iter().map(name_of).collect();
    let favourite = passage_key(&ref_names[2]);

    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (mut services, _cat) = build_services(catalog, embedder, None, None);
    services.register::<Arc<dyn Reranker>>("reranker", prefers(favourite));

    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "Rust Python French",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10).with_mode(BM25Mode::ContainsSplit))).unwrap();
    graph.add_node(Box::new(RerankNode::new("rerank").with_candidates(3))).unwrap();
    // Un seul signal de fusion (bm25, scores bruts) + le rerank en boost.
    graph.add_node(Box::new(FuseResultsNode::new("fuse").with_boost("rerank").with_weight("rerank", 5.0))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "rerank", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("bm25", "results", "rerank", "results").unwrap();
    graph.connect("bm25", "results", "fuse", "bm25").unwrap();
    graph.connect("rerank", "results", "fuse", "signals").unwrap();
    graph.connect("fuse", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let fused = extract_results(&output, "resolve");
    let names: Vec<String> = fused.iter().map(name_of).collect();
    eprintln!("[bm25 order] {ref_names:?}\n[boosted]    {names:?}");
    assert_eq!(fused.len(), 3);
    assert_eq!(names[0], ref_names[2], "boosted last-of-reference comes first");
    // Les deux autres gardent l'ordre BM25 : le boost ne les a pas touchés
    // (score × (1 + 5 × 0)).
    assert_eq!(&names[1..], &ref_names[..2]);
    assert!(fused.iter().all(|r| r.signal.is_some()), "chaque résultat garde sa provenance");
}


/// **Le filtre hérité descend aussi.** `SearchOptions.filters` — le
/// `HashMap` d'avant `filter_condition` — était ignoré sur le chemin
/// composable : seule la condition structurée était lue. Même précédence que
/// le monolithe depuis le 6 septembre 2026 (B5) : le précis l'emporte sur le
/// grossier, et le grossier n'est plus jeté.
#[test]
#[ignore]
fn le_filtre_herite_descend_sur_le_chemin_composable() {
    use rag3weaver::filter::{FilterOp, FilterValue};
    use rag3weaver::connection::CypherValue;

    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (services, _cat) = build_services(catalog, embedder, None, None);

    let mut filters = HashMap::new();
    filters.insert(
        "price".to_string(),
        FilterValue::Ops(vec![FilterOp::Lt(CypherValue::Float(100.0))]),
    );
    let mut graph = DataflowGraph::new();
    graph.add_node(Box::new(SearchSourceNode::new(
        "source", "Product", "Rust Python French",
        SearchOptions { consistency: Consistency::Immediate, filters, ..Default::default() },
    ))).unwrap();
    graph.add_node(Box::new(BM25SearchNode::new("bm25", 10).with_mode(BM25Mode::ContainsSplit))).unwrap();
    graph.add_node(Box::new(ResolveParentNode::new("resolve"))).unwrap();
    graph.connect("source", "query", "bm25", "query").unwrap();
    graph.connect("source", "query", "resolve", "query").unwrap();
    graph.connect("bm25", "results", "resolve", "results").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let resultats = extract_results(&output, "resolve");
    let noms: Vec<String> = resultats
        .iter()
        .map(|r| r.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str()).unwrap_or("?").to_string())
        .collect();
    eprintln!("[filtre hérité] {noms:?}");
    assert_eq!(resultats.len(), 2, "le couteau à 129,99 doit être filtré : {noms:?}");
    assert!(noms.iter().all(|n| !n.contains("Knife")), "{noms:?}");
}



/// **Le port d'entrée de la source (pas B du doc 02).** Câblé, un
/// `QueryPayload` d'amont remplace la fiche — c'est ce qui rendra le
/// sous-graphe de recherche branchable derrière un nœud qui calcule sa
/// requête. La fiche reste le défaut du graphe nu.
#[test]
#[ignore]
fn la_source_prend_sa_requete_au_port_quand_il_est_cable() {
    let mut catalog = setup_simple_catalog(4);
    catalog.ingest_entities("Product", test_products()).unwrap();
    let embedder: Arc<dyn Embedder> = Arc::new(MockEmbedder::new(4));
    let (services, _cat_arc) = build_services(catalog, embedder, None, None);

    let mut graph = DataflowGraph::new();
    graph
        .add_node(Box::new(KBQuerySourceNode::new(
            "Product",
            "programming language",
            &SearchOptions { consistency: Consistency::Immediate, signals: Some(SearchSignals::BM25), ..Default::default() },
        )))
        .unwrap();
    graph
        .add_node(Box::new(SearchSourceNode::new(
            "source", "Product", "requete-de-la-fiche", SearchOptions::default(),
        )))
        .unwrap();
    graph.connect("query_source", "query", "source", "query").unwrap();

    let runtime = DataflowRuntime::with_services(100, services);
    let output = runtime.execute(&mut graph).unwrap();
    let qp = output
        .get("source", "query")
        .and_then(|v| v.downcast::<QueryPayload>())
        .cloned()
        .expect("le payload sortant de la source");
    assert_eq!(qp.query, "programming language", "la requête vient du port, pas de la fiche");
    assert_eq!(qp.options.signals, Some(SearchSignals::BM25), "les options aussi");
    assert!(qp.target.is_some(), "la cible est résolue par la source");
}


/// **La pondération par champ, de bout en bout** (pas C du 2 octobre 2026) :
/// l'entité déclare « les gadgets pèsent moins », le nœud `weigh` posé
/// d'office dans `search_base` l'applique sans qu'aucun graphe ait à le
/// savoir ; et l'appelant la renverse — l'échelle, en production.
#[test]
#[ignore]
fn entity_weights_reorder_and_caller_overrides() {
    use rag3weaver::search::FieldWeight;
    use std::sync::Mutex;
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), make_empty_config(4));
    catalog.initialize().unwrap();

    let mut fields = HashMap::new();
    fields.insert("name".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("description".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    fields.insert("kind".into(), SimpleFieldDef { field_type: FieldType::String, ..Default::default() });
    let config = EntityConfig {
        fields,
        signals: SearchSignals::BM25,
        return_fields: Some(vec!["kind".into(), "name".into()]),
        field_weights: vec![FieldWeight {
            field: "kind".into(),
            weights: [("gadget".to_string(), 0.1)].into_iter().collect(),
            default: 1.0,
        }],
        ..Default::default()
    };
    catalog.register_entity("Gizmo", config).unwrap();
    let doc = |nom: &str, kind: &str| {
        let mut m = BTreeMap::new();
        m.insert("name".to_string(), CypherValue::String(nom.into()));
        m.insert("description".to_string(), CypherValue::String("a shiny widget for measuring widgets".into()));
        m.insert("kind".to_string(), CypherValue::String(kind.into()));
        m
    };
    catalog.ingest_entities("Gizmo", vec![doc("Alpha", "gadget"), doc("Beta", "tool")]).unwrap();
    let cat = Arc::new(Mutex::new(catalog));
    let top_names = |options: SearchOptions| -> Vec<String> {
        Catalog::rechercher(&cat, "Gizmo", "shiny widget", options)
            .unwrap()
            .results
            .iter()
            .filter_map(|x| x.data.as_ref()?.get("name")?.as_str().map(str::to_string))
            .collect()
    };

    // Deux textes identiques : seul le poids du genre départage.
    let options = SearchOptions { consistency: Consistency::Immediate, signals: Some(SearchSignals::BM25), ..Default::default() };
    let names = top_names(options.clone());
    assert_eq!(names.first().map(String::as_str), Some("Beta"), "le gadget déclaré pèse moins : {names:?}");
    assert_eq!(names.len(), 2, "un poids, jamais un filtre : {names:?}");

    // L'appelant renverse la déclaration : les outils pèsent moins.
    let mut reversed = options;
    reversed.field_weights = vec![FieldWeight {
        field: "kind".into(),
        weights: [("tool".to_string(), 0.05)].into_iter().collect(),
        default: 1.0,
    }];
    let names = top_names(reversed);
    assert_eq!(names.first().map(String::as_str), Some("Alpha"), "l'appelant renverse : {names:?}");
}

// ═══ Le lanceur : un seul chemin de recherche ═══════════════════════════════

/// **Le lanceur tient la page et les comptes.** Héritier de B13 : depuis le
/// 18 septembre 2026 le monolithe `Catalog::search` n'existe plus, et la
/// moitié « référence » de l'équivalence est partie avec lui. Restent les
/// invariants qu'elle prouvait : sur BM25 seul, sur le vecteur seul, sur
/// l'hybride, la page respecte la limite, les résultats sont enrichis, la
/// méta porte les comptes des signaux — et `offset` n'est pas inerte.
#[test]
#[ignore]
fn le_lanceur_tient_la_page_et_les_comptes() {
    use std::sync::Mutex;
    // `HashEmbedder`, pas `MockEmbedder` : le second rend des vecteurs nuls,
    // et l'ordre vectoriel de deux appels n'aurait rien de comparable.
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let mut catalog = Catalog::new(
        boxed,
        Box::new(rag3weaver::embedder::HashEmbedder::new(4)),
        make_empty_config(4),
    );
    catalog.initialize().unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();

    for signals in [SearchSignals::BM25, SearchSignals::VECTOR, SearchSignals::HYBRID] {
        let options = SearchOptions {
            consistency: Consistency::Immediate,
            signals: Some(signals),
            limit: 2,
            ..Default::default()
        };
        let cat = Arc::new(Mutex::new(catalog));
        let reponse = Catalog::rechercher(&cat, "Product", "programming language", options).unwrap();
        eprintln!(
            "[lanceur {signals:?}] {} résultats, bm25={} vecteur={} avertissements={:?}",
            reponse.results.len(), reponse.meta.bm25_count,
            reponse.meta.vector_count, reponse.meta.warnings,
        );
        assert!(
            !reponse.results.is_empty() && reponse.results.len() <= 2,
            "{signals:?} : la page respecte la limite : {} résultats", reponse.results.len()
        );
        if signals.bm25() {
            assert!(reponse.meta.bm25_count > 0, "{signals:?} : le plein texte a compté");
        }
        if signals.vector() {
            assert!(reponse.meta.vector_count > 0, "{signals:?} : le vecteur a compté");
        }
        assert!(reponse.results.iter().all(|r| r.data.is_some()), "{signals:?} : enrichis");
        catalog = Arc::try_unwrap(cat).ok().expect("le graphe ne retient pas le catalogue").into_inner().unwrap();
    }

    // La page : `offset` n'est plus inerte.
    let cat = Arc::new(Mutex::new(catalog));
    let tout = Catalog::rechercher(&cat, "Product", "Rust Python French", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        limit: 10,
        ..Default::default()
    }).unwrap();
    let page2 = Catalog::rechercher(&cat, "Product", "Rust Python French", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        bm25_mode: BM25Mode::ContainsSplit,
        limit: 1,
        offset: 1,
        ..Default::default()
    }).unwrap();
    assert_eq!(tout.results.len(), 3);
    assert_eq!(page2.results.len(), 1);
    assert_eq!(page2.results[0].uuid, tout.results[1].uuid, "la seconde page est le deuxième de la liste");
    assert_eq!(page2.meta.fused_count, 3, "la méta dit combien il y avait avant la page");
}

/// **Les contrats du 18 septembre** : ce que le monolithe donnait et que le
/// lanceur taisait — `Detailed` de bout en bout (le `result_mode` de la
/// requête atteint les nœuds), les diagnostics BM25 par hit, l'erreur typée
/// d'une cible inconnue (résolue avant de monter le graphe), et
/// `meta.signals` = les signaux **demandés**, même quand l'un d'eux se tait.
#[test]
#[ignore]
fn le_lanceur_honore_detailed_diagnostics_et_contrats() {
    use std::sync::Mutex;
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let mut catalog = Catalog::new(
        boxed,
        Box::new(rag3weaver::embedder::HashEmbedder::new(4)),
        make_empty_config(4),
    );
    catalog.initialize().unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.ingest_entities("Product", test_products()).unwrap();
    let cat = Arc::new(Mutex::new(catalog));

    // 1. `Detailed` atteint les nœuds : les chunks attribués sont portés.
    let detailed = Catalog::rechercher(&cat, "Product", "programming language", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        result_mode: rag3weaver::search::ResultMode::Detailed,
        limit: 3,
        ..Default::default()
    }).unwrap();
    assert!(!detailed.results.is_empty());
    assert!(
        detailed.results.iter().any(|r| r.chunks.as_ref().is_some_and(|c| !c.is_empty())),
        "Detailed doit porter les chunks attribués ; reçus : {:?}",
        detailed.results.iter().map(|r| r.chunks.as_ref().map(|c| c.len())).collect::<Vec<_>>()
    );

    // 2. Les diagnostics BM25 remontent par la méta du nœud.
    let diag = Catalog::rechercher(&cat, "Product", "programming language", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        diagnostics: true,
        ..Default::default()
    }).unwrap();
    let d = diag.meta.diagnostics.as_ref().expect("diagnostics demandés");
    assert!(!d.bm25_hits.is_empty(), "bm25_hits peuplés par le nœud, plus passés sous silence");

    // 3. Cible inconnue : l'erreur typée du monolithe, pas un DbError habillé.
    let err = Catalog::rechercher(&cat, "Nowhere", "x", SearchOptions::default()).unwrap_err();
    assert!(
        matches!(err, rag3weaver::CatalogError::UnknownKB(_) | rag3weaver::CatalogError::UnknownEntity(_)),
        "erreur typée attendue, reçu : {err:?}"
    );

    // 4. `meta.signals` = demandés — même quand un signal rend zéro.
    let hybride = Catalog::rechercher(&cat, "Product", "zzzz-introuvable", SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::HYBRID),
        ..Default::default()
    }).unwrap();
    assert_eq!(hybride.meta.signals, SearchSignals::HYBRID, "les signaux demandés, pas ceux qui ont parlé");
}

/// Le catalogue des produits, en dette de vecteurs (plein texte seul), avec un
/// embarqueur qui ne répond pas dans le délai pour tout lot contenant `muet`.
/// Rend le catalogue et le nombre d'appels qui ont contenu le marqueur.
fn catalogue_avec_un_muet(muet: &'static str) -> (Catalog, Arc<std::sync::atomic::AtomicUsize>) {
    use rag3weaver::disponibilite::Disponibilites as D;
    use rag3weaver::embedder::{CallbackEmbedder, EmbedError};
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let appels = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let compte = appels.clone();
    let embarqueur = CallbackEmbedder::new(4, move |textes: &[String]| -> Result<Vec<Vec<f32>>, EmbedError> {
        if textes.iter().any(|t| t.contains(muet)) {
            compte.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            return Err(EmbedError::Timeout);
        }
        Ok(textes.iter().map(|t| vec![t.len() as f32, 1.0, 0.5, 0.25]).collect())
    });
    let mut catalog = Catalog::new(boxed, Box::new(embarqueur), make_empty_config(4));
    catalog.initialize().unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.ingest_entities_jusqu_a("Product", test_products(), D::RECHERCHE_TEXTE).expect("ingestion sans GPU");
    (catalog, appels)
}

/// **Un morceau muet n'arrête pas l'embarquement** (ticket du délai
/// d'embarquement, 11 octobre 2026) : le lot qui le contient expire, il est
/// rejoué morceau par morceau, les autres s'écrivent, le muet est nommé et
/// relevé dans le suivi, **exclu** des réclamations de ce run (la passe
/// suivante ne le retente pas) et **toujours dû** (un run neuf le retente).
#[test]
#[ignore]
fn un_morceau_muet_est_nomme_exclu_du_run_et_reste_du() {
    use rag3weaver::dataflow::record_nodes::EmbedWatch;
    use rag3weaver::disponibilite::Disponibilites as D;
    let (mut catalog, appels) = catalogue_avec_un_muet("zero-cost abstractions");
    let dus_avant = catalog.index_progress().unwrap().dense_missing();
    assert!(dus_avant >= 3, "la dette des trois produits : {dus_avant}");

    let run = Arc::new(EmbedWatch::default());
    let n = catalog.embarquer_le_retard_suivi(D::TOUT, 512, None, Some(&run)).expect("rattrapage");
    let muets = run.silent.lock().unwrap().clone();
    eprintln!("[muet] passe 1 : {n} réclamés, muets {muets:?}, appels muets {}", appels.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(muets.len(), 1, "un seul morceau muet : {muets:?}");
    assert!(muets[0].1.contains("Rust Book"), "le muet est nommé par son parent : {muets:?}");
    assert_eq!(catalog.index_progress().unwrap().dense_missing(), 1, "les autres sont écrits, le muet reste dû");
    let appels_passe_1 = appels.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(appels_passe_1, 2, "le lot qui expire, puis le morceau seul");

    // La passe suivante du même run ne le retente pas.
    let n = catalog.embarquer_le_retard_suivi(D::TOUT, 512, None, Some(&run)).expect("rattrapage");
    assert_eq!(n, 0, "rien à réclamer : le muet est exclu de ce run");
    assert_eq!(appels.load(std::sync::atomic::Ordering::SeqCst), appels_passe_1, "pas un appel de plus pour le muet");

    // Un run neuf le retente : il est toujours dû.
    let neuf = Arc::new(EmbedWatch::default());
    catalog.embarquer_le_retard_suivi(D::TOUT, 512, None, Some(&neuf)).expect("rattrapage");
    assert_eq!(neuf.silent.lock().unwrap().len(), 1, "un run neuf le retente et le relève de nouveau");
    assert!(run.errors.lock().unwrap().is_empty(), "un muet n'est pas un échec de la passe");
}

/// **Un échec d'embarquement n'est plus avalé** (ticket du même jour) :
/// toutes les tables échouent, la passe rend zéro comme avant, et le suivi
/// relève l'échec — le journal de l'indexation le dira au lieu de « terminée ».
#[test]
#[ignore]
fn un_echec_d_embarquement_est_releve_et_non_avale() {
    use rag3weaver::dataflow::record_nodes::EmbedWatch;
    use rag3weaver::disponibilite::Disponibilites as D;
    use rag3weaver::embedder::{CallbackEmbedder, EmbedError};
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let en_panne = CallbackEmbedder::new(4, |_: &[String]| -> Result<Vec<Vec<f32>>, EmbedError> {
        Err(EmbedError::ProviderError("le service a refusé (503)".into()))
    });
    let mut catalog = Catalog::new(boxed, Box::new(en_panne), make_empty_config(4));
    catalog.initialize().unwrap();
    catalog.register_entity("Product", make_product_config()).unwrap();
    catalog.ingest_entities_jusqu_a("Product", test_products(), D::RECHERCHE_TEXTE).expect("ingestion sans GPU");

    let run = Arc::new(EmbedWatch::default());
    let n = catalog.embarquer_le_retard_suivi(D::TOUT, 512, None, Some(&run)).expect("la passe rend la main");
    let echecs = run.errors.lock().unwrap().clone();
    eprintln!("[panne] {n} rattrapés, échecs {echecs:?}");
    assert_eq!(n, 0);
    assert_eq!(echecs.len(), 1, "l'échec de la table est relevé : {echecs:?}");
    assert!(echecs[0].contains("503"), "il dit la cause : {echecs:?}");
    assert!(catalog.index_progress().unwrap().dense_missing() >= 3, "la dette reste, entière");
}

/// **Un morceau sans texte n'est jamais dû** (le gel de code-lourd, 11 octobre
/// 2026) : l'embarquement le sautait sans le marquer, la réclamation le
/// reprenait à chaque passe, et une passe faite de morceaux sans texte ne
/// progressait plus jamais — `run_index` bouclait, aucun appel au service, le
/// catalogue repris à chaque tour. Les morceaux vidés ici ne sont plus comptés
/// ni réclamés : deux passes rendent zéro, la dette est nulle.
#[test]
#[ignore]
fn un_morceau_sans_texte_n_est_jamais_du() {
    use rag3weaver::dataflow::record_nodes::EmbedWatch;
    use rag3weaver::disponibilite::Disponibilites as D;
    let (mut catalog, _) = catalogue_avec_un_muet("rien ne sera muet ici");
    assert!(catalog.index_progress().unwrap().dense_missing() >= 3, "la dette des trois produits");
    catalog.execute_raw("MATCH (n:Product_Chunk) SET n._text = ''").expect("vider les textes");
    assert_eq!(catalog.index_progress().unwrap().dense_missing(), 0, "un morceau sans texte n'est pas dû");
    let vides = catalog.chunks_sans_texte();
    assert_eq!(vides.iter().map(|(_, n)| n).sum::<usize>(), 6, "mais il est compté et dit : {vides:?}");
    let run = Arc::new(EmbedWatch::default());
    for passe in 1..=2 {
        let n = catalog.embarquer_le_retard_suivi(D::TOUT, 512, None, Some(&run)).expect("rattrapage");
        assert_eq!(n, 0, "passe {passe} : rien à réclamer — avant, les mêmes trois morceaux revenaient à chaque passe");
    }
}

/// **Les morceaux sans texte se réécrivent depuis leur entité** (ticket des
/// morceaux de code écrits sans texte) : la montre ne les reverrait jamais,
/// le fichier n'ayant pas changé. Leurs entités sont marquées à redécouper,
/// la passe de rattrapage du découpage les réécrit : le texte revient, aucun
/// vide ne reste, et les morceaux neufs doivent leurs vecteurs.
#[test]
#[ignore]
fn les_morceaux_sans_texte_se_reecrivent_depuis_leur_entite() {
    let (mut catalog, _) = catalogue_avec_un_muet("rien ne sera muet ici");
    let pleins = |c: &Catalog| -> i64 {
        c.execute_raw("MATCH (n:Product_Chunk) WHERE n._text IS NOT NULL AND n._text <> '' RETURN count(n)")
            .unwrap()
            .rows[0][0]
            .as_i64()
            .unwrap()
    };
    let avant = pleins(&catalog);
    assert!(avant >= 3, "des morceaux pleins au départ : {avant}");
    catalog.execute_raw("MATCH (n:Product_Chunk) SET n._text = ''").expect("vider les textes");
    assert_eq!(pleins(&catalog), 0);
    assert_eq!(catalog.chunks_sans_texte().iter().map(|(_, n)| *n as i64).sum::<i64>(), avant);

    let (marquees, redecoupees) = catalog.reecrire_les_morceaux_sans_texte().expect("réécrire");
    eprintln!("[réécriture] {marquees} entités marquées, {redecoupees} redécoupées");
    assert_eq!(marquees, 3, "les trois produits");
    assert_eq!(redecoupees, 3);
    assert!(catalog.chunks_sans_texte().is_empty(), "aucun vide ne reste : {:?}", catalog.chunks_sans_texte());
    assert_eq!(pleins(&catalog), avant, "le texte est revenu");
    assert_eq!(catalog.index_progress().unwrap().dense_missing() as i64, avant, "les morceaux neufs doivent leurs vecteurs");
}

/// **Un parent dont un morceau a perdu son texte n'est pas complet** (ticket
/// des morceaux de code écrits sans texte) : le court-circuit de l'inchangé
/// le jugeait complet sur ses marqueurs et son arête, et une réingestion
/// identique gardait le vide. Désormais la complétude compte aussi les
/// morceaux qui ont un texte : la même réingestion le redécoupe, et le texte
/// revient.
#[test]
#[ignore]
fn une_reingestion_identique_ne_garde_pas_un_morceau_sans_texte() {
    use rag3weaver::disponibilite::Disponibilites as D;
    let (mut catalog, _) = catalogue_avec_un_muet("rien ne sera muet ici");
    catalog.embarquer_le_retard_suivi(D::TOUT, 512, None, None).expect("tout embarquer");
    assert_eq!(catalog.index_progress().unwrap().dense_missing(), 0, "plus rien de dû");
    let pleins = |c: &Catalog| -> i64 {
        c.execute_raw("MATCH (n:Product_Chunk) WHERE n._text IS NOT NULL AND n._text <> '' RETURN count(n)")
            .unwrap()
            .rows[0][0]
            .as_i64()
            .unwrap()
    };
    let avant = pleins(&catalog);
    assert!(avant >= 3, "des morceaux pleins au départ : {avant}");

    // La réingestion identique d'un index complet ne réécrit rien.
    catalog.ingest_entities_jusqu_a("Product", test_products(), D::TOUT).expect("réingestion identique");
    assert_eq!(pleins(&catalog), avant);

    catalog.execute_raw("MATCH (n:Product_Chunk) SET n._text = ''").expect("vider les textes");
    assert_eq!(pleins(&catalog), 0);
    catalog.ingest_entities_jusqu_a("Product", test_products(), D::RECHERCHE_TEXTE).expect("réingestion identique");
    assert!(catalog.chunks_sans_texte().is_empty(), "aucun vide ne survit à la réingestion : {:?}", catalog.chunks_sans_texte());
    assert_eq!(pleins(&catalog), avant, "le texte est revenu");
}

//! E2E — reranking du pool fusionné (doc 29, chantier 3), avec un reranker
//! mock (recouvrement lexical) : l'ordre, le pool, la pagination et les
//! avertissements sont le contrat ; le cross-encoder burn a sa propre suite.
//!
//! Run: cargo test --features rag3db-native --test e2e_rerank -- --ignored --test-threads=1
#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rag3weaver::config::FieldType;
use rag3weaver::connection::{CypherValue, DbConnection};
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::{BM25Mode, Consistency, RerankOptions, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, CatalogConfig, EntityConfig, MockReranker, Rag3dbConnection, SimpleFieldDef};

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap().to_string_lossy().to_string()
    })
}

fn catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn DbConnection> = Box::new(conn);
    let path = format!("{}/extension/vector/build/libvector.rag3db_extension", rag3db_root());
    boxed.execute(&format!("LOAD EXTENSION '{path}'")).expect("load vector");
    let config = CatalogConfig { name: Some("rerank-test".into()), embedding_dim: 4, ..Default::default() };
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config);
    catalog.initialize().unwrap();
    let mut fields = HashMap::new();
    fields.insert("name".into(), SimpleFieldDef { field_type: FieldType::String, is_title: true, ..Default::default() });
    fields.insert("body".into(), SimpleFieldDef { field_type: FieldType::Text, is_content: true, ..Default::default() });
    catalog.register_entity("Note", EntityConfig { fields, signals: SearchSignals::BM25, ..Default::default() }).unwrap();
    catalog
}

fn note(name: &str, body: &str) -> BTreeMap<String, CypherValue> {
    let mut d = BTreeMap::new();
    d.insert("name".into(), CypherValue::String(name.into()));
    d.insert("body".into(), CypherValue::String(body.into()));
    d
}

fn names(catalog: &mut Catalog, q: &str, opts: SearchOptions) -> (Vec<String>, rag3weaver::search::SearchMeta) {
    let resp = catalog.search("Note", q, opts).unwrap();
    let names = resp
        .results
        .iter()
        .filter_map(|r| r.data.as_ref().and_then(|d| d.get("name")).and_then(|v| v.as_str()).map(String::from))
        .collect();
    (names, resp.meta)
}

fn opts(rerank: Option<RerankOptions>) -> SearchOptions {
    SearchOptions {
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        // Requêtes multi-mots : Contains (défaut) cherche la chaîne entière.
        bm25_mode: BM25Mode::ContainsSplit,
        limit: 10,
        rerank,
        ..Default::default()
    }
}

/// Trois notes qui contiennent toutes « scheduler » ; seule une contient aussi
/// « preemption ». BM25 seul les classe par sa loi ; le reranker mock (taux de
/// mots de la requête présents) met la note complète devant.
#[test]
#[ignore]
fn rerank_reorders_the_fused_pool() {
    let mut catalog = catalog();
    catalog.ingest_entities("Note", vec![
        note("A", "scheduler scheduler scheduler scheduler scheduler notes"),
        note("B", "the scheduler handles preemption of tasks"),
        note("C", "scheduler overview"),
    ]).unwrap();

    let (baseline, meta) = names(&mut catalog, "scheduler preemption", opts(None));
    assert_eq!(baseline.len(), 3);
    assert_eq!(meta.reranked_count, 0);

    catalog.set_reranker(Arc::new(MockReranker));
    let (reranked, meta) = names(&mut catalog, "scheduler preemption", opts(Some(RerankOptions::default())));
    assert_eq!(reranked[0], "B", "la note qui couvre les deux termes passe devant : {reranked:?}");
    assert_eq!(reranked.len(), 3);
    assert_eq!(meta.reranked_count, 3, "warnings: {:?}", meta.warnings);
    assert!(meta.warnings.iter().all(|w| !w.contains("rerank")), "{:?}", meta.warnings);
}

/// Sans reranker branché, `rerank` est un avertissement, pas une erreur, et
/// l'ordre de fusion est conservé.
#[test]
#[ignore]
fn rerank_without_reranker_warns_and_keeps_order() {
    let mut catalog = catalog();
    catalog.ingest_entities("Note", vec![note("A", "scheduler"), note("B", "scheduler preemption")]).unwrap();
    let (plain, _) = names(&mut catalog, "scheduler preemption", opts(None));
    let (with, meta) = names(&mut catalog, "scheduler preemption", opts(Some(RerankOptions::default())));
    assert_eq!(plain, with);
    assert_eq!(meta.reranked_count, 0);
    assert!(meta.warnings.iter().any(|w| w.contains("aucun reranker")), "{:?}", meta.warnings);
}

/// Le pool rescoré est borné par `candidates` (au moins `limit + offset`) ;
/// la pagination s'applique après le rerank.
#[test]
#[ignore]
fn rerank_pool_and_pagination() {
    let mut catalog = catalog();
    let mut rows = Vec::new();
    for i in 0..12 {
        rows.push(note(&format!("N{i:02}"), &format!("scheduler note number {i}")));
    }
    rows.push(note("GOLD", "scheduler preemption and priority inheritance"));
    catalog.ingest_entities("Note", rows).unwrap();
    catalog.set_reranker(Arc::new(MockReranker));

    let mut o = opts(Some(RerankOptions { candidates: 50 }));
    o.limit = 2;
    let (page1, meta) = names(&mut catalog, "scheduler preemption priority", o.clone());
    assert_eq!(page1.len(), 2);
    assert_eq!(page1[0], "GOLD");
    assert!(meta.reranked_count >= 13, "pool = tout : {}", meta.reranked_count);

    o.offset = 2;
    let (page2, _) = names(&mut catalog, "scheduler preemption priority", o);
    assert_eq!(page2.len(), 2);
    assert!(!page2.contains(&"GOLD".to_string()) && page2.iter().all(|n| !page1.contains(n)));

    // Pool minimal : limit + offset quand candidates est plus petit.
    let mut small = opts(Some(RerankOptions { candidates: 1 }));
    small.limit = 3;
    let (_, meta) = names(&mut catalog, "scheduler preemption priority", small);
    assert_eq!(meta.reranked_count, 3);
}

//! Where does the time actually go on a tiny corpus?
//!
//! lucivy indexes ~5 ms/doc and searches 50k docs in ~50 ms, so nine documents
//! should cost tens of milliseconds end to end. Our E2E suites spend north of a
//! second per test on exactly that corpus. This measures the two sides so the
//! gap has a number instead of a hunch.
//!
//! ```bash
//! cargo test --features rag3db-native --test e2e_profile_overhead -- --ignored --nocapture
//! ```

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use rag3weaver::config::{CatalogConfig, EntityDef, FieldDef, FieldType, KBConfig};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::{BM25Mode, Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, Rag3dbConnection};

const CORPUS: &[(&str, &str)] = &[
    ("arrow", "let value = foo->bar;"),
    ("underscore", "let value = foo_bar;"),
    ("colons", "let value = foo::bar;"),
    ("spaced", "let value = foo -> bar;"),
    ("brace", "if (ok) { return 1; };"),
    ("cpp", "this module was compiled with c++ and gcc 13"),
    ("generic", "type Shared = std::sync::Arc<Mutex<T>>;"),
    ("emoji", "deploy status: shipped, reviewed by the platform team"),
    ("accents", "DEJA vu, la creme brulee etait trop cuite"),
];

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

// ─── A. lucivy seul ─────────────────────────────────────────────────────────

/// The floor: index nine documents and search them, with no database at all.
#[test]
#[ignore]
fn profile_lucivy_alone() {
    use lucivy_core::blob_store::MemBlobStore;
    use lucivy_core::sharded_handle::{BlobShardStorage, ShardedHandle};
    use rag3weaver::fts_handle::{build_schema_config, fts_index_name, index_document};
    use std::sync::Arc;

    let tmp = std::env::temp_dir().join(format!("rag3w_profile_{}", std::process::id()));

    let t = Instant::now();
    let store = Arc::new(MemBlobStore::new());
    let cfg = build_schema_config(&["body".to_string()], &[], 2).unwrap();
    let storage = BlobShardStorage::new(store.clone(), fts_index_name("Snippet"), &tmp);
    let handle = ShardedHandle::create_with_storage(Box::new(storage), &cfg).unwrap();
    let create_ms = ms(t);

    let t = Instant::now();
    for (i, (_, body)) in CORPUS.iter().enumerate() {
        index_document(&handle, &[("body".to_string(), body.to_string())], i as u64).unwrap();
    }
    let index_ms = ms(t);

    let t = Instant::now();
    handle.commit().unwrap();
    let commit_ms = ms(t);

    let q: lucivy_core::query::QueryConfig = serde_json::from_value(serde_json::json!({
        "type": "contains", "field": "body", "value": "foo->bar", "strict_separators": true
    }))
    .unwrap();

    let t = Instant::now();
    let hits = handle.search_with_docs(&q, 10).unwrap();
    let search_ms = ms(t);

    eprintln!("\n── A. lucivy seul, {} documents ──", CORPUS.len());
    eprintln!("  création index : {create_ms:8.1} ms");
    eprintln!("  indexation     : {index_ms:8.1} ms  ({:.1} ms/doc)", index_ms / CORPUS.len() as f64);
    eprintln!("  commit         : {commit_ms:8.1} ms");
    eprintln!("  recherche      : {search_ms:8.1} ms  ({} hits)", hits.len());
    eprintln!("  TOTAL          : {:8.1} ms", create_ms + index_ms + commit_ms + search_ms);

    handle.close().ok();
    let _ = std::fs::remove_dir_all(&tmp);
}

// ─── B. la chaîne complète ──────────────────────────────────────────────────

fn text_title_for(kb: &str) -> FieldDef {
    FieldDef { field_type: FieldType::Text, title_for: Some(kb.into()), content_for: None, boost: None, default_value: None }
}
fn text_content_for(kb: &str) -> FieldDef {
    FieldDef { field_type: FieldType::Text, title_for: None, content_for: Some(vec![kb.into()]), boost: None, default_value: None }
}

fn make_config() -> CatalogConfig {
    let mut fields = HashMap::new();
    fields.insert("title".into(), text_title_for("kb"));
    fields.insert("body".into(), text_content_for("kb"));
    let mut entities = HashMap::new();
    entities.insert("Snippet".into(), EntityDef { fields, hashsafe: None });
    let mut kbs = HashMap::new();
    kbs.insert("kb".into(), KBConfig { signals: SearchSignals::BM25, ..Default::default() });
    CatalogConfig {
        name: Some("profile".into()),
        entities,
        relations: HashMap::new(),
        knowledge_bases: kbs,
        embedding_dim: 384,
        ..Default::default()
    }
}

fn rag3db_root() -> String {
    std::env::var("RAG3DB_ROOT").unwrap_or_else(|_| {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        std::path::PathBuf::from(&manifest).join("../..").canonicalize().unwrap()
            .to_string_lossy().to_string()
    })
}

/// The same nine documents, through `Catalog`. Every phase timed separately.
#[test]
#[ignore]
fn profile_full_catalog_path() {
    let root = rag3db_root();

    let t = Instant::now();
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    let db_ms = ms(t);

    let t = Instant::now();
    for (name, path) in [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
        ("sparse_vector", format!("{root}/extension/sparse_vector/build/libsparse_vector.rag3db_extension")),
    ] {
        boxed.execute(&format!("LOAD EXTENSION '{path}'"))
            .unwrap_or_else(|e| panic!("load {name}: {e}"));
    }
    let ext_ms = ms(t);

    let t = Instant::now();
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(384)), make_config());
    catalog.initialize().unwrap();
    let init_ms = ms(t);

    let t = Instant::now();
    for (title, body) in CORPUS {
        let mut data = BTreeMap::new();
        data.insert("title".into(), CypherValue::String(title.to_string()));
        data.insert("body".into(), CypherValue::String(body.to_string()));
        catalog.create("Snippet", data).unwrap();
    }
    let create_ms = ms(t);

    let t = Instant::now();
    let drain = catalog.drain();
    let drain_ms = ms(t);
    assert_eq!(drain.failed, 0);

    let opts = SearchOptions {
        bm25_mode: BM25Mode::Symbol,
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        ..Default::default()
    };
    let t = Instant::now();
    let response = catalog.search("kb", "foo->bar", opts.clone()).unwrap();
    let search1_ms = ms(t);

    let t = Instant::now();
    let _ = catalog.search("kb", "c++", opts).unwrap();
    let search2_ms = ms(t);

    let total = db_ms + ext_ms + init_ms + create_ms + drain_ms + search1_ms + search2_ms;
    eprintln!("\n── B. chaîne complète, {} documents ──", CORPUS.len());
    eprintln!("  ouverture DB       : {db_ms:8.1} ms");
    eprintln!("  LOAD EXTENSION x3  : {ext_ms:8.1} ms");
    eprintln!("  Catalog::initialize: {init_ms:8.1} ms");
    eprintln!("  create() x{:<8}: {create_ms:8.1} ms", CORPUS.len());
    eprintln!("  drain()            : {drain_ms:8.1} ms");
    eprintln!("  search #1 (froid)  : {search1_ms:8.1} ms  ({} hits)", response.results.len());
    eprintln!("  search #2 (chaud)  : {search2_ms:8.1} ms");
    eprintln!("  TOTAL              : {total:8.1} ms");
}

/// Is the ~670 ms `commit()` proportional to the corpus, or a fixed wait?
///
/// Release changed nothing (679 ms → 663 ms), which already rules out CPU work.
/// This pins the shape: vary the document count, and commit twice.
#[test]
#[ignore]
fn profile_commit_floor() {
    use lucivy_core::blob_store::MemBlobStore;
    use lucivy_core::sharded_handle::{BlobShardStorage, ShardedHandle};
    use rag3weaver::fts_handle::{build_schema_config, fts_index_name, index_document};
    use std::sync::Arc;

    eprintln!("\n── C. forme du coût de commit ──");
    for n in [1usize, 9, 90, 900] {
        let tmp = std::env::temp_dir()
            .join(format!("rag3w_floor_{}_{n}", std::process::id()));
        let store = Arc::new(MemBlobStore::new());
        let cfg = build_schema_config(&["body".to_string()], &[], 2).unwrap();
        let storage = BlobShardStorage::new(store.clone(), fts_index_name("F"), &tmp);
        let handle = ShardedHandle::create_with_storage(Box::new(storage), &cfg).unwrap();

        let t = Instant::now();
        for i in 0..n {
            let body = format!("document numero {i} avec du contenu foo->bar et du texte");
            index_document(&handle, &[("body".to_string(), body)], i as u64).unwrap();
        }
        let idx = ms(t);

        let t = Instant::now();
        handle.commit().unwrap();
        let c1 = ms(t);

        // Second commit, rien de sale entre les deux.
        let t = Instant::now();
        handle.commit().unwrap();
        let c2 = ms(t);

        eprintln!(
            "  {n:>4} docs : index {idx:7.1} ms · commit#1 {c1:7.1} ms · commit#2 (à vide) {c2:7.1} ms"
        );

        handle.close().ok();
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

// ─── D. le drain : coût fixe ou coût par document ? ─────────────────────────

/// The ~80 ms that remain in a 9-document drain after the engine and the store
/// were fixed: fixed per drain, or per document? Same catalog, same KB, N
/// varied. Per-document cost is what decides whether it matters at scale.
#[test]
#[ignore]
fn profile_drain_scaling() {
    let root = rag3db_root();

    eprintln!("\n── D. drain en fonction de N (BM25 KB, MockEmbedder) ──");
    eprintln!("  {:>5}  {:>9}  {:>9}  {:>9}  {:>9}  {:>8}", "N", "create", "drain", "search", "total", "ms/doc");

    for n in [1usize, 9, 90, 900] {
        let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
        let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
        for path in [
            format!("{root}/extension/vector/build/libvector.rag3db_extension"),
            format!("{root}/extension/lucivy_fts/build/liblucivy_fts.rag3db_extension"),
            format!("{root}/extension/sparse_vector/build/libsparse_vector.rag3db_extension"),
        ] {
            boxed.execute(&format!("LOAD EXTENSION '{path}'")).expect("load extension");
        }
        let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(384)), make_config());
        catalog.initialize().unwrap();

        let t = Instant::now();
        for i in 0..n {
            let mut data = BTreeMap::new();
            data.insert("title".into(), CypherValue::String(format!("snippet {i}")));
            // Varied text so lucivy has real tokens and the chunker real work.
            data.insert(
                "body".into(),
                CypherValue::String(format!(
                    "fn handler_{i}(req: &Request) -> Result<Response, Error> {{ \
                     let value = store.get(\"key_{i}\")?; \
                     if value.len() > {} {{ return Err(Error::TooLarge); }} \
                     Ok(Response::json(value)) }}",
                    i % 97
                )),
            );
            catalog.create("Snippet", data).unwrap();
        }
        let create_ms = ms(t);

        let t = Instant::now();
        let drain = catalog.drain();
        let drain_ms = ms(t);
        assert_eq!(drain.failed, 0, "drain must not fail at N={n}");

        let t = Instant::now();
        let hits = catalog
            .search(
                "kb",
                "Error::TooLarge",
                SearchOptions {
                    bm25_mode: BM25Mode::Symbol,
                    consistency: Consistency::Immediate,
                    signals: Some(SearchSignals::BM25),
                    ..Default::default()
                },
            )
            .unwrap();
        let search_ms = ms(t);
        assert!(!hits.results.is_empty(), "search must find results at N={n}");

        let total = create_ms + drain_ms + search_ms;
        eprintln!(
            "  {n:>5}  {create_ms:>7.1}ms  {drain_ms:>7.1}ms  {search_ms:>7.1}ms  {total:>7.1}ms  {:>8.2}",
            drain_ms / n as f64
        );
    }
}

//! E2E: exact symbol search — separators, operators, emoji, accents.
//!
//! This is the founding differentiator of the project, stated on 6 February 2026:
//! retrieve an arbitrary string of symbols *separators included*. lucivy v3 closed
//! the engine-side gap; these tests check the whole rag3weaver chain actually
//! carries it, and that `BM25Mode::Symbol` discriminates where every other mode
//! deliberately conflates.
//!
//! BM25-only KB, mock embedder: no GPU, no model download, runs in seconds.
//!
//! ```bash
//! cargo test --features rag3db-native --test e2e_symbol_search -- --ignored --test-threads=1
//! ```

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::{CatalogConfig, EntityDef, FieldDef, FieldType, KBConfig};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::{BM25Mode, Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, Rag3dbConnection};

// ─── Fixtures ───────────────────────────────────────────────────────────────

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

fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let root = rag3db_root();
    let extensions = [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
        ("lucivy_fts", format!("{root}/extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")),
        ("sparse_vector", format!("{root}/extension/sparse_vector/build/libsparse_vector.rag3db_extension")),
    ];
    for (name, ext_path) in &extensions {
        if !std::path::Path::new(ext_path).exists() {
            panic!(
                "Extension '{name}' not found at: {ext_path}\n\
                 Run ./run_e2e.sh --build-only first."
            );
        }
        conn.execute(&format!("LOAD EXTENSION '{ext_path}'"))
            .unwrap_or_else(|e| panic!("Failed to load {name} from {ext_path}: {e}"));
    }
}

fn text_title_for(kb: &str) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: Some(kb.to_string()),
        content_for: None,
        boost: None,
        default_value: None,
    }
}

fn text_content_for(kb: &str) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: None,
        content_for: Some(vec![kb.to_string()]),
        boost: None,
        default_value: None,
    }
}

fn make_config() -> CatalogConfig {
    let mut fields = HashMap::new();
    fields.insert("title".into(), text_title_for("kb"));
    fields.insert("body".into(), text_content_for("kb"));

    let mut entities = HashMap::new();
    entities.insert("Snippet".into(), EntityDef { fields, hashsafe: None });

    let mut kbs = HashMap::new();
    kbs.insert(
        "kb".into(),
        KBConfig {
            signals: SearchSignals::BM25,
            ..Default::default()
        },
    );

    CatalogConfig {
        name: Some("symbol-search".into()),
        entities,
        relations: HashMap::new(),
        knowledge_bases: kbs,
        embedding_dim: 384,
        ..Default::default()
    }
}

/// Four near-identical snippets differing **only** by their separators, plus a
/// set of hostile strings. If separators were not honoured byte for byte, the
/// first four would be indistinguishable.
const SNIPPETS: &[(&str, &str)] = &[
    ("arrow",     "let value = foo->bar;"),
    ("underscore", "let value = foo_bar;"),
    ("colons",    "let value = foo::bar;"),
    ("spaced",    "let value = foo -> bar;"),
    ("brace",     "if (ok) { return 1; };"),
    ("cpp",       "this module was compiled with c++ and gcc 13"),
    ("generic",   "type Shared = std::sync::Arc<Mutex<T>>;"),
    ("emoji",     "deploy status: 🚀 shipped, reviewed by 👩‍💻 the platform team"),
    ("accents",   "DÉJÀ vu — la crème brûlée était trop cuite"),
];

fn setup() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());

    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(384)), make_config());
    catalog.initialize().unwrap();

    for (title, body) in SNIPPETS {
        let mut data = BTreeMap::new();
        data.insert("title".into(), CypherValue::String(title.to_string()));
        data.insert("body".into(), CypherValue::String(body.to_string()));
        catalog.create("Snippet", data).unwrap();
    }

    let result = catalog.drain();
    assert_eq!(result.failed, 0, "drain must not fail");
    catalog
}

fn options(mode: BM25Mode) -> SearchOptions {
    SearchOptions {
        bm25_mode: mode,
        consistency: Consistency::Immediate,
        signals: Some(SearchSignals::BM25),
        ..Default::default()
    }
}

/// Titles of every hit, sorted — order is irrelevant here, membership is the point.
fn titles(catalog: &mut Catalog, query: &str, mode: BM25Mode) -> Vec<String> {
    let response = catalog.search("kb", query, options(mode)).unwrap();
    let mut out: Vec<String> = response
        .results
        .iter()
        .filter_map(|r| {
            r.data
                .as_ref()
                .and_then(|d| d.get("_title"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .collect();
    out.sort();
    eprintln!("  {mode:?}  {query:?} -> {out:?}");
    out
}

// ─── Tests ──────────────────────────────────────────────────────────────────

/// The core claim: `foo->bar` retrieves the arrow snippet and nothing else.
#[test]
#[ignore]
fn symbol_matches_separators_byte_for_byte() {
    let mut catalog = setup();
    let hits = titles(&mut catalog, "foo->bar", BM25Mode::Symbol);
    assert_eq!(
        hits,
        vec!["arrow"],
        "strict separators must reject foo_bar, foo::bar and foo -> bar"
    );
}

/// The contrast that gives the previous test its meaning: the relaxed default
/// deliberately treats all four separators as equivalent. Both behaviours are
/// wanted — one for tolerant RAG, one for code.
#[test]
#[ignore]
fn contains_conflates_where_symbol_discriminates() {
    let mut catalog = setup();
    let relaxed = titles(&mut catalog, "foo->bar", BM25Mode::Contains);
    let strict = titles(&mut catalog, "foo->bar", BM25Mode::Symbol);

    assert!(
        relaxed.len() > strict.len(),
        "relaxed mode should match more broadly than Symbol (relaxed={relaxed:?}, strict={strict:?})"
    );
    assert!(relaxed.contains(&"arrow".to_string()));
    assert_eq!(strict, vec!["arrow"]);
}

/// A query made **only** of separators.
#[test]
#[ignore]
fn symbol_matches_pure_punctuation() {
    let mut catalog = setup();
    let hits = titles(&mut catalog, "};", BM25Mode::Symbol);
    assert!(
        hits.contains(&"brace".to_string()),
        "a query made only of punctuation should find the brace snippet"
    );
}

/// `c++` — the string that breaks classic query parsers, and the reason
/// auto-detection of regex was refused back in February.
#[test]
#[ignore]
fn symbol_matches_cpp() {
    let mut catalog = setup();
    let hits = titles(&mut catalog, "c++", BM25Mode::Symbol);
    assert_eq!(hits, vec!["cpp"], "`c++` must be a literal, never a regex");
}

/// A long literal mixing `::`, `<`, `>` — the shape a code agent actually types.
#[test]
#[ignore]
fn symbol_matches_nested_generic() {
    let mut catalog = setup();
    let hits = titles(&mut catalog, "std::sync::Arc<Mutex<T>>", BM25Mode::Symbol);
    assert_eq!(hits, vec!["generic"]);
}

/// Emoji, including a ZWJ sequence (👩‍💻 = woman + ZWJ + computer).
#[test]
#[ignore]
fn symbol_matches_emoji_and_zwj_sequences() {
    let mut catalog = setup();
    assert_eq!(titles(&mut catalog, "🚀", BM25Mode::Symbol), vec!["emoji"]);
    assert_eq!(
        titles(&mut catalog, "👩‍💻", BM25Mode::Symbol),
        vec!["emoji"],
        "ZWJ sequence must survive the whole chain unsplit"
    );
}

/// Accents, where case folding changes the byte length (É → é is 2 bytes both
/// ways, but À/à and the folding path are what broke naive offset maths before).
#[test]
#[ignore]
fn symbol_matches_accented_text() {
    let mut catalog = setup();
    assert_eq!(titles(&mut catalog, "crème brûlée", BM25Mode::Symbol), vec!["accents"]);
}

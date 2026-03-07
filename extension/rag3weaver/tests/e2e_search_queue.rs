//! E2E integration tests: Dataflow search (search_with_strategy).
//!
//! Uses the same Directory + File + HAS_FILE schema as e2e_result_mode.
//!
//! Run with: ./run_e2e.sh --test e2e_search_queue

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rag3weaver::config::{
    CatalogConfig, EntityDef, FieldDef, FieldType, KBConfig, RelationDef,
};
use rag3weaver::connection::CypherValue;
use rag3weaver::dataflow::{DataflowEvent, DataflowRuntime};
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::search_strategy::{
    ExpansionDirection, ExpansionRule, SearchStrategy,
};
use rag3weaver::{Catalog, Rag3dbConnection};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn text_title_for(kb: &str) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: Some(kb.to_string()),
        content_for: None,
        boost: None,
        default_value: None,
    }
}

fn text_content_for(kbs: &[&str]) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: None,
        content_for: Some(kbs.iter().map(|s| s.to_string()).collect()),
        boost: None,
        default_value: None,
    }
}

fn text_title_and_content(title_kb: &str, content_kbs: &[&str]) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: Some(title_kb.to_string()),
        content_for: Some(content_kbs.iter().map(|s| s.to_string()).collect()),
        boost: None,
        default_value: None,
    }
}

fn field(ft: FieldType) -> FieldDef {
    FieldDef {
        field_type: ft,
        title_for: None,
        content_for: None,
        boost: None,
        default_value: None,
    }
}

fn make_config() -> CatalogConfig {
    let mut dir_fields = HashMap::new();
    dir_fields.insert("name".into(), text_title_for("TreeKB"));
    dir_fields.insert("absolute_path".into(), text_content_for(&["TreeKB"]));
    dir_fields.insert("depth".into(), field(FieldType::Integer));

    let mut file_fields = HashMap::new();
    file_fields.insert("name".into(), text_content_for(&["TreeKB"]));
    file_fields.insert("absolute_path".into(), text_content_for(&["TreeKB"]));
    file_fields.insert("body".into(), text_content_for(&["TreeKB"]));

    let mut entities = HashMap::new();
    entities.insert(
        "Directory".into(),
        EntityDef {
            fields: dir_fields,
            hashsafe: Some(vec!["absolute_path".into()]),
        },
    );
    entities.insert(
        "File".into(),
        EntityDef {
            fields: file_fields,
            hashsafe: Some(vec!["absolute_path".into()]),
        },
    );

    let mut relations = HashMap::new();
    relations.insert(
        "HAS_FILE".into(),
        RelationDef {
            from: "Directory".into(),
            to: "File".into(),
            properties: None,
        },
    );

    let mut kbs = HashMap::new();
    kbs.insert(
        "TreeKB".into(),
        KBConfig {
            signals: SearchSignals::FULLTEXT,
            ..Default::default()
        },
    );

    CatalogConfig {
        name: Some("search-queue-test".into()),
        entities,
        relations,
        knowledge_bases: kbs,
        embedding_dim: 4,
        ..Default::default()
    }
}

fn make_directory(name: &str, path: &str, depth: i64) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("absolute_path".into(), CypherValue::String(path.into()));
    data.insert("depth".into(), CypherValue::Int(depth));
    data
}

fn make_file(name: &str, path: &str, body: &str) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("absolute_path".into(), CypherValue::String(path.into()));
    data.insert("body".into(), CypherValue::String(body.into()));
    data
}

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

async fn load_extensions(conn: &dyn rag3weaver::connection::DbConnection) {
    let root = rag3db_root();
    let extensions = [
        ("vector", format!("{root}/extension/vector/build/libvector.rag3db_extension")),
        ("lucivy_fts", format!("{root}/extension/lucivy_fts/build/liblucivy_fts.rag3db_extension")),
    ];
    for (name, ext_path) in &extensions {
        if !std::path::Path::new(ext_path).exists() {
            panic!(
                "Extension '{name}' not found at: {ext_path}\n\
                 Run ./run_e2e.sh --build-only first."
            );
        }
        let result = conn.execute(&format!("LOAD EXTENSION '{ext_path}'")).await;
        match result {
            Ok(_) => eprintln!("  loaded {name}"),
            Err(e) => panic!("Failed to load {name} from {ext_path}: {e}"),
        }
    }
}

async fn make_catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;
    Catalog::new(boxed, Box::new(MockEmbedder::new(4)), make_config())
}

/// Setup: 1 Directory ("src") with 2 Files, linked via HAS_FILE.
async fn setup_catalog() -> Catalog {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    let dir_ref = catalog
        .create("Directory", make_directory("src", "/repo/src/", 1))
        .unwrap();
    let file1_ref = catalog
        .create(
            "File",
            make_file(
                "auth.ts",
                "/repo/src/auth.ts",
                "export function authenticate(req: Request) { return validateToken(req.headers.authorization); }",
            ),
        )
        .unwrap();
    let file2_ref = catalog
        .create(
            "File",
            make_file(
                "db.ts",
                "/repo/src/db.ts",
                "export class Database { connect(url: string) { return pool.connect(url); } }",
            ),
        )
        .unwrap();

    catalog
        .link("HAS_FILE", dir_ref.clone(), file1_ref.clone(), BTreeMap::new())
        .unwrap();
    catalog
        .link("HAS_FILE", dir_ref.clone(), file2_ref.clone(), BTreeMap::new())
        .unwrap();

    let result = catalog.drain().await;
    eprintln!(
        "setup drain: processed={}, failed={}",
        result.processed, result.failed
    );
    assert_eq!(result.failed, 0);

    catalog
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 1: strategy_no_expansion — no expansions, same as search(), children=None
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn strategy_no_expansion() {
    let catalog = setup_catalog().await;
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    let strategy = SearchStrategy {
        search: SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
        expansions: vec![],
        max_rounds: 10,
    };

    let response = Catalog::search_with_strategy(
        catalog.clone(),
        "TreeKB",
        "src",
        strategy,
    )
    .await
    .unwrap();

    eprintln!(
        "no_expansion 'src': {} results, bm25={}",
        response.results.len(),
        response.meta.bm25_count,
    );

    assert!(!response.results.is_empty(), "Should find 'src' in TreeKB");

    // No expansion → all results should have other_children = None
    for (i, r) in response.results.iter().enumerate() {
        eprintln!(
            "  result[{i}]: uuid={}, entity={:?}, other_children={:?}",
            &r.uuid[..8.min(r.uuid.len())],
            r.entity,
            r.other_children.as_ref().map(|c| c.len()),
        );
        assert!(
            r.other_children.is_none(),
            "result[{i}] should have no children without expansion"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 2: strategy_expand_has_file — Directory expanded with HAS_FILE → 2 File children
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn strategy_expand_has_file() {
    let catalog = setup_catalog().await;
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    let strategy = SearchStrategy {
        search: SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
        expansions: vec![ExpansionRule {
            relation: "HAS_FILE".into(),
            direction: ExpansionDirection::Outgoing,
            source_entity: Some("Directory".into()),
            limit: 50,
        }],
        max_rounds: 10,
    };

    // Use build_dataflow_graph + runtime.subscribe for event tracing
    let (mut graph, services) = Catalog::build_dataflow_graph(
        catalog.clone(),
        "TreeKB",
        "src",
        strategy,
    )
    .await;

    let runtime = DataflowRuntime::with_services(10, services);
    let mut rx = runtime.subscribe();
    let output = runtime.execute(&mut graph).await.unwrap();

    // Dump all events
    while let Ok(ev) = rx.try_recv() {
        eprintln!("  [event] {ev:?}");
    }

    // Get results from compose node (terminal after expansion)
    let results = output
        .get("compose", "results")
        .and_then(|v| match v {
            rag3weaver::dataflow::PortValue::Results(r) => Some(r),
            _ => None,
        })
        .expect("compose node should have results output");

    eprintln!(
        "expand_has_file 'src': {} results",
        results.len(),
    );

    // Debug: print result data
    for (i, r) in results.iter().enumerate() {
        let src_entity = r.data.as_ref().and_then(|d| d.get("_source_entity")).and_then(|v| v.as_str());
        let src_uuid = r.data.as_ref().and_then(|d| d.get("_source_uuid")).and_then(|v| v.as_str());
        eprintln!(
            "  result[{i}]: entity={:?}, _source_entity={src_entity:?}, _source_uuid={src_uuid:?}, other_children={:?}",
            r.entity,
            r.other_children.as_ref().map(|c| c.len()),
        );
        if let Some(ref children) = r.other_children {
            for child in children {
                eprintln!(
                    "    child: entity={}, relation={}, uuid={}",
                    child.entity, child.relation, &child.uuid[..8.min(child.uuid.len())]
                );
            }
        }
    }

    // Find a result with children
    let mut found_children = false;
    for r in results {
        if let Some(ref children) = r.other_children {
            if !children.is_empty() {
                found_children = true;
                assert_eq!(
                    children.len(),
                    2,
                    "Directory 'src' should have 2 File children via HAS_FILE"
                );
                for child in children {
                    assert_eq!(child.entity, "File");
                    assert_eq!(child.relation, "HAS_FILE");
                }
            }
        }
    }

    assert!(
        found_children,
        "At least one result should have children after HAS_FILE expansion"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 3: strategy_entity_filter — File results NOT expanded (filter = Directory only)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn strategy_entity_filter() {
    let catalog = setup_catalog().await;
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    // Search "auth" — should match File(auth.ts) content, not Directory
    let strategy = SearchStrategy {
        search: SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
        expansions: vec![ExpansionRule {
            relation: "HAS_FILE".into(),
            direction: ExpansionDirection::Outgoing,
            source_entity: Some("Directory".into()), // Only expand Directory
            limit: 50,
        }],
        max_rounds: 10,
    };

    let response = Catalog::search_with_strategy(
        catalog.clone(),
        "TreeKB",
        "auth",
        strategy,
    )
    .await
    .unwrap();

    eprintln!(
        "entity_filter 'auth': {} results",
        response.results.len(),
    );

    assert!(!response.results.is_empty(), "Should find 'auth' in TreeKB");

    for (i, r) in response.results.iter().enumerate() {
        eprintln!(
            "  result[{i}]: entity={:?}, other_children={:?}",
            r.entity,
            r.other_children.as_ref().map(|c| c.len()),
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 4: strategy_child_data — ChildSummary.data contains File fields
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn strategy_child_data() {
    let catalog = setup_catalog().await;
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    let strategy = SearchStrategy {
        search: SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
        expansions: vec![ExpansionRule {
            relation: "HAS_FILE".into(),
            direction: ExpansionDirection::Outgoing,
            source_entity: Some("Directory".into()),
            limit: 50,
        }],
        max_rounds: 10,
    };

    let response = Catalog::search_with_strategy(
        catalog.clone(),
        "TreeKB",
        "src",
        strategy,
    )
    .await
    .unwrap();

    assert!(!response.results.is_empty());

    // Find result with children and check data fields
    let mut checked = false;
    for r in &response.results {
        if let Some(ref children) = r.other_children {
            for child in children {
                eprintln!(
                    "  child entity={}, data keys: {:?}",
                    child.entity,
                    child.data.keys().collect::<Vec<_>>()
                );

                // File entity should have name, absolute_path, body + internal fields
                assert!(
                    child.data.contains_key("name"),
                    "ChildSummary.data should contain 'name'"
                );
                assert!(
                    child.data.contains_key("absolute_path"),
                    "ChildSummary.data should contain 'absolute_path'"
                );
                assert!(
                    child.data.contains_key("body"),
                    "ChildSummary.data should contain 'body'"
                );
                checked = true;
            }
        }
    }

    assert!(checked, "Should have verified at least one child's data");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 5: strategy_max_rounds_guard — max_rounds=0 → error
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn strategy_max_rounds_guard() {
    let catalog = setup_catalog().await;
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    let strategy = SearchStrategy {
        search: SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
        expansions: vec![],
        max_rounds: 0, // Should fail
    };

    let result = Catalog::search_with_strategy(
        catalog.clone(),
        "TreeKB",
        "src",
        strategy,
    )
    .await;

    assert!(result.is_err(), "max_rounds=0 should produce an error");
    let err = result.unwrap_err().to_string();
    eprintln!("max_rounds error: {err}");
    assert!(
        err.contains("max iterations") || err.contains("max_rounds"),
        "Error should mention max iterations, got: {err}"
    );
}

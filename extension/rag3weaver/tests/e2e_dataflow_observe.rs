//! E2E integration tests: Dataflow observability (Phase 2).
//!
//! Tests tap, execute_with_report, and JSONL recording with real search.
//!
//! Run with: ./run_e2e.sh --test e2e_dataflow_observe

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rag3weaver::config::{
    CatalogConfig, EntityDef, FieldDef, FieldType, KBConfig, RelationDef,
};
use rag3weaver::connection::CypherValue;
use rag3weaver::dataflow::{
    DataflowEvent, DataflowRecorder, DataflowRuntime, ExecutionReport,
    ExecutionStatus, PortValue, RecordSink, TapEvent,
};
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::search_strategy::{
    ExpansionDirection, ExpansionRule, SearchStrategy,
};
use rag3weaver::{Catalog, Rag3dbConnection};

// ─── Helpers (same schema as e2e_search_queue) ──────────────────────────────

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
        name: Some("observe-test".into()),
        entities,
        relations,
        knowledge_bases: kbs,
        embedding_dim: 4,
        ..Default::default()
    }
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

async fn setup_catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;
    let mut catalog = Catalog::new(boxed, Box::new(MockEmbedder::new(4)), make_config());
    catalog.initialize().await.unwrap();

    let dir_ref = catalog
        .create(
            "Directory",
            {
                let mut d = BTreeMap::new();
                d.insert("name".into(), CypherValue::String("src".into()));
                d.insert("absolute_path".into(), CypherValue::String("/repo/src/".into()));
                d.insert("depth".into(), CypherValue::Int(1));
                d
            },
        )
        .unwrap();
    let file1_ref = catalog
        .create(
            "File",
            {
                let mut d = BTreeMap::new();
                d.insert("name".into(), CypherValue::String("auth.ts".into()));
                d.insert("absolute_path".into(), CypherValue::String("/repo/src/auth.ts".into()));
                d.insert("body".into(), CypherValue::String(
                    "export function authenticate(req: Request) { return validateToken(req.headers.authorization); }".into(),
                ));
                d
            },
        )
        .unwrap();
    let file2_ref = catalog
        .create(
            "File",
            {
                let mut d = BTreeMap::new();
                d.insert("name".into(), CypherValue::String("db.ts".into()));
                d.insert("absolute_path".into(), CypherValue::String("/repo/src/db.ts".into()));
                d.insert("body".into(), CypherValue::String(
                    "export class Database { connect(url: string) { return pool.connect(url); } }".into(),
                ));
                d
            },
        )
        .unwrap();

    catalog.link("HAS_FILE", dir_ref.clone(), file1_ref, BTreeMap::new()).unwrap();
    catalog.link("HAS_FILE", dir_ref, file2_ref, BTreeMap::new()).unwrap();

    let result = catalog.drain().await;
    assert_eq!(result.failed, 0);
    catalog
}

fn strategy_with_expansion() -> SearchStrategy {
    SearchStrategy {
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
    }
}

fn strategy_no_expansion() -> SearchStrategy {
    SearchStrategy {
        search: SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
        expansions: vec![],
        max_rounds: 10,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 1: execute_with_report — simple search produces a valid report
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn observe_execute_with_report_simple() {
    let catalog = setup_catalog().await;
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    let mut graph = Catalog::build_dataflow_graph(
        catalog.clone(),
        "TreeKB",
        "src",
        strategy_no_expansion(),
    )
    .await;

    let runtime = DataflowRuntime::new(10);
    let (output, report) = runtime.execute_with_report(&mut graph).await.unwrap();

    eprintln!("report: {}", serde_json::to_string_pretty(&report).unwrap());

    // Report should show completed
    assert!(matches!(report.status, ExecutionStatus::Completed));
    assert!(report.total_duration_ms < 10_000); // sanity

    // Should have 2 nodes: query_source + primary_search
    assert_eq!(report.nodes.len(), 2, "nodes: {:?}", report.nodes.iter().map(|n| &n.name).collect::<Vec<_>>());

    // Edges: query_source → primary_search
    assert!(!report.edges.is_empty());
    assert_eq!(report.edges[0].from_node, "query_source");
    assert_eq!(report.edges[0].to_node, "primary_search");

    // Output should have results
    let results = output.get("primary_search", "results");
    assert!(results.is_some(), "primary_search should have results output");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 2: execute_with_report — expansion produces expanded_nodes + more edges
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn observe_execute_with_report_expansion() {
    let catalog = setup_catalog().await;
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    let mut graph = Catalog::build_dataflow_graph(
        catalog.clone(),
        "TreeKB",
        "src",
        strategy_with_expansion(),
    )
    .await;

    let runtime = DataflowRuntime::new(10);
    let (_output, report) = runtime.execute_with_report(&mut graph).await.unwrap();

    eprintln!("report: {}", serde_json::to_string_pretty(&report).unwrap());

    assert!(matches!(report.status, ExecutionStatus::Completed));

    // Should have expanded nodes (fetch + compose added dynamically)
    assert!(
        !report.expanded_nodes.is_empty(),
        "Expansion should add dynamic nodes"
    );
    eprintln!("expanded: {:?}", report.expanded_nodes);

    // Should have > 2 nodes (query_source, primary_search, expansion, fetch_0, compose)
    assert!(
        report.nodes.len() >= 4,
        "Expected at least 4 nodes, got {}: {:?}",
        report.nodes.len(),
        report.nodes.iter().map(|n| &n.name).collect::<Vec<_>>()
    );

    // Edges should include dynamic ones (fetch → compose)
    let compose_edges: Vec<_> = report
        .edges
        .iter()
        .filter(|e| e.to_node == "compose")
        .collect();
    assert!(
        !compose_edges.is_empty(),
        "Should have edges to compose node"
    );
    eprintln!("compose edges: {:?}", compose_edges.iter().map(|e| format!("{}.{} → compose.{}", e.from_node, e.from_port, e.to_port)).collect::<Vec<_>>());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 3: tap_all — captures data flowing through every edge
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn observe_tap_all() {
    let catalog = setup_catalog().await;
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    let mut graph = Catalog::build_dataflow_graph(
        catalog.clone(),
        "TreeKB",
        "src",
        strategy_with_expansion(),
    )
    .await;

    let mut runtime = DataflowRuntime::new(10);
    let mut tap_rx = runtime.tap_all();
    runtime.execute(&mut graph).await.unwrap();

    // Collect all tap events
    let mut tap_events: Vec<TapEvent> = Vec::new();
    while let Ok(ev) = tap_rx.try_recv() {
        tap_events.push(ev);
    }

    eprintln!("tap events: {}", tap_events.len());
    for ev in &tap_events {
        let summary = match &ev.value {
            PortValue::Results(r) => format!("Results({})", r.len()),
            PortValue::Children(c) => format!("Children({} parents)", c.len()),
            PortValue::Meta(_) => "Meta".into(),
            PortValue::Query { kb_name, .. } => format!("Query({})", kb_name),
            _ => format!("{:?}", ev.value),
        };
        eprintln!(
            "  {}.{} → {}.{} : {}",
            ev.from_node, ev.from_port, ev.to_node, ev.to_port, summary
        );
    }

    // Should have at least the query_source → primary_search edge
    assert!(
        !tap_events.is_empty(),
        "tap_all should capture at least one edge"
    );

    // Check query edge
    let query_tap = tap_events
        .iter()
        .find(|e| e.from_node == "query_source" && e.to_node == "primary_search");
    assert!(
        query_tap.is_some(),
        "Should capture query_source → primary_search edge"
    );
    if let Some(ev) = query_tap {
        assert!(
            matches!(&ev.value, PortValue::Query { .. }),
            "Query edge should carry PortValue::Query"
        );
    }

    // Check results edge
    let results_tap = tap_events
        .iter()
        .find(|e| e.from_port == "results" && e.to_node != "compose");
    assert!(
        results_tap.is_some(),
        "Should capture a results edge"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 4: tap specific edge — only captures the targeted edge
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn observe_tap_specific_edge() {
    let catalog = setup_catalog().await;
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    let mut graph = Catalog::build_dataflow_graph(
        catalog.clone(),
        "TreeKB",
        "src",
        strategy_no_expansion(),
    )
    .await;

    let mut runtime = DataflowRuntime::new(10);
    // Only tap the query edge
    let mut tap_rx = runtime.tap("query_source", "query", "primary_search", "query");
    runtime.execute(&mut graph).await.unwrap();

    let mut tap_events: Vec<TapEvent> = Vec::new();
    while let Ok(ev) = tap_rx.try_recv() {
        tap_events.push(ev);
    }

    assert_eq!(
        tap_events.len(),
        1,
        "Specific tap should capture exactly 1 event"
    );
    assert_eq!(tap_events[0].from_node, "query_source");
    assert_eq!(tap_events[0].to_node, "primary_search");
    assert!(matches!(&tap_events[0].value, PortValue::Query { .. }));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 5: record to JSONL — write execution report to file
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn observe_record_jsonl() {
    let catalog = setup_catalog().await;
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    let mut graph = Catalog::build_dataflow_graph(
        catalog.clone(),
        "TreeKB",
        "src",
        strategy_with_expansion(),
    )
    .await;

    let runtime = DataflowRuntime::new(10);
    let (_output, report) = runtime.execute_with_report(&mut graph).await.unwrap();

    // Record to JSONL
    let jsonl_path = std::env::temp_dir().join("e2e_observe_test.jsonl");
    let _ = std::fs::remove_file(&jsonl_path);
    let recorder = DataflowRecorder::new(RecordSink::File(jsonl_path.clone()));
    recorder.record("search_pipeline", &report).await.unwrap();

    // Verify file contents
    let content = std::fs::read_to_string(&jsonl_path).unwrap();
    eprintln!("JSONL content:\n{}", content);

    assert!(content.contains("search_pipeline"));
    assert!(content.contains("query_source"));
    assert!(content.contains("primary_search"));
    assert!(content.contains("Completed"));

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
    assert_eq!(parsed["pipeline"], "search_pipeline");
    assert!(parsed["report"]["nodes"].as_array().unwrap().len() >= 2);

    let _ = std::fs::remove_file(&jsonl_path);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 6: record to rag3db — write execution to database nodes
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn observe_record_database() {
    let catalog = setup_catalog().await;
    let conn = catalog.conn_arc();
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    let mut graph = Catalog::build_dataflow_graph(
        catalog.clone(),
        "TreeKB",
        "src",
        strategy_with_expansion(),
    )
    .await;

    let runtime = DataflowRuntime::new(10);
    let (_output, report) = runtime.execute_with_report(&mut graph).await.unwrap();

    // Record to database
    let recorder = DataflowRecorder::new(RecordSink::Database(conn.clone()));
    recorder.record("search_pipeline", &report).await.unwrap();

    // Query back: _DataflowExecution
    let exec_result = conn
        .execute("MATCH (e:_DataflowExecution) RETURN e.pipeline_name, e.status, e.duration_ms, e.node_count")
        .await
        .unwrap();
    eprintln!("executions: {:?}", exec_result.rows);
    assert!(
        !exec_result.rows.is_empty(),
        "_DataflowExecution node should exist"
    );
    assert_eq!(
        exec_result.rows[0][0].as_str(),
        Some("search_pipeline")
    );
    assert_eq!(exec_result.rows[0][1].as_str(), Some("completed"));

    // Query back: _DataflowNodeRun
    let node_result = conn
        .execute("MATCH (n:_DataflowNodeRun)-[:PART_OF]->(e:_DataflowExecution) RETURN n.node_name, n.status ORDER BY n.node_name")
        .await
        .unwrap();
    eprintln!("node runs: {:?}", node_result.rows);
    assert!(
        node_result.rows.len() >= 2,
        "Should have at least 2 node runs, got {}",
        node_result.rows.len()
    );

    // Query back: _DataflowEdgeRun
    let edge_result = conn
        .execute("MATCH (r:_DataflowEdgeRun)-[:PART_OF]->(e:_DataflowExecution) RETURN r.from_node, r.to_node, r.value_summary")
        .await
        .unwrap();
    eprintln!("edge runs: {:?}", edge_result.rows);
    assert!(
        !edge_result.rows.is_empty(),
        "Should have at least 1 edge run"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 7: report serializes to JSON correctly
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn observe_report_json_structure() {
    let catalog = setup_catalog().await;
    let catalog = Arc::new(tokio::sync::Mutex::new(catalog));

    let mut graph = Catalog::build_dataflow_graph(
        catalog.clone(),
        "TreeKB",
        "src",
        strategy_with_expansion(),
    )
    .await;

    let runtime = DataflowRuntime::new(10);
    let (_output, report) = runtime.execute_with_report(&mut graph).await.unwrap();

    // Serialize to JSON
    let json = serde_json::to_value(&report).unwrap();

    // Verify structure
    assert!(json["nodes"].is_array());
    assert!(json["edges"].is_array());
    assert!(json["expanded_nodes"].is_array());
    assert!(json["total_duration_ms"].is_number());
    assert_eq!(json["status"], "Completed");

    // Each node has required fields
    for node in json["nodes"].as_array().unwrap() {
        assert!(node["name"].is_string(), "node missing name: {node}");
        assert!(node["duration_ms"].is_number(), "node missing duration_ms: {node}");
        assert!(node["output_ports"].is_array(), "node missing output_ports: {node}");
    }

    // Each edge has required fields
    for edge in json["edges"].as_array().unwrap() {
        assert!(edge["from_node"].is_string(), "edge missing from_node: {edge}");
        assert!(edge["to_node"].is_string(), "edge missing to_node: {edge}");
        assert!(edge["value_summary"].is_string(), "edge missing value_summary: {edge}");
    }

    // Value summaries should be descriptive
    let summaries: Vec<&str> = json["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["value_summary"].as_str())
        .collect();
    eprintln!("summaries: {:?}", summaries);

    // At least one should show Results(N) or Query(...)
    assert!(
        summaries.iter().any(|s| s.starts_with("Results(") || s.starts_with("Query(")),
        "Edge summaries should include data type info: {:?}",
        summaries
    );
}

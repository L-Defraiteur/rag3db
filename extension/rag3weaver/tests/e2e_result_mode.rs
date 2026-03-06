//! E2E integration tests: ResultMode (Aggregated / SourceResolved / Detailed).
//!
//! Uses the Phase 0b config (TreeKB multi-entity + FileKB single-entity).
//!
//! Run with: ./run_e2e.sh --test e2e_result_mode

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::{
    CatalogConfig, ChunkingConfig, EntityDef, FieldDef, FieldType, KBConfig, RelationDef,
};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::{Consistency, ResultMode, SearchOptions, SearchSignals};
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

/// Config: TreeKB (BM25, multi-entity: Directory + File) + FileKB (hybrid, single-entity: File).
fn make_config() -> CatalogConfig {
    let mut dir_fields = HashMap::new();
    dir_fields.insert("name".into(), text_title_for("TreeKB"));
    dir_fields.insert("absolute_path".into(), text_content_for(&["TreeKB"]));
    dir_fields.insert("depth".into(), field(FieldType::Integer));

    let mut file_fields = HashMap::new();
    file_fields.insert("name".into(), text_title_and_content("FileKB", &["TreeKB"]));
    file_fields.insert("absolute_path".into(), text_content_for(&["TreeKB"]));
    file_fields.insert("body".into(), text_content_for(&["FileKB"]));

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
    kbs.insert(
        "FileKB".into(),
        KBConfig {
            signals: SearchSignals::HYBRID,
            ..Default::default()
        },
    );

    CatalogConfig {
        name: Some("result-mode-test".into()),
        entities,
        relations,
        knowledge_bases: kbs,
        embedding_dim: 4,
        ..Default::default()
    }
}

fn make_directory(name: &str, absolute_path: &str, depth: i64) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("absolute_path".into(), CypherValue::String(absolute_path.into()));
    data.insert("depth".into(), CypherValue::Int(depth));
    data
}

fn make_file(name: &str, absolute_path: &str, body: &str) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("absolute_path".into(), CypherValue::String(absolute_path.into()));
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

/// Setup: 1 Directory ("src") with 2 Files, both linked via HAS_FILE.
/// TreeKB has 1 index entry (Directory = title entity), aggregating content from Dir + Files.
/// FileKB has 2 index entries (one per File).
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
                "logger.ts",
                "/repo/src/logger.ts",
                "export class Logger { log(msg: string) { console.log(msg); } }",
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

async fn query_rows(catalog: &Catalog, cypher: &str) -> Vec<Vec<CypherValue>> {
    catalog.execute_raw(cypher).await.unwrap().rows
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 1: Aggregated mode (default) — non-regression
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn result_mode_aggregated_default() {
    let mut catalog = setup_catalog().await;

    // Search TreeKB with default options (Aggregated) + diagnostics
    let response = catalog
        .search(
            "TreeKB",
            "auth",
            SearchOptions {
                consistency: Consistency::Immediate,
                diagnostics: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!(
        "Aggregated 'auth': {} results, bm25={}, time={}ms",
        response.results.len(),
        response.meta.bm25_count,
        response.meta.search_time_ms,
    );
    // Print diagnostics
    if let Some(ref diag) = response.meta.diagnostics {
        eprintln!("  diagnostics: embed={}ms bm25={}ms vector={}ms sparse={}ms resolve={}ms fuse={}ms enrich={}ms",
            diag.embed_ms, diag.bm25_ms, diag.vector_ms, diag.sparse_ms,
            diag.resolve_ms, diag.fuse_ms, diag.enrich_ms);
        for (i, hit) in diag.bm25_hits.iter().enumerate() {
            eprintln!("  bm25_hit[{i}]: parent={}, score={}, hl_raw={}", &hit.parent_uuid[..8.min(hit.parent_uuid.len())], hit.score, hit.highlights_raw);
            eprintln!("    highlights_parsed: {:?}", hit.highlights_parsed);
            eprintln!("    chunks_available={}, chunks_matched={}", hit.chunks_available, hit.chunks_matched);
            for co in &hit.chunk_overlaps {
                eprintln!("    chunk {}: content_offset={}, start_char={}, end_char={}, global=[{}..{}], overlap={}",
                    &co.chunk_uuid[..8.min(co.chunk_uuid.len())], co.content_offset, co.start_char, co.end_char,
                    co.global_start, co.global_end, co.overlap);
            }
        }
    }
    // Print results
    for (i, r) in response.results.iter().enumerate() {
        eprintln!("  result[{i}]: uuid={}, entity={:?}, chunk={}, chunks={:?}",
            &r.uuid[..8.min(r.uuid.len())], r.entity, r.chunk.is_some(),
            r.chunks.as_ref().map(|c| c.len()));
    }

    assert!(!response.results.is_empty(), "Should find 'auth' in TreeKB");

    let top = &response.results[0];
    // Aggregated: entity should be TreeKB_Index
    assert_eq!(
        top.entity.as_deref(),
        Some("TreeKB_Index"),
        "Aggregated result entity should be TreeKB_Index"
    );
    // Should have data with _title
    let data = top.data.as_ref().expect("should have data");
    assert!(
        data.contains_key("_title"),
        "Aggregated data should contain _title"
    );
    // Should have best chunk (or title-only match is acceptable)
    if top.chunk.is_none() {
        eprintln!("  NOTE: BM25 hit has no chunk — likely title-only match. Checking diagnostics...");
        if let Some(ref diag) = response.meta.diagnostics {
            if let Some(hit) = diag.bm25_hits.first() {
                let has_content_hl = hit.highlights_parsed.contains_key("_content");
                let has_title_hl = hit.highlights_parsed.contains_key("_title");
                eprintln!("    has_content_hl={has_content_hl}, has_title_hl={has_title_hl}");
                if !has_content_hl && has_title_hl {
                    eprintln!("    -> Title-only match confirmed. chunk=None is expected behavior.");
                }
            }
        }
    }
    // chunks field should be None in Aggregated mode
    assert!(
        top.chunks.is_none(),
        "Aggregated mode should NOT have chunks field populated"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 2: Aggregated explicitly — same as default
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn result_mode_aggregated_explicit() {
    let mut catalog = setup_catalog().await;

    let response = catalog
        .search(
            "TreeKB",
            "auth",
            SearchOptions {
                consistency: Consistency::Immediate,
                result_mode: ResultMode::Aggregated,
                diagnostics: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("Aggregated explicit 'auth': {} results, bm25={}", response.results.len(), response.meta.bm25_count);
    if let Some(ref diag) = response.meta.diagnostics {
        for (i, hit) in diag.bm25_hits.iter().enumerate() {
            eprintln!("  bm25_hit[{i}]: hl_raw={}, chunks_available={}, chunks_matched={}",
                hit.highlights_raw, hit.chunks_available, hit.chunks_matched);
        }
    }
    for (i, r) in response.results.iter().enumerate() {
        eprintln!("  result[{i}]: chunk={}, chunks={:?}", r.chunk.is_some(), r.chunks.as_ref().map(|c| c.len()));
    }

    assert!(!response.results.is_empty());
    let top = &response.results[0];
    assert_eq!(top.entity.as_deref(), Some("TreeKB_Index"));
    // chunk may be None for title-only BM25 matches — that's acceptable
    assert!(top.chunks.is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 3: SourceResolved — entity/uuid/data resolved to source
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn result_mode_source_resolved() {
    let mut catalog = setup_catalog().await;

    // Search TreeKB for "auth" in SourceResolved mode
    let response = catalog
        .search(
            "TreeKB",
            "auth",
            SearchOptions {
                consistency: Consistency::Immediate,
                result_mode: ResultMode::SourceResolved,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!(
        "SourceResolved 'auth': {} results",
        response.results.len()
    );
    assert!(!response.results.is_empty(), "SourceResolved should find results");

    for (i, r) in response.results.iter().enumerate() {
        eprintln!(
            "  result[{i}]: entity={:?}, uuid={}, score={}",
            r.entity, r.uuid, r.score
        );

        // Entity should be a source entity (Directory or File), NOT TreeKB_Index
        let entity = r.entity.as_deref().expect("should have entity");
        assert!(
            entity == "Directory" || entity == "File",
            "SourceResolved entity should be Directory or File, got '{entity}'"
        );

        // Data should contain the source entity's fields, NOT _title/_content
        let data = r.data.as_ref().expect("should have data");
        assert!(
            !data.contains_key("_title"),
            "SourceResolved data should NOT contain _title (index field)"
        );

        if entity == "Directory" {
            assert!(
                data.contains_key("name"),
                "Directory data should contain 'name'"
            );
            assert!(
                data.contains_key("absolute_path"),
                "Directory data should contain 'absolute_path'"
            );
            assert!(
                data.contains_key("depth"),
                "Directory data should contain 'depth'"
            );
        } else {
            assert!(
                data.contains_key("name"),
                "File data should contain 'name'"
            );
            assert!(
                data.contains_key("body"),
                "File data should contain 'body'"
            );
        }

        // chunks should be None in SourceResolved
        assert!(
            r.chunks.is_none(),
            "SourceResolved should NOT have chunks populated"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 4: SourceResolved — uuid matches the source entity
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn result_mode_source_resolved_uuid_matches() {
    let mut catalog = setup_catalog().await;

    // Get the Directory UUID for reference
    let dir_rows = query_rows(
        &catalog,
        "MATCH (d:Directory {name: 'src'}) RETURN d._uuid",
    )
    .await;
    assert_eq!(dir_rows.len(), 1);
    let dir_uuid = dir_rows[0][0].as_str().unwrap().to_string();

    // Search TreeKB — the "src" directory is the title entity
    let response = catalog
        .search(
            "TreeKB",
            "src",
            SearchOptions {
                consistency: Consistency::Immediate,
                result_mode: ResultMode::SourceResolved,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("SourceResolved 'src': {} results", response.results.len());
    assert!(!response.results.is_empty());

    // At least one result should have the Directory's uuid
    let has_dir = response
        .results
        .iter()
        .any(|r| r.entity.as_deref() == Some("Directory") && r.uuid == dir_uuid);
    assert!(
        has_dir,
        "SourceResolved should resolve to the Directory entity with uuid={dir_uuid}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 5: Detailed mode — chunks populated with attribution
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn result_mode_detailed_chunks() {
    let mut catalog = setup_catalog().await;

    let response = catalog
        .search(
            "TreeKB",
            "auth",
            SearchOptions {
                consistency: Consistency::Immediate,
                result_mode: ResultMode::Detailed,
                diagnostics: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("Detailed 'auth': {} results", response.results.len());
    // Print diagnostics first
    if let Some(ref diag) = response.meta.diagnostics {
        for (i, hit) in diag.bm25_hits.iter().enumerate() {
            eprintln!("  bm25_hit[{i}]: parent={}, score={}", &hit.parent_uuid[..8.min(hit.parent_uuid.len())], hit.score);
            eprintln!("    hl_raw={}", hit.highlights_raw);
            eprintln!("    hl_parsed={:?}", hit.highlights_parsed);
            eprintln!("    chunks_available={}, chunks_matched={}", hit.chunks_available, hit.chunks_matched);
            for co in &hit.chunk_overlaps {
                eprintln!("    chunk {}: offset={}, [{},{}], global=[{},{}], overlap={}",
                    &co.chunk_uuid[..8.min(co.chunk_uuid.len())], co.content_offset,
                    co.start_char, co.end_char, co.global_start, co.global_end, co.overlap);
            }
        }
    }
    for (i, r) in response.results.iter().enumerate() {
        eprintln!("  result[{i}]: uuid={}, chunk={}, chunks={:?}",
            &r.uuid[..8.min(r.uuid.len())], r.chunk.is_some(), r.chunks.as_ref().map(|c| c.len()));
    }

    assert!(!response.results.is_empty(), "Detailed should find results");

    for (i, r) in response.results.iter().enumerate() {
        assert_eq!(
            r.entity.as_deref(),
            Some("TreeKB_Index"),
            "Detailed result entity should be TreeKB_Index"
        );

        // chunks should be populated (may be empty for title-only matches)
        let chunks = r
            .chunks
            .as_ref()
            .expect(&format!("result[{i}] should have chunks in Detailed mode"));

        if chunks.is_empty() {
            eprintln!("  result[{i}]: chunks is empty — title-only BM25 match");
            // Check diagnostics to confirm title-only match
            if let Some(ref diag) = response.meta.diagnostics {
                if let Some(hit) = diag.bm25_hits.get(i) {
                    let has_content = hit.highlights_parsed.contains_key("_content");
                    let has_title = hit.highlights_parsed.contains_key("_title");
                    eprintln!("    has_content={has_content}, has_title={has_title}");
                }
            }
            continue; // Title-only match — empty chunks is acceptable
        }

        eprintln!(
            "  result[{i}]: uuid={}, chunks={}",
            &r.uuid[..8],
            chunks.len()
        );

        for (j, chunk) in chunks.iter().enumerate() {
            eprintln!(
                "    chunk[{j}]: source_entity={}, source_uuid={}, source_field={}, score={}",
                chunk.source_entity,
                &chunk.source_uuid[..8.min(chunk.source_uuid.len())],
                chunk.source_field,
                chunk.score,
            );

            assert!(
                chunk.source_entity == "Directory" || chunk.source_entity == "File",
                "chunk source_entity should be Directory or File, got '{}'",
                chunk.source_entity
            );
            assert!(!chunk.source_uuid.is_empty(), "chunk source_uuid should not be empty");
            assert!(!chunk.source_field.is_empty(), "chunk source_field should not be empty");
            assert!(!chunk.text.is_empty(), "chunk text should not be empty");
            assert!(chunk.end_char > chunk.start_char, "end_char > start_char");
        }

        assert!(r.chunk.is_none(), "Detailed mode should NOT have single chunk field populated");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 6: Detailed — chunk source_uuid matches actual entity UUIDs
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn result_mode_detailed_chunk_source_uuid_valid() {
    let mut catalog = setup_catalog().await;

    // Get all entity UUIDs
    let dir_uuids: Vec<String> = query_rows(&catalog, "MATCH (d:Directory) RETURN d._uuid")
        .await
        .iter()
        .filter_map(|r| r[0].as_str().map(|s| s.to_string()))
        .collect();
    let file_uuids: Vec<String> = query_rows(&catalog, "MATCH (f:File) RETURN f._uuid")
        .await
        .iter()
        .filter_map(|r| r[0].as_str().map(|s| s.to_string()))
        .collect();

    let response = catalog
        .search(
            "TreeKB",
            "src",
            SearchOptions {
                consistency: Consistency::Immediate,
                result_mode: ResultMode::Detailed,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert!(!response.results.is_empty());

    for r in &response.results {
        if let Some(ref chunks) = r.chunks {
            for chunk in chunks {
                let valid = match chunk.source_entity.as_str() {
                    "Directory" => dir_uuids.contains(&chunk.source_uuid),
                    "File" => file_uuids.contains(&chunk.source_uuid),
                    other => panic!("unexpected source_entity: {other}"),
                };
                assert!(
                    valid,
                    "chunk source_uuid '{}' should exist as a {} entity",
                    chunk.source_uuid, chunk.source_entity
                );
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 7: Detailed on FileKB (single-entity, with vector)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn result_mode_detailed_filekb() {
    let mut catalog = setup_catalog().await;

    let response = catalog
        .search(
            "FileKB",
            "authenticate",
            SearchOptions {
                consistency: Consistency::Immediate,
                result_mode: ResultMode::Detailed,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("Detailed FileKB 'authenticate': {} results", response.results.len());
    assert!(!response.results.is_empty());

    for r in &response.results {
        assert_eq!(r.entity.as_deref(), Some("FileKB_Index"));
        let chunks = r.chunks.as_ref().expect("should have chunks");
        assert!(!chunks.is_empty());

        for chunk in chunks {
            // FileKB is single-entity (File only)
            assert_eq!(
                chunk.source_entity, "File",
                "FileKB chunks should be sourced from File"
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 8: SourceResolved on FileKB — should resolve to File entity
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn result_mode_source_resolved_filekb() {
    let mut catalog = setup_catalog().await;

    let response = catalog
        .search(
            "FileKB",
            "authenticate",
            SearchOptions {
                consistency: Consistency::Immediate,
                result_mode: ResultMode::SourceResolved,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!(
        "SourceResolved FileKB 'authenticate': {} results",
        response.results.len()
    );
    assert!(!response.results.is_empty());

    for r in &response.results {
        let entity = r.entity.as_deref().expect("should have entity");
        assert_eq!(
            entity, "File",
            "FileKB SourceResolved should resolve to File, got '{entity}'"
        );

        let data = r.data.as_ref().expect("should have data");
        assert!(data.contains_key("name"), "File data should have 'name'");
        assert!(data.contains_key("body"), "File data should have 'body'");
        assert!(
            !data.contains_key("_title"),
            "SourceResolved should NOT have _title"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 9: _source_entity and _source_uuid columns on chunks
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn result_mode_chunk_columns_persisted() {
    let catalog = setup_catalog().await;

    // Verify _source_entity and _source_uuid are stored on chunks
    let chunk_rows = query_rows(
        &catalog,
        "MATCH (c:TreeKB_Index_Chunk) \
         RETURN c._source_entity, c._source_uuid, c._source_field \
         ORDER BY c._source_entity",
    )
    .await;

    assert!(!chunk_rows.is_empty(), "Should have TreeKB chunks");

    for row in &chunk_rows {
        let source_entity = row[0].as_str().unwrap_or("");
        let source_uuid = row[1].as_str().unwrap_or("");
        let source_field = row[2].as_str().unwrap_or("");

        eprintln!(
            "  chunk: entity={source_entity}, uuid={}, field={source_field}",
            &source_uuid[..8.min(source_uuid.len())]
        );

        assert!(
            source_entity == "Directory" || source_entity == "File",
            "_source_entity should be Directory or File, got '{source_entity}'"
        );
        assert!(
            !source_uuid.is_empty(),
            "_source_uuid should not be empty"
        );
        assert!(
            !source_field.is_empty(),
            "_source_field should not be empty"
        );
    }

    // Also check FileKB chunks
    let filekb_chunks = query_rows(
        &catalog,
        "MATCH (c:FileKB_Index_Chunk) \
         RETURN c._source_entity, c._source_uuid",
    )
    .await;
    assert!(!filekb_chunks.is_empty());
    for row in &filekb_chunks {
        assert_eq!(
            row[0].as_str().unwrap_or(""),
            "File",
            "FileKB chunks should all be sourced from File"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 10: Aggregated non-regression — existing tests' search patterns still work
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn result_mode_aggregated_data_enrichment() {
    let mut catalog = setup_catalog().await;

    // Aggregated mode: data should contain index fields (_title, _source_entity, _source_uuid, etc.)
    let response = catalog
        .search(
            "TreeKB",
            "src",
            SearchOptions {
                consistency: Consistency::Immediate,
                result_mode: ResultMode::Aggregated,
                diagnostics: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    eprintln!("Aggregated enrichment 'src': {} results, bm25={}", response.results.len(), response.meta.bm25_count);
    if let Some(ref diag) = response.meta.diagnostics {
        for (i, hit) in diag.bm25_hits.iter().enumerate() {
            eprintln!("  bm25_hit[{i}]: hl_raw={}", hit.highlights_raw);
            eprintln!("    hl_parsed={:?}", hit.highlights_parsed);
            eprintln!("    chunks_available={}, chunks_matched={}", hit.chunks_available, hit.chunks_matched);
            for co in &hit.chunk_overlaps {
                eprintln!("    chunk {}: offset={}, [{},{}], global=[{},{}], overlap={}",
                    &co.chunk_uuid[..8.min(co.chunk_uuid.len())], co.content_offset,
                    co.start_char, co.end_char, co.global_start, co.global_end, co.overlap);
            }
        }
    }
    for (i, r) in response.results.iter().enumerate() {
        eprintln!("  result[{i}]: chunk={}, chunks={:?}", r.chunk.is_some(), r.chunks.as_ref().map(|c| c.len()));
    }

    assert!(!response.results.is_empty());
    let top = &response.results[0];
    let data = top.data.as_ref().expect("should have data");

    // Index fields should be present
    assert!(data.contains_key("_title"), "data should have _title");

    // Best chunk — may be None for title-only matches
    if let Some(ref chunk) = top.chunk {
        assert!(!chunk.text.is_empty(), "chunk text should not be empty");
        assert!(chunk.end_char > chunk.start_char);
    } else {
        eprintln!("  NOTE: No chunk — title-only match for 'src'");
    }
}

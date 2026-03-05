//! E2E integration tests: Phase 0b — cross-entity KB, AggregateProcessor,
//! highlight→chunk resolution, _content_offset, SOURCED rels, title truncation,
//! delete/update contentFor-only propagation.
//!
//! Config: TreeKB (multi-entity, BM25 only) + FileKB (single-entity, BM25+vector).
//!
//! Run with: ./run_e2e.sh --test e2e_phase0b

#![cfg(feature = "rag3db-native")]

use std::collections::{BTreeMap, HashMap};

use rag3weaver::config::{
    CatalogConfig, ChunkingConfig, EntityDef, FieldDef, FieldType, KBConfig, RelationDef,
};
use rag3weaver::connection::CypherValue;
use rag3weaver::embedder::MockEmbedder;
use rag3weaver::search::{Consistency, SearchOptions, SearchSignals};
use rag3weaver::{Catalog, Hashsafe, Rag3dbConnection};

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

/// Field that is titleFor one KB and contentFor another.
fn text_title_and_content(title_kb: &str, content_kbs: &[&str]) -> FieldDef {
    FieldDef {
        field_type: FieldType::Text,
        title_for: Some(title_kb.to_string()),
        content_for: Some(content_kbs.iter().map(|s| s.to_string()).collect()),
        boost: None,
        default_value: None,
    }
}

/// Config from doc 13:
///
/// - Directory: name (titleFor TreeKB), absolute_path (contentFor TreeKB)
/// - File: name (titleFor FileKB, contentFor TreeKB), absolute_path (contentFor TreeKB),
///          body (contentFor FileKB)
/// - HAS_FILE: Directory → File
/// - TreeKB: BM25 only (multi-entity: Directory + File)
/// - FileKB: BM25 + vector (single-entity: File)
fn make_phase0b_config() -> CatalogConfig {
    // Directory entity
    let mut dir_fields = HashMap::new();
    dir_fields.insert("name".into(), text_title_for("TreeKB"));
    dir_fields.insert("absolute_path".into(), text_content_for(&["TreeKB"]));

    // File entity
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

    // Relations
    let mut relations = HashMap::new();
    relations.insert(
        "HAS_FILE".into(),
        RelationDef {
            from: "Directory".into(),
            to: "File".into(),
            properties: None,
        },
    );

    // Knowledge Bases
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
        name: Some("phase0b-test".into()),
        entities,
        relations,
        knowledge_bases: kbs,
        embedding_dim: 4,
        ..Default::default()
    }
}

/// Root path of the rag3db source tree.
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

/// Load required extensions into a native connection.
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
    Catalog::new(boxed, Box::new(MockEmbedder::new(4)), make_phase0b_config())
}

/// Same as make_catalog but with a custom ChunkingConfig override.
async fn make_catalog_with_chunking(chunking: ChunkingConfig) -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref()).await;
    let mut config = make_phase0b_config();
    for kb in config.knowledge_bases.values_mut() {
        kb.chunking = chunking.clone();
    }
    Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config)
}

fn make_directory(name: &str, absolute_path: &str) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("absolute_path".into(), CypherValue::String(absolute_path.into()));
    data
}

fn make_file(name: &str, absolute_path: &str, body: &str) -> BTreeMap<String, CypherValue> {
    let mut data = BTreeMap::new();
    data.insert("name".into(), CypherValue::String(name.into()));
    data.insert("absolute_path".into(), CypherValue::String(absolute_path.into()));
    data.insert("body".into(), CypherValue::String(body.into()));
    data
}

/// Query helper: execute raw Cypher and return all rows.
async fn query_rows(catalog: &Catalog, cypher: &str) -> Vec<Vec<CypherValue>> {
    let result = catalog.execute_raw(cypher).await.unwrap();
    result.rows
}

/// Query helper: return the single scalar value from a COUNT query.
async fn query_count(catalog: &Catalog, cypher: &str) -> i64 {
    let rows = query_rows(catalog, cypher).await;
    rows.first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 1: Ingestion + schema validation
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_ingest_and_schema() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    // Schema should have created all required tables
    // Entity tables
    assert!(catalog.get_entity_def("Directory").is_some());
    assert!(catalog.get_entity_def("File").is_some());
    assert!(catalog.get_relation_def("HAS_FILE").is_some());

    // KB metadata
    let tree_kb = catalog.get_kb_metadata("TreeKB").expect("TreeKB metadata");
    assert_eq!(tree_kb.title.entity, "Directory");
    assert_eq!(tree_kb.title.field, "name");
    assert!(tree_kb.entities.contains("Directory"));
    assert!(tree_kb.entities.contains("File"));

    let file_kb = catalog.get_kb_metadata("FileKB").expect("FileKB metadata");
    assert_eq!(file_kb.title.entity, "File");
    assert_eq!(file_kb.title.field, "name");

    // Create entities
    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate(req: Request) { return validateToken(req.headers.authorization); }"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain().await;
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert_eq!(result.failed, 0);

    // Verify entity counts
    assert_eq!(catalog.count("Directory").await.unwrap(), 1);
    assert_eq!(catalog.count("File").await.unwrap(), 1);

    // TreeKB_Index should have 1 entry (Directory = title entity)
    let tree_idx_count = query_count(&catalog, "MATCH (t:TreeKB_Index) RETURN count(t)").await;
    assert_eq!(tree_idx_count, 1, "TreeKB should have 1 index entry (for the Directory)");

    // TreeKB_Index entry should have aggregated content from Directory + File
    let rows = query_rows(
        &catalog,
        "MATCH (t:TreeKB_Index) RETURN t._title, t._content, t._content_hash",
    ).await;
    assert_eq!(rows.len(), 1);
    let title = rows[0][0].as_str().unwrap_or("");
    let content = rows[0][1].as_str().unwrap_or("");
    let content_hash = rows[0][2].as_str().unwrap_or("");
    assert_eq!(title, "src", "TreeKB title should be Directory.name");
    assert!(content.contains("/repo/src/"), "TreeKB content should contain Directory.absolute_path");
    assert!(content.contains("auth.ts"), "TreeKB content should contain File.name");
    assert!(content.contains("/repo/src/auth.ts"), "TreeKB content should contain File.absolute_path");
    assert!(!content_hash.is_empty(), "content_hash should be set (not sentinel)");

    // TreeKB_Index_Chunk should exist
    let chunk_count = query_count(&catalog, "MATCH (c:TreeKB_Index_Chunk) RETURN count(c)").await;
    assert!(chunk_count > 0, "TreeKB should have chunks: got {chunk_count}");

    // FileKB_Index should have 1 entry (File = title entity for FileKB)
    let file_idx_count = query_count(&catalog, "MATCH (f:FileKB_Index) RETURN count(f)").await;
    assert_eq!(file_idx_count, 1, "FileKB should have 1 index entry");

    // FileKB chunks
    let filekb_chunk_count = query_count(&catalog, "MATCH (c:FileKB_Index_Chunk) RETURN count(c)").await;
    assert!(filekb_chunk_count > 0, "FileKB should have chunks: got {filekb_chunk_count}");

    // SOURCED rels should exist
    let dir_sourced = query_count(
        &catalog,
        "MATCH (:Directory)-[:Directory_SOURCED_TreeKB]->(:TreeKB_Index_Chunk) RETURN count(*)",
    ).await;
    assert!(dir_sourced > 0, "Directory should have SOURCED rels to TreeKB chunks");

    let file_sourced_tree = query_count(
        &catalog,
        "MATCH (:File)-[:File_SOURCED_TreeKB]->(:TreeKB_Index_Chunk) RETURN count(*)",
    ).await;
    assert!(file_sourced_tree > 0, "File should have SOURCED rels to TreeKB chunks");

    let file_sourced_file = query_count(
        &catalog,
        "MATCH (:File)-[:File_SOURCED_FileKB]->(:FileKB_Index_Chunk) RETURN count(*)",
    ).await;
    assert!(file_sourced_file > 0, "File should have SOURCED rels to FileKB chunks");

    eprintln!(
        "Schema OK: TreeKB chunks={chunk_count}, FileKB chunks={filekb_chunk_count}, \
         SOURCED: dir→tree={dir_sourced}, file→tree={file_sourced_tree}, file→file={file_sourced_file}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 2: BM25 search on multi-entity KB (TreeKB)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_bm25_search_multi_entity() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate(req: Request) { return true; }"),
    ).unwrap();
    catalog.create("Directory", make_directory("lib", "/repo/lib/")).unwrap();

    catalog.link("HAS_FILE", Hashsafe::new("Directory", &["/repo/src/"]), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain().await;
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert_eq!(result.failed, 0);

    // Search for "auth" — should find in TreeKB (File.name = "auth.ts")
    let response = catalog.search(
        "TreeKB",
        "auth",
        SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
    ).await.unwrap();

    eprintln!("TreeKB search 'auth': {} results, bm25_count={}", response.results.len(), response.meta.bm25_count);
    assert!(response.results.len() > 0, "TreeKB should find 'auth' in File content");

    // Search for "lib" — should find the lib Directory's content
    let response2 = catalog.search(
        "TreeKB",
        "lib",
        SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
    ).await.unwrap();
    eprintln!("TreeKB search 'lib': {} results", response2.results.len());
    assert!(response2.results.len() > 0, "TreeKB should find 'lib' in Directory content");

    // Search for nonsense — 0 results
    let response3 = catalog.search(
        "TreeKB",
        "xyznonexistent",
        SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
    ).await.unwrap();
    assert_eq!(response3.results.len(), 0, "nonsense query should return 0 results");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 3: BM25 highlight → chunk resolution (single-entity FileKB)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_bm25_highlight_chunk_single_entity() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    // Create a File with a body long enough to produce multiple chunks,
    // containing "authentication" at a known position.
    let body = format!(
        "{}authentication is the process of verifying identity.{}",
        "Lorem ipsum dolor sit amet. ".repeat(60),  // ~1680 chars before
        " More text follows here to extend the body.".repeat(20),
    );

    catalog.create(
        "File",
        make_file("auth_module.ts", "/repo/src/auth_module.ts", &body),
    ).unwrap();

    let result = catalog.drain().await;
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert_eq!(result.failed, 0);

    // Search FileKB for "authentication"
    let response = catalog.search(
        "FileKB",
        "authentication",
        SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
    ).await.unwrap();

    eprintln!("FileKB search 'authentication': {} results", response.results.len());
    assert!(response.results.len() > 0, "FileKB should find 'authentication'");

    // If we have chunk info, verify offsets
    for r in &response.results {
        if let Some(ref chunk) = r.chunk {
            eprintln!(
                "  chunk: start_char={}, end_char={}, start_line={}, end_line={}, text_len={}",
                chunk.start_char, chunk.end_char, chunk.start_line, chunk.end_line, chunk.text.len()
            );
            // Chunk text should be a valid substring of body
            assert!(chunk.end_char > chunk.start_char, "end_char > start_char");
            assert!(chunk.end_char <= body.len(), "end_char <= body.len()");
            let slice = &body[chunk.start_char..chunk.end_char];
            // The chunk text should match (modulo trimming)
            assert!(
                slice.contains(&chunk.text) || chunk.text.contains(slice.trim()),
                "chunk text should correspond to body[start_char..end_char]"
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 4: Vector search + chunk-to-source entity resolution
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_vector_chunk_to_source_entity() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    // Two files with distinct bodies
    catalog.create(
        "File",
        make_file(
            "auth.ts",
            "/repo/src/auth.ts",
            "Authentication module handling JWT tokens, session management, and user login.",
        ),
    ).unwrap();
    catalog.create(
        "File",
        make_file(
            "logger.ts",
            "/repo/src/logger.ts",
            "Logging utility for console and file output with log levels and rotation.",
        ),
    ).unwrap();

    let result = catalog.drain().await;
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert_eq!(result.failed, 0);

    // FileKB should have 2 index entries
    let idx_count = query_count(&catalog, "MATCH (f:FileKB_Index) RETURN count(f)").await;
    assert_eq!(idx_count, 2, "FileKB should have 2 index entries");

    // Chunks should exist and be linked via SOURCED
    let chunk_count = query_count(&catalog, "MATCH (c:FileKB_Index_Chunk) RETURN count(c)").await;
    assert!(chunk_count >= 2, "FileKB should have at least 2 chunks");

    // Verify SOURCED rels link chunks back to the correct File
    let sourced_rows = query_rows(
        &catalog,
        "MATCH (f:File)-[:File_SOURCED_FileKB]->(c:FileKB_Index_Chunk) \
         RETURN f.name, c._text",
    ).await;
    assert!(!sourced_rows.is_empty(), "SOURCED rels should exist");
    for row in &sourced_rows {
        let file_name = row[0].as_str().unwrap_or("");
        let chunk_text = row[1].as_str().unwrap_or("");
        eprintln!("  SOURCED: {} -> '{}'", file_name, &chunk_text[..chunk_text.len().min(50)]);
        // Each chunk should belong to the right file
        if chunk_text.contains("JWT") || chunk_text.contains("session") || chunk_text.contains("login") {
            assert_eq!(file_name, "auth.ts", "auth chunk should be sourced from auth.ts");
        }
        if chunk_text.contains("Logging") || chunk_text.contains("rotation") {
            assert_eq!(file_name, "logger.ts", "logger chunk should be sourced from logger.ts");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 5: _content_offset verified arithmetically
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_content_offset_arithmetic() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    let dir_ref = catalog.create("Directory", make_directory("src", "/app/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("main.rs", "/app/src/main.rs", "fn main() { println!(\"Hello\"); }"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain().await;
    assert_eq!(result.failed, 0);

    // Get the concatenated _content
    let content_rows = query_rows(
        &catalog,
        "MATCH (t:TreeKB_Index) RETURN t._content",
    ).await;
    assert_eq!(content_rows.len(), 1);
    let full_content = content_rows[0][0].as_str().unwrap();
    eprintln!("TreeKB _content: '{full_content}' (len={})", full_content.len());

    // Get all chunks with their offsets
    let chunk_rows = query_rows(
        &catalog,
        "MATCH (c:TreeKB_Index_Chunk) \
         RETURN c._text, c._start_char, c._end_char, c._content_offset, c._source_field \
         ORDER BY c._content_offset, c._start_char",
    ).await;
    assert!(!chunk_rows.is_empty(), "Should have TreeKB chunks");

    for row in &chunk_rows {
        let chunk_text = row[0].as_str().unwrap_or("");
        let start_char = row[1].as_i64().unwrap() as usize;
        let end_char = row[2].as_i64().unwrap() as usize;
        let content_offset = row[3].as_i64().unwrap() as usize;
        let source_field = row[4].as_str().unwrap_or("");

        eprintln!(
            "  chunk: field={source_field}, offset={content_offset}, start={start_char}, end={end_char}, text='{chunk_text}'"
        );

        // Verify: full_content[content_offset + start_char .. content_offset + end_char]
        // should contain the chunk text (modulo trimming)
        let global_start = content_offset + start_char;
        let global_end = content_offset + end_char;
        assert!(
            global_end <= full_content.len(),
            "global_end ({global_end}) should be <= full_content.len() ({})",
            full_content.len()
        );
        let extracted = &full_content[global_start..global_end];
        assert!(
            extracted.contains(chunk_text.trim()) || chunk_text.trim().contains(extracted.trim()),
            "Extracted text should match chunk text.\n  extracted: '{extracted}'\n  chunk:     '{chunk_text}'"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 6: Delete contentFor-only entity → re-aggregate
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_delete_content_for_only() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate() {}"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain().await;
    assert_eq!(result.failed, 0);

    // Verify File content is in TreeKB
    let content_before = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content").await;
    let content_str = content_before[0][0].as_str().unwrap();
    assert!(content_str.contains("auth.ts"), "Before delete: content should contain 'auth.ts'");
    let hash_before = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content_hash").await;
    let hash_str = hash_before[0][0].as_str().unwrap().to_string();

    // Delete the File (contentFor-only for TreeKB)
    let file_uuid = file_ref.uuid().unwrap();
    catalog.delete("File", &file_uuid).await.unwrap();

    // Drain the AggregateOp that was enqueued by delete
    let drain2 = catalog.drain().await;
    eprintln!("drain after delete: processed={}, failed={}", drain2.processed, drain2.failed);
    assert_eq!(drain2.failed, 0);

    // TreeKB content should no longer contain File data
    let content_after = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content").await;
    let content_after_str = content_after[0][0].as_str().unwrap();
    assert!(
        !content_after_str.contains("auth.ts"),
        "After delete: content should NOT contain 'auth.ts', got: '{content_after_str}'"
    );
    assert!(
        !content_after_str.contains("/repo/src/auth.ts"),
        "After delete: content should NOT contain File.absolute_path"
    );

    // Hash should have changed
    let hash_after = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content_hash").await;
    let hash_after_str = hash_after[0][0].as_str().unwrap();
    assert_ne!(hash_str, hash_after_str, "content_hash should change after delete");

    // File SOURCED rels should be gone
    let file_sourced = query_count(
        &catalog,
        "MATCH (:File)-[:File_SOURCED_TreeKB]->(:TreeKB_Index_Chunk) RETURN count(*)",
    ).await;
    assert_eq!(file_sourced, 0, "File SOURCED rels should be deleted");

    // BM25 search for "auth" should return 0 results
    let response = catalog.search(
        "TreeKB",
        "auth",
        SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
    ).await.unwrap();
    assert_eq!(response.results.len(), 0, "After delete, 'auth' should not be found in TreeKB");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 7: Update contentFor-only entity → re-aggregate
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_update_content_for_only() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate() {}"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain().await;
    assert_eq!(result.failed, 0);

    // Verify initial state
    let content_before = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content").await;
    assert!(content_before[0][0].as_str().unwrap().contains("auth.ts"));

    // Update the File: rename to login.ts
    let file_uuid = file_ref.uuid().unwrap();
    let mut update_data = BTreeMap::new();
    update_data.insert("name".into(), CypherValue::String("login.ts".into()));
    update_data.insert("absolute_path".into(), CypherValue::String("/repo/src/login.ts".into()));
    catalog.update("File", &file_uuid, update_data).await.unwrap();

    // Drain the AggregateOp enqueued by update
    let drain2 = catalog.drain().await;
    assert_eq!(drain2.failed, 0);
    assert_eq!(drain2.failed, 0);

    // TreeKB content should now contain "login.ts" instead of "auth.ts"
    let content_after = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content").await;
    let content_str = content_after[0][0].as_str().unwrap();
    assert!(
        content_str.contains("login.ts"),
        "After update: content should contain 'login.ts', got: '{content_str}'"
    );
    assert!(
        !content_str.contains("auth.ts"),
        "After update: content should NOT contain 'auth.ts', got: '{content_str}'"
    );

    // Search should find "login" but not "auth"
    let response_login = catalog.search(
        "TreeKB",
        "login",
        SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
    ).await.unwrap();
    assert!(response_login.results.len() > 0, "TreeKB should find 'login' after update");

    let response_auth = catalog.search(
        "TreeKB",
        "auth",
        SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
    ).await.unwrap();
    assert_eq!(response_auth.results.len(), 0, "TreeKB should NOT find 'auth' after update");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 8: Title truncation (title_max_chars)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_title_truncation() {
    let chunking = ChunkingConfig {
        title_max_chars: 20,
        ..Default::default()
    };
    let mut catalog = make_catalog_with_chunking(chunking).await;
    catalog.initialize().await.unwrap();

    // Create a File with a very long name (100 chars)
    let long_name = "a".repeat(100);
    catalog.create(
        "File",
        make_file(&long_name, "/repo/long_name_file.ts", "Some body content here."),
    ).unwrap();

    let result = catalog.drain().await;
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert_eq!(result.failed, 0);

    // FileKB_Index._title should be truncated to 20 chars
    let rows = query_rows(
        &catalog,
        "MATCH (f:FileKB_Index) RETURN f._title",
    ).await;
    assert_eq!(rows.len(), 1);
    let title = rows[0][0].as_str().unwrap();
    eprintln!("FileKB_Index._title: '{}' (len={})", title, title.len());
    assert!(
        title.len() <= 20,
        "Title should be truncated to <= 20 chars, got {} chars",
        title.len()
    );

    // Chunks should still have correct offsets (relative to body, not affected by title)
    let chunk_rows = query_rows(
        &catalog,
        "MATCH (c:FileKB_Index_Chunk) RETURN c._start_char, c._end_char, c._text",
    ).await;
    for row in &chunk_rows {
        let start = row[0].as_i64().unwrap() as usize;
        let end = row[1].as_i64().unwrap() as usize;
        let text = row[2].as_str().unwrap();
        eprintln!("  chunk: start={start}, end={end}, text='{text}'");
        assert!(end > start, "end_char should be > start_char");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 9: SOURCED rels multi-entity correctness
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_sourced_rels_multi_entity() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    let dir_ref = catalog.create("Directory", make_directory("components", "/repo/components/")).unwrap();
    let file1_ref = catalog.create(
        "File",
        make_file("Button.tsx", "/repo/components/Button.tsx", "export const Button = () => {};"),
    ).unwrap();
    let file2_ref = catalog.create(
        "File",
        make_file("Modal.tsx", "/repo/components/Modal.tsx", "export const Modal = () => {};"),
    ).unwrap();

    catalog.link("HAS_FILE", dir_ref.clone(), file1_ref.clone(), BTreeMap::new()).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file2_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain().await;
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert_eq!(result.failed, 0);

    // Directory SOURCED → chunks from Directory's own fields (absolute_path)
    let dir_sourced = query_rows(
        &catalog,
        "MATCH (d:Directory)-[:Directory_SOURCED_TreeKB]->(c:TreeKB_Index_Chunk) \
         RETURN d.name, c._source_field, c._text",
    ).await;
    eprintln!("Directory SOURCED chunks: {}", dir_sourced.len());
    for row in &dir_sourced {
        let dname = row[0].as_str().unwrap_or("");
        let field = row[1].as_str().unwrap_or("");
        let text = row[2].as_str().unwrap_or("");
        eprintln!("  Directory.{dname} -> field={field}, text='{text}'");
        assert_eq!(dname, "components", "Only our Directory should source these chunks");
    }

    // File SOURCED → chunks from File's contentFor fields (name, absolute_path)
    let file_sourced = query_rows(
        &catalog,
        "MATCH (f:File)-[:File_SOURCED_TreeKB]->(c:TreeKB_Index_Chunk) \
         RETURN f.name, c._source_field, c._text \
         ORDER BY f.name",
    ).await;
    eprintln!("File SOURCED chunks: {}", file_sourced.len());
    for row in &file_sourced {
        let fname = row[0].as_str().unwrap_or("");
        let field = row[1].as_str().unwrap_or("");
        let text = row[2].as_str().unwrap_or("");
        eprintln!("  File.{fname} -> field={field}, text='{text}'");
    }

    // Both files should have SOURCED rels
    let file_names: Vec<&str> = file_sourced.iter()
        .filter_map(|r| r[0].as_str())
        .collect();
    assert!(file_names.contains(&"Button.tsx"), "Button.tsx should have SOURCED rels");
    assert!(file_names.contains(&"Modal.tsx"), "Modal.tsx should have SOURCED rels");

    // No chunk should be SOURCED from a wrong entity
    // (e.g., a Directory chunk shouldn't be SOURCED from a File)
    let cross_check = query_count(
        &catalog,
        "MATCH (f:File)-[:Directory_SOURCED_TreeKB]->(c:TreeKB_Index_Chunk) RETURN count(*)",
    ).await;
    assert_eq!(cross_check, 0, "No File should have Directory_SOURCED_TreeKB rels");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 10: Aggregate idempotent (hash unchanged → skip)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_aggregate_skip_unchanged() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("index.ts", "/repo/src/index.ts", "export default {};"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    // First drain: full processing
    let drain1 = catalog.drain().await;
    assert_eq!(drain1.failed, 0);

    // Record hash and chunk count
    let hash1_rows = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content_hash, t._uuid").await;
    let hash1 = hash1_rows[0][0].as_str().unwrap().to_string();
    let _idx_uuid = hash1_rows[0][1].as_str().unwrap().to_string();
    let chunk_count1 = query_count(&catalog, "MATCH (c:TreeKB_Index_Chunk) RETURN count(c)").await;

    eprintln!("After drain 1: hash={hash1}, chunks={chunk_count1}");

    // Manually enqueue another AggregateOp for the same index entry
    // We do this by updating the Directory with the same data (no actual change to entity,
    // but it triggers re-aggregate)
    let dir_uuid = dir_ref.uuid().unwrap();
    let mut same_data = BTreeMap::new();
    same_data.insert("name".into(), CypherValue::String("src".into()));
    catalog.update("Directory", &dir_uuid, same_data).await.unwrap();

    // Second drain: AggregateProcessor should detect hash unchanged and skip
    let drain2 = catalog.drain().await;
    eprintln!("After drain 2: processed={}, failed={}", drain2.processed, drain2.failed);
    assert_eq!(drain2.failed, 0);

    // Hash should be identical
    let hash2_rows = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content_hash").await;
    let hash2 = hash2_rows[0][0].as_str().unwrap();
    assert_eq!(hash1, hash2, "content_hash should be unchanged after re-aggregate with same content");

    // Chunk count should be the same
    let chunk_count2 = query_count(&catalog, "MATCH (c:TreeKB_Index_Chunk) RETURN count(c)").await;
    assert_eq!(chunk_count1, chunk_count2, "chunk count should be unchanged");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 11: link() incremental triggers AggregateOp
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_link_incremental_aggregate() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    // Create Directory and drain (no files linked yet)
    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let drain1 = catalog.drain().await;
    assert_eq!(drain1.failed, 0);

    // TreeKB should have the Directory's content only
    let content1 = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content").await;
    let content1_str = content1[0][0].as_str().unwrap();
    assert!(!content1_str.contains("utils.ts"), "Before link: no File content in TreeKB");

    // Create a File and drain (entity exists, but not linked to Directory yet)
    let file_ref = catalog.create(
        "File",
        make_file("utils.ts", "/repo/src/utils.ts", "export function helper() {}"),
    ).unwrap();
    let drain2 = catalog.drain().await;
    assert_eq!(drain2.failed, 0);

    // Now link File to Directory — should trigger incremental AggregateOp
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();
    let drain3 = catalog.drain().await;
    eprintln!("drain after link: processed={}, failed={}", drain3.processed, drain3.failed);
    assert_eq!(drain3.failed, 0);

    // TreeKB content should now include the File's data
    let content2 = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content").await;
    let content2_str = content2[0][0].as_str().unwrap();
    assert!(
        content2_str.contains("utils.ts"),
        "After link: TreeKB should contain File.name 'utils.ts', got: '{content2_str}'"
    );
    assert!(
        content2_str.contains("/repo/src/utils.ts"),
        "After link: TreeKB should contain File.absolute_path"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 12: Multiple files + delete one → only that file's content removed
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_delete_one_of_multiple_files() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file1 = catalog.create(
        "File",
        make_file("alpha.ts", "/repo/src/alpha.ts", "alpha content"),
    ).unwrap();
    let file2 = catalog.create(
        "File",
        make_file("beta.ts", "/repo/src/beta.ts", "beta content"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file1.clone(), BTreeMap::new()).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file2.clone(), BTreeMap::new()).unwrap();

    let drain1 = catalog.drain().await;
    assert_eq!(drain1.failed, 0);

    // Both files should be in TreeKB
    let content1 = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content").await;
    let c1 = content1[0][0].as_str().unwrap();
    assert!(c1.contains("alpha.ts"), "TreeKB should contain alpha.ts");
    assert!(c1.contains("beta.ts"), "TreeKB should contain beta.ts");

    // Delete alpha.ts
    let alpha_uuid = file1.uuid().unwrap();
    catalog.delete("File", &alpha_uuid).await.unwrap();
    let drain2 = catalog.drain().await;
    assert_eq!(drain2.failed, 0);

    // Only beta.ts should remain
    let content2 = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._content").await;
    let c2 = content2[0][0].as_str().unwrap();
    assert!(
        !c2.contains("alpha.ts"),
        "After delete: TreeKB should NOT contain alpha.ts, got: '{c2}'"
    );
    assert!(
        c2.contains("beta.ts"),
        "After delete: TreeKB should still contain beta.ts, got: '{c2}'"
    );

    // Search should find beta but not alpha
    let r_beta = catalog.search("TreeKB", "beta", SearchOptions {
        consistency: Consistency::Immediate,
        ..Default::default()
    }).await.unwrap();
    assert!(r_beta.results.len() > 0, "Should find 'beta' after deleting alpha");

    let r_alpha = catalog.search("TreeKB", "alpha", SearchOptions {
        consistency: Consistency::Immediate,
        ..Default::default()
    }).await.unwrap();
    assert_eq!(r_alpha.results.len(), 0, "Should NOT find 'alpha' after deletion");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test DEBUG: Full pipeline trace with queue events
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_debug_trace_pipeline() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    // Subscribe to queue events
    let mut queue_rx = catalog.subscribe_queue();

    // Create Directory + File + link
    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate() { return true; }"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    eprintln!("\n══ QUEUE AFTER CREATE+LINK (before drain) ══");
    while let Ok(ev) = queue_rx.try_recv() {
        eprintln!("  [Q] {:?}", ev);
    }

    eprintln!("\n══ DRAIN ══");
    let result = catalog.drain().await;
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);

    eprintln!("\n══ QUEUE EVENTS DURING DRAIN ══");
    while let Ok(ev) = queue_rx.try_recv() {
        eprintln!("  [Q] {:?}", ev);
    }

    // Dump DB state
    eprintln!("\n══ DB STATE ══");
    let dirs = query_rows(&catalog, "MATCH (d:Directory) RETURN d._uuid, d.name").await;
    eprintln!("Directories: {:?}", dirs);
    let files = query_rows(&catalog, "MATCH (f:File) RETURN f._uuid, f.name").await;
    eprintln!("Files: {:?}", files);

    let tree_idx = query_rows(&catalog,
        "MATCH (t:TreeKB_Index) RETURN t._uuid, t._title, t._content, t._content_hash, t._source_entity, t._source_uuid"
    ).await;
    eprintln!("TreeKB_Index entries: {}", tree_idx.len());
    for row in &tree_idx {
        eprintln!("  {:?}", row);
    }

    let tree_chunks = query_rows(&catalog,
        "MATCH (c:TreeKB_Index_Chunk) RETURN c._uuid, c._text, c._source_field, c._content_offset, c._start_char, c._end_char"
    ).await;
    eprintln!("TreeKB_Index_Chunk: {}", tree_chunks.len());
    for row in &tree_chunks {
        eprintln!("  {:?}", row);
    }

    let file_idx = query_rows(&catalog,
        "MATCH (f:FileKB_Index) RETURN f._uuid, f._title, f._content, f._content_hash, f._source_entity, f._source_uuid"
    ).await;
    eprintln!("FileKB_Index entries: {}", file_idx.len());
    for row in &file_idx {
        eprintln!("  {:?}", row);
    }

    let file_chunks = query_rows(&catalog,
        "MATCH (c:FileKB_Index_Chunk) RETURN c._uuid, c._text, c._source_field, c._content_offset"
    ).await;
    eprintln!("FileKB_Index_Chunk: {}", file_chunks.len());
    for row in &file_chunks {
        eprintln!("  {:?}", row);
    }

    // SOURCED rels — query each known rel type separately
    let dir_sourced = query_rows(&catalog,
        "MATCH (d:Directory)-[:Directory_SOURCED_TreeKB]->(c:TreeKB_Index_Chunk) RETURN d.name, c._uuid, c._text"
    ).await;
    eprintln!("Directory_SOURCED_TreeKB: {}", dir_sourced.len());
    for row in &dir_sourced { eprintln!("  {:?}", row); }

    let file_sourced_tree = query_rows(&catalog,
        "MATCH (f:File)-[:File_SOURCED_TreeKB]->(c:TreeKB_Index_Chunk) RETURN f.name, c._uuid, c._text"
    ).await;
    eprintln!("File_SOURCED_TreeKB: {}", file_sourced_tree.len());
    for row in &file_sourced_tree { eprintln!("  {:?}", row); }

    let file_sourced_file = query_rows(&catalog,
        "MATCH (f:File)-[:File_SOURCED_FileKB]->(c:FileKB_Index_Chunk) RETURN f.name, c._uuid, c._text"
    ).await;
    eprintln!("File_SOURCED_FileKB: {}", file_sourced_file.len());
    for row in &file_sourced_file { eprintln!("  {:?}", row); }

    // Try Lucivy raw query to check if FTS index has data
    eprintln!("\n══ RAW LUCIVY QUERY ══");
    let fts_result = catalog.execute_raw(
        "CALL QUERY_LUCIVY_INDEX('TreeKB_Index', '{\"type\":\"parse\",\"fields\":[\"_title\",\"_content\"],\"value\":\"auth\"}', 10) RETURN node_id, score"
    ).await;
    match fts_result {
        Ok(r) => {
            eprintln!("Lucivy 'auth' on TreeKB_Index: {} results", r.rows.len());
            for row in &r.rows { eprintln!("  {:?}", row); }
        }
        Err(e) => eprintln!("Lucivy error: {e:?}"),
    }

    // Try search through Catalog API
    eprintln!("\n══ CATALOG SEARCH ══");
    let search_result = catalog.search(
        "TreeKB", "auth",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    ).await;
    match search_result {
        Ok(r) => eprintln!("Catalog search 'auth': {} results, bm25={}", r.results.len(), r.meta.bm25_count),
        Err(e) => eprintln!("Catalog search error: {e:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 14: Isolate Lucivy query modes — Contains vs Parse on same index
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore]
async fn phase0b_lucivy_contains_vs_parse() {
    let mut catalog = make_catalog().await;
    catalog.initialize().await.unwrap();

    let mut queue_rx = catalog.subscribe_queue();

    catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate(req: Request) { return true; }"),
    ).unwrap();
    catalog.link("HAS_FILE", Hashsafe::new("Directory", &["/repo/src/"]), file_ref.clone(), BTreeMap::new()).unwrap();

    eprintln!("\n══ QUEUE AFTER CREATE+LINK ══");
    while let Ok(ev) = queue_rx.try_recv() {
        eprintln!("  [Q] {:?}", ev);
    }

    let result = catalog.drain().await;
    eprintln!("\ndrain: processed={}, failed={}", result.processed, result.failed);

    eprintln!("\n══ QUEUE EVENTS DURING DRAIN ══");
    while let Ok(ev) = queue_rx.try_recv() {
        eprintln!("  [Q] {:?}", ev);
    }

    // Dump what's in the index
    let idx = query_rows(&catalog, "MATCH (t:TreeKB_Index) RETURN t._uuid, t._title, t._content").await;
    eprintln!("\nTreeKB_Index rows:");
    for row in &idx { eprintln!("  {:?}", row); }

    let chunks = query_rows(&catalog, "MATCH (c:TreeKB_Index_Chunk) RETURN c._uuid, c._text, c._source_field").await;
    eprintln!("TreeKB_Index_Chunk rows:");
    for row in &chunks { eprintln!("  {:?}", row); }

    // ── Test raw Lucivy queries directly ──
    let queries = vec![
        ("parse, fields=[_title,_content], 'auth'",
         r#"{"type":"parse","fields":["_title","_content"],"value":"auth"}"#),
        ("parse, field=_content, 'auth'",
         r#"{"type":"parse","field":"_content","value":"auth"}"#),
        ("parse, field=_title, 'src'",
         r#"{"type":"parse","field":"_title","value":"src"}"#),
        ("contains, field=_content, 'auth', distance=1",
         r#"{"type":"contains","field":"_content","value":"auth","distance":1}"#),
        ("contains, field=_title, 'src', distance=1",
         r#"{"type":"contains","field":"_title","value":"src","distance":1}"#),
        ("contains, field=_content, 'auth', distance=0",
         r#"{"type":"contains","field":"_content","value":"auth","distance":0}"#),
        ("boolean should [contains _title + _content], 'auth'",
         r#"{"type":"boolean","should":[{"type":"contains","field":"_title","value":"auth","distance":1},{"type":"contains","field":"_content","value":"auth","distance":1}]}"#),
        ("contains, field=_content, 'authenticate', distance=1",
         r#"{"type":"contains","field":"_content","value":"authenticate","distance":1}"#),
    ];

    eprintln!("\n══ RAW LUCIVY QUERY COMPARISON ══");
    for (label, json) in &queries {
        let escaped = json.replace('\'', "''");
        let cypher = format!(
            "CALL QUERY_LUCIVY_INDEX('TreeKB_Index', '{}', 10) RETURN node_id, score, highlights",
            escaped,
        );
        match catalog.execute_raw(&cypher).await {
            Ok(r) => {
                eprintln!("\n  {} → {} results", label, r.rows.len());
                for row in &r.rows { eprintln!("    {:?}", row); }
            }
            Err(e) => eprintln!("\n  {} → ERROR: {:?}", label, e),
        }
    }
}

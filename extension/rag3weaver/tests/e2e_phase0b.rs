//! E2E integration tests: Phase 0b — KB multi-entité rendue en entité dérivée
//! (une ligne par racine, chunks portant la racine), résolution
//! surlignage→chunk, _content_offset, titre, propagation delete/update d'une
//! entité contributrice (contentFor seulement).
//!
//! Config: TreeKB (multi-entity, BM25 only) + FileKB (single-entity, BM25+vector).
//! Les deux KB sont traduites en entités dérivées : `TreeKB` depuis Directory
//! (rassemble File par HAS_FILE), `FileKB` depuis File. Tables `{KB}` et
//! `{KB}_Chunk`, relations `{KB}_DERIVED_FROM` (dérivée → racine) et
//! `{KB}_CHUNKED_FROM` (chunk → dérivée).
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
use rag3weaver::{Catalog, Rag3dbConnection, hashsafe_uuid};
use rag3weaver::disponibilite::RegimeEcriture;

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
            derived_from: None,
        },
    );
    entities.insert(
        "File".into(),
        EntityDef {
            fields: file_fields,
            hashsafe: Some(vec!["absolute_path".into()]),
            derived_from: None,
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
            Ok(_) => eprintln!("  loaded {name}"),
            Err(e) => panic!("Failed to load {name} from {ext_path}: {e}"),
        }
    }
}

fn make_catalog() -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    Catalog::new(boxed, Box::new(MockEmbedder::new(4)), make_phase0b_config()).avec_regime(RegimeEcriture::ParLot)
}

/// Same as make_catalog but with a custom ChunkingConfig override.
fn make_catalog_with_chunking(chunking: ChunkingConfig) -> Catalog {
    let conn = Rag3dbConnection::in_memory().expect("in-memory DB");
    let boxed: Box<dyn rag3weaver::connection::DbConnection> = Box::new(conn);
    load_extensions(boxed.as_ref());
    let mut config = make_phase0b_config();
    for kb in config.knowledge_bases.values_mut() {
        kb.chunking = chunking.clone();
    }
    Catalog::new(boxed, Box::new(MockEmbedder::new(4)), config).avec_regime(RegimeEcriture::ParLot)
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
fn query_rows(catalog: &Catalog, cypher: &str) -> Vec<Vec<CypherValue>> {
    let result = catalog.execute_raw(cypher).unwrap();
    result.rows
}

/// Query helper: return the single scalar value from a COUNT query.
fn query_count(catalog: &Catalog, cypher: &str) -> i64 {
    let rows = query_rows(catalog, cypher);
    rows.first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 1: Ingestion + schema validation
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn phase0b_ingest_and_schema() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

    // Schema should have created all required tables
    // Entity tables
    assert!(catalog.get_entity_def("Directory").is_some());
    assert!(catalog.get_entity_def("File").is_some());
    assert!(catalog.get_relation_def("HAS_FILE").is_some());

    // Les bases sont des entités dérivées : TreeKB de Directory, qui
    // rassemble File ; FileKB de File.
    let tree_kb = catalog.entity_configs().get("TreeKB").expect("TreeKB traduite");
    let tree_d = tree_kb.derived.as_ref().expect("dérivée");
    assert_eq!(tree_d.from, "Directory");
    assert!(tree_d.render["title"].starts_with("{{ root.name"), "{}", tree_d.render["title"]);
    assert!(tree_d.gather.iter().any(|g| g.name == "File"), "{:?}", tree_d.gather);

    let file_kb = catalog.entity_configs().get("FileKB").expect("FileKB traduite");
    let file_d = file_kb.derived.as_ref().expect("dérivée");
    assert_eq!(file_d.from, "File");
    assert!(file_d.render["title"].starts_with("{{ root.name"), "{}", file_d.render["title"]);

    // Create entities
    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate(req: Request) { return validateToken(req.headers.authorization); }"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain();
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert_eq!(result.failed, 0);

    // Verify entity counts
    assert_eq!(catalog.count("Directory").unwrap(), 1);
    assert_eq!(catalog.count("File").unwrap(), 1);

    // TreeKB : une ligne dérivée par racine (Directory)
    let tree_count = query_count(&catalog, "MATCH (t:TreeKB) RETURN count(t)");
    assert_eq!(tree_count, 1, "TreeKB should have 1 derived row (for the Directory)");

    // La ligne TreeKB rend le contenu de Directory + File, et porte sa racine
    let rows = query_rows(
        &catalog,
        "MATCH (t:TreeKB) RETURN t.title, t.content, t._content_hash, t._source_entity, t._source_uuid, t._uuid",
    );
    assert_eq!(rows.len(), 1);
    let title = rows[0][0].as_str().unwrap_or("");
    let content = rows[0][1].as_str().unwrap_or("");
    let content_hash = rows[0][2].as_str().unwrap_or("");
    let source_entity = rows[0][3].as_str().unwrap_or("");
    let source_uuid = rows[0][4].as_str().unwrap_or("");
    let derived_uuid = rows[0][5].as_str().unwrap_or("");
    assert_eq!(title, "src", "TreeKB title should be Directory.name");
    assert!(content.contains("/repo/src/"), "TreeKB content should contain Directory.absolute_path");
    assert!(content.contains("auth.ts"), "TreeKB content should contain File.name");
    assert!(content.contains("/repo/src/auth.ts"), "TreeKB content should contain File.absolute_path");
    assert!(!content_hash.is_empty(), "content_hash should be set (not sentinel)");
    let dir_uuid = dir_ref.uuid().unwrap();
    assert_eq!(source_entity, "Directory", "_source_entity = entité racine");
    assert_eq!(source_uuid, dir_uuid, "_source_uuid = uuid de la racine");
    assert_eq!(
        derived_uuid,
        rag3weaver::dataflow::derive_nodes::derived_uuid("TreeKB", &dir_uuid),
        "l'uuid de la dérivée est déterministe depuis la racine"
    );

    // La dérivée pointe sa racine : TreeKB_DERIVED_FROM (dérivée → Directory)
    let derived_from = query_count(
        &catalog,
        "MATCH (:TreeKB)-[:TreeKB_DERIVED_FROM]->(:Directory) RETURN count(*)",
    );
    assert_eq!(derived_from, 1, "TreeKB row should be linked to its Directory root");

    // TreeKB_Chunk : des chunks, chacun rattaché à sa ligne dérivée (chunk → parent)
    let chunk_count = query_count(&catalog, "MATCH (c:TreeKB_Chunk) RETURN count(c)");
    assert!(chunk_count > 0, "TreeKB should have chunks: got {chunk_count}");
    let chunked_from = query_count(
        &catalog,
        "MATCH (:TreeKB_Chunk)-[:TreeKB_CHUNKED_FROM]->(:TreeKB) RETURN count(*)",
    );
    assert_eq!(chunked_from, chunk_count, "every TreeKB chunk should be linked to its derived row");

    // Les chunks d'une dérivée portent la racine (_source_entity/_source_uuid)
    let chunks_from_dir = query_count(
        &catalog,
        "MATCH (c:TreeKB_Chunk) WHERE c._source_entity = 'Directory' RETURN count(c)",
    );
    assert_eq!(chunks_from_dir, chunk_count, "every TreeKB chunk should carry the Directory root");

    // FileKB : une ligne dérivée par File
    let file_kb_count = query_count(&catalog, "MATCH (f:FileKB) RETURN count(f)");
    assert_eq!(file_kb_count, 1, "FileKB should have 1 derived row");

    // FileKB chunks, rattachés et portant leur racine File
    let filekb_chunk_count = query_count(&catalog, "MATCH (c:FileKB_Chunk) RETURN count(c)");
    assert!(filekb_chunk_count > 0, "FileKB should have chunks: got {filekb_chunk_count}");
    let filekb_chunks_from_file = query_count(
        &catalog,
        "MATCH (c:FileKB_Chunk) WHERE c._source_entity = 'File' RETURN count(c)",
    );
    assert_eq!(filekb_chunks_from_file, filekb_chunk_count, "every FileKB chunk should carry the File root");

    // Plus de liens `_SOURCED_` par contributrice : le contenu rendu est un seul
    // texte, ses chunks ne sont plus attribués à chaque entité contributrice.

    eprintln!(
        "Schema OK: TreeKB chunks={chunk_count}, FileKB chunks={filekb_chunk_count}, \
         DERIVED_FROM: tree→dir={derived_from}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 2: BM25 search on multi-entity KB (TreeKB)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn phase0b_bm25_search_multi_entity() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

    catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate(req: Request) { return true; }"),
    ).unwrap();
    catalog.create("Directory", make_directory("lib", "/repo/lib/")).unwrap();

    catalog.link("HAS_FILE", hashsafe_uuid("Directory", &["/repo/src/"]), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain();
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
    ).unwrap();

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
    ).unwrap();
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
    ).unwrap();
    assert_eq!(response3.results.len(), 0, "nonsense query should return 0 results");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 3: BM25 highlight → chunk resolution (single-entity FileKB)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn phase0b_bm25_highlight_chunk_single_entity() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

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

    let result = catalog.drain();
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
    ).unwrap();

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

#[test]
#[ignore]
fn phase0b_vector_chunk_to_source_entity() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

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

    let result = catalog.drain();
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert_eq!(result.failed, 0);

    // FileKB : une ligne dérivée par File
    let idx_count = query_count(&catalog, "MATCH (f:FileKB) RETURN count(f)");
    assert_eq!(idx_count, 2, "FileKB should have 2 derived rows");

    // Des chunks, rattachés à leur ligne dérivée
    let chunk_count = query_count(&catalog, "MATCH (c:FileKB_Chunk) RETURN count(c)");
    assert!(chunk_count >= 2, "FileKB should have at least 2 chunks");

    // Chaque chunk remonte au bon File : chunk → dérivée (CHUNKED_FROM) → racine
    // (DERIVED_FROM), et son _source_uuid est l'uuid de ce File.
    let sourced_rows = query_rows(
        &catalog,
        "MATCH (c:FileKB_Chunk)-[:FileKB_CHUNKED_FROM]->(:FileKB)-[:FileKB_DERIVED_FROM]->(f:File) \
         RETURN f.name, c._text, c._source_uuid, f._uuid",
    );
    assert!(!sourced_rows.is_empty(), "chunks should resolve to their File root");
    assert_eq!(sourced_rows.len() as i64, chunk_count, "every chunk should resolve to exactly one File");
    for row in &sourced_rows {
        let file_name = row[0].as_str().unwrap_or("");
        let chunk_text = row[1].as_str().unwrap_or("");
        let source_uuid = row[2].as_str().unwrap_or("");
        let file_uuid = row[3].as_str().unwrap_or("");
        eprintln!("  chunk of {} -> '{}'", file_name, &chunk_text[..chunk_text.len().min(50)]);
        assert_eq!(source_uuid, file_uuid, "chunk._source_uuid should be the File root uuid");
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

#[test]
#[ignore]
fn phase0b_content_offset_arithmetic() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

    let dir_ref = catalog.create("Directory", make_directory("src", "/app/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("main.rs", "/app/src/main.rs", "fn main() { println!(\"Hello\"); }"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain();
    assert_eq!(result.failed, 0);

    // Le contenu rendu de la dérivée (un seul champ `content`)
    let content_rows = query_rows(
        &catalog,
        "MATCH (t:TreeKB) RETURN t.content",
    );
    assert_eq!(content_rows.len(), 1);
    let full_content = content_rows[0][0].as_str().unwrap();
    eprintln!("TreeKB content: '{full_content}' (len={})", full_content.len());

    // Le gabarit traduit : entités contributrices par nom (Directory < File),
    // champs de contenu triés (absolute_path < name), chaque valeur suivie
    // d'un saut de ligne.
    assert_eq!(
        full_content, "/app/src/\n/app/src/main.rs\nmain.rs\n",
        "rendered content should follow the translated template"
    );

    // Get all chunks with their offsets
    let chunk_rows = query_rows(
        &catalog,
        "MATCH (c:TreeKB_Chunk) \
         RETURN c._text, c._start_char, c._end_char, c._content_offset, c._parent_field \
         ORDER BY c._content_offset, c._start_char",
    );
    assert!(!chunk_rows.is_empty(), "Should have TreeKB chunks");

    for row in &chunk_rows {
        let chunk_text = row[0].as_str().unwrap_or("");
        let start_char = row[1].as_i64().unwrap() as usize;
        let end_char = row[2].as_i64().unwrap() as usize;
        let content_offset = row[3].as_i64().unwrap() as usize;
        let parent_field = row[4].as_str().unwrap_or("");

        eprintln!(
            "  chunk: field={parent_field}, offset={content_offset}, start={start_char}, end={end_char}, text='{chunk_text}'"
        );
        // Un seul champ de contenu sur la dérivée : l'offset est toujours 0
        assert_eq!(parent_field, "content", "derived chunks come from the single `content` field");
        assert_eq!(content_offset, 0, "single content field: offset is 0");

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

#[test]
#[ignore]
fn phase0b_delete_content_for_only() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate() {}"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain();
    assert_eq!(result.failed, 0);

    // Verify File content is in TreeKB
    let content_before = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t.content");
    let content_str = content_before[0][0].as_str().unwrap();
    assert!(content_str.contains("auth.ts"), "Before delete: content should contain 'auth.ts'");
    let hash_before = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t._content_hash");
    let hash_str = hash_before[0][0].as_str().unwrap().to_string();

    // Delete the File (contentFor-only for TreeKB)
    let file_uuid = file_ref.uuid().unwrap();
    catalog.delete("File", &file_uuid).unwrap();

    // Drain the derivation enqueued by delete (the Directory root is re-rendered)
    let drain2 = catalog.drain();
    eprintln!("drain after delete: processed={}, failed={}", drain2.processed, drain2.failed);
    assert_eq!(drain2.failed, 0);

    // TreeKB content should no longer contain File data
    let content_after = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t.content");
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
    let hash_after = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t._content_hash");
    let hash_after_str = hash_after[0][0].as_str().unwrap();
    assert_ne!(hash_str, hash_after_str, "content_hash should change after delete");

    // Plus de liens `_SOURCED_` : on vérifie à la place qu'aucun chunk de TreeKB
    // ne porte encore le texte du File supprimé.
    let stale_chunks = query_count(
        &catalog,
        "MATCH (c:TreeKB_Chunk) WHERE c._text CONTAINS 'auth.ts' RETURN count(c)",
    );
    assert_eq!(stale_chunks, 0, "no TreeKB chunk should still carry the deleted File's text");

    // BM25 search for "auth" should return 0 results
    let response = catalog.search(
        "TreeKB",
        "auth",
        SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
    ).unwrap();
    assert_eq!(response.results.len(), 0, "After delete, 'auth' should not be found in TreeKB");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 7: Update contentFor-only entity → re-aggregate
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn phase0b_update_content_for_only() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate() {}"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain();
    assert_eq!(result.failed, 0);

    // Verify initial state
    let content_before = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t.content");
    assert!(content_before[0][0].as_str().unwrap().contains("auth.ts"));

    // Update the File: rename to login.ts
    let file_uuid = file_ref.uuid().unwrap();
    let mut update_data = BTreeMap::new();
    update_data.insert("name".into(), CypherValue::String("login.ts".into()));
    update_data.insert("absolute_path".into(), CypherValue::String("/repo/src/login.ts".into()));
    catalog.update("File", &file_uuid, update_data).unwrap();

    // Drain the derivation enqueued by update (the Directory root is re-rendered)
    let drain2 = catalog.drain();
    assert_eq!(drain2.failed, 0);

    // TreeKB content should now contain "login.ts" instead of "auth.ts"
    let content_after = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t.content");
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
    ).unwrap();
    assert!(response_login.results.len() > 0, "TreeKB should find 'login' after update");

    let response_auth = catalog.search(
        "TreeKB",
        "auth",
        SearchOptions {
            consistency: Consistency::Immediate,
            ..Default::default()
        },
    ).unwrap();
    assert_eq!(response_auth.results.len(), 0, "TreeKB should NOT find 'auth' after update");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 8: Titre long (title_max_chars) — le titre rendu, les chunks intacts
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn phase0b_title_truncation() {
    let chunking = ChunkingConfig {
        title_max_chars: 20,
        ..Default::default()
    };
    let mut catalog = make_catalog_with_chunking(chunking);
    catalog.initialize().unwrap();

    // Create a File with a very long name (100 chars)
    let long_name = "a".repeat(100);
    catalog.create(
        "File",
        make_file(&long_name, "/repo/long_name_file.ts", "Some body content here."),
    ).unwrap();

    let result = catalog.drain();
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert_eq!(result.failed, 0);

    // Le titre de la dérivée est rendu par gabarit, borné à `title_max_chars`
    // (`{{ root.name[:20] }}`) : comme l'ancienne ligne d'index.
    let rows = query_rows(
        &catalog,
        "MATCH (f:FileKB) RETURN f.title",
    );
    assert_eq!(rows.len(), 1);
    let title = rows[0][0].as_str().unwrap();
    eprintln!("FileKB.title: '{}' (len={})", title, title.len());
    assert_eq!(title.chars().count(), 20, "derived title bounded by title_max_chars");
    assert!(long_name.starts_with(title));

    // Chunks should still have correct offsets (relative to body, not affected by title)
    let chunk_rows = query_rows(
        &catalog,
        "MATCH (c:FileKB_Chunk) RETURN c._start_char, c._end_char, c._text",
    );
    for row in &chunk_rows {
        let start = row[0].as_i64().unwrap() as usize;
        let end = row[1].as_i64().unwrap() as usize;
        let text = row[2].as_str().unwrap();
        eprintln!("  chunk: start={start}, end={end}, text='{text}'");
        assert!(end > start, "end_char should be > start_char");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 9: Multi-entité — les chunks de la dérivée portent la racine, le contenu
// rendu porte toutes les contributrices
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn phase0b_sourced_rels_multi_entity() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

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

    let result = catalog.drain();
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);
    assert_eq!(result.failed, 0);

    // Plus de liens `_SOURCED_` par contributrice : le contenu rendu est un seul
    // texte. Chaque chunk de TreeKB porte la racine (Directory) et remonte à elle
    // par CHUNKED_FROM → DERIVED_FROM.
    let dir_uuid = dir_ref.uuid().unwrap();
    let chunk_rows = query_rows(
        &catalog,
        "MATCH (c:TreeKB_Chunk)-[:TreeKB_CHUNKED_FROM]->(:TreeKB)-[:TreeKB_DERIVED_FROM]->(d:Directory) \
         RETURN d.name, d._uuid, c._source_entity, c._source_uuid, c._parent_field, c._text",
    );
    eprintln!("TreeKB chunks resolved to their root: {}", chunk_rows.len());
    assert!(!chunk_rows.is_empty(), "TreeKB should have chunks resolving to the Directory");
    for row in &chunk_rows {
        let dname = row[0].as_str().unwrap_or("");
        let duuid = row[1].as_str().unwrap_or("");
        let source_entity = row[2].as_str().unwrap_or("");
        let source_uuid = row[3].as_str().unwrap_or("");
        let field = row[4].as_str().unwrap_or("");
        let text = row[5].as_str().unwrap_or("");
        eprintln!("  Directory.{dname} <- field={field}, text='{text}'");
        assert_eq!(dname, "components", "Only our Directory should be the root of these chunks");
        assert_eq!(duuid, dir_uuid, "root reached by DERIVED_FROM is our Directory");
        assert_eq!(source_entity, "Directory", "chunk._source_entity is the root entity");
        assert_eq!(source_uuid, dir_uuid, "chunk._source_uuid is the root uuid");
    }

    // Tous les chunks sont rattachés : autant de liens CHUNKED_FROM que de chunks
    let chunk_count = query_count(&catalog, "MATCH (c:TreeKB_Chunk) RETURN count(c)");
    assert_eq!(chunk_rows.len() as i64, chunk_count, "every TreeKB chunk should resolve to the root");

    // Le contenu rendu porte les deux fichiers (contributrices via HAS_FILE)
    let content_rows = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t.content");
    assert_eq!(content_rows.len(), 1, "one derived row for the Directory");
    let content = content_rows[0][0].as_str().unwrap_or("");
    eprintln!("TreeKB content: '{content}'");
    assert!(content.contains("Button.tsx"), "content should carry Button.tsx");
    assert!(content.contains("Modal.tsx"), "content should carry Modal.tsx");
    assert!(content.contains("/repo/components/"), "content should carry the Directory path");

    // Les chunks, mis bout à bout, portent aussi les deux fichiers
    let all_text: String = chunk_rows.iter().filter_map(|r| r[5].as_str()).collect::<Vec<_>>().join("");
    assert!(all_text.contains("Button.tsx"), "chunks should carry Button.tsx");
    assert!(all_text.contains("Modal.tsx"), "chunks should carry Modal.tsx");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 10: Dérivation idempotente (entrées inchangées → rien de réécrit)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn phase0b_aggregate_skip_unchanged() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("index.ts", "/repo/src/index.ts", "export default {};"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    // First drain: full processing
    let drain1 = catalog.drain();
    assert_eq!(drain1.failed, 0);

    // Record hashes and chunk count
    let hash1_rows = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t._content_hash, t._render_hash, t._uuid");
    let hash1 = hash1_rows[0][0].as_str().unwrap().to_string();
    let render_hash1 = hash1_rows[0][1].as_str().unwrap().to_string();
    let _derived_uuid = hash1_rows[0][2].as_str().unwrap().to_string();
    let chunk_count1 = query_count(&catalog, "MATCH (c:TreeKB_Chunk) RETURN count(c)");

    eprintln!("After drain 1: hash={hash1}, render_hash={render_hash1}, chunks={chunk_count1}");

    // Enqueue another derivation for the same root: update the Directory with
    // the same data (no actual change to the entity, but the root is re-derived)
    let dir_uuid = dir_ref.uuid().unwrap();
    let mut same_data = BTreeMap::new();
    same_data.insert("name".into(), CypherValue::String("src".into()));
    catalog.update("Directory", &dir_uuid, same_data).unwrap();

    // Second drain: the derive node sees `_render_hash` unchanged and skips
    let drain2 = catalog.drain();
    eprintln!("After drain 2: processed={}, failed={}", drain2.processed, drain2.failed);
    assert_eq!(drain2.failed, 0);

    // Hashes should be identical
    let hash2_rows = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t._content_hash, t._render_hash");
    let hash2 = hash2_rows[0][0].as_str().unwrap();
    let render_hash2 = hash2_rows[0][1].as_str().unwrap();
    assert_eq!(hash1, hash2, "content_hash should be unchanged after re-deriving with same inputs");
    assert_eq!(render_hash1, render_hash2, "render_hash should be unchanged after re-deriving with same inputs");

    // Chunk count should be the same
    let chunk_count2 = query_count(&catalog, "MATCH (c:TreeKB_Chunk) RETURN count(c)");
    assert_eq!(chunk_count1, chunk_count2, "chunk count should be unchanged");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 11: link() met en file une dérivation de la racine
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn phase0b_link_incremental_aggregate() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

    // Create Directory and drain (no files linked yet)
    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let drain1 = catalog.drain();
    assert_eq!(drain1.failed, 0);

    // TreeKB should have the Directory's content only
    let content1 = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t.content");
    let content1_str = content1[0][0].as_str().unwrap();
    assert!(!content1_str.contains("utils.ts"), "Before link: no File content in TreeKB");

    // Create a File and drain (entity exists, but not linked to Directory yet)
    let file_ref = catalog.create(
        "File",
        make_file("utils.ts", "/repo/src/utils.ts", "export function helper() {}"),
    ).unwrap();
    let drain2 = catalog.drain();
    assert_eq!(drain2.failed, 0);

    // Now link File to Directory — enqueues 1 relation + 1 derivation of the root
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();
    let drain3 = catalog.drain();
    eprintln!("drain after link: processed={}, failed={}", drain3.processed, drain3.failed);
    assert_eq!(drain3.failed, 0);

    // TreeKB content should now include the File's data
    let content2 = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t.content");
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

#[test]
#[ignore]
fn phase0b_delete_one_of_multiple_files() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

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

    let drain1 = catalog.drain();
    assert_eq!(drain1.failed, 0);

    // Both files should be in TreeKB
    let content1 = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t.content");
    let c1 = content1[0][0].as_str().unwrap();
    assert!(c1.contains("alpha.ts"), "TreeKB should contain alpha.ts");
    assert!(c1.contains("beta.ts"), "TreeKB should contain beta.ts");

    // Delete alpha.ts
    let alpha_uuid = file1.uuid().unwrap();
    catalog.delete("File", &alpha_uuid).unwrap();
    let drain2 = catalog.drain();
    assert_eq!(drain2.failed, 0);

    // Only beta.ts should remain
    let content2 = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t.content");
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
    }).unwrap();
    assert!(r_beta.results.len() > 0, "Should find 'beta' after deleting alpha");

    let r_alpha = catalog.search("TreeKB", "alpha", SearchOptions {
        consistency: Consistency::Immediate,
        ..Default::default()
    }).unwrap();
    assert_eq!(r_alpha.results.len(), 0, "Should NOT find 'alpha' after deletion");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test DEBUG: Full pipeline trace with queue events
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn phase0b_debug_trace_pipeline() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

    // Create Directory + File + link
    let dir_ref = catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate() { return true; }"),
    ).unwrap();
    catalog.link("HAS_FILE", dir_ref.clone(), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain();
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);

    // Dump DB state
    eprintln!("\n══ DB STATE ══");
    let dirs = query_rows(&catalog, "MATCH (d:Directory) RETURN d._uuid, d.name");
    eprintln!("Directories: {:?}", dirs);
    let files = query_rows(&catalog, "MATCH (f:File) RETURN f._uuid, f.name");
    eprintln!("Files: {:?}", files);

    let tree_rows = query_rows(&catalog,
        "MATCH (t:TreeKB) RETURN t._uuid, t.title, t.content, t._content_hash, t._render_hash, t._source_entity, t._source_uuid"
    );
    eprintln!("TreeKB rows: {}", tree_rows.len());
    for row in &tree_rows {
        eprintln!("  {:?}", row);
    }

    let tree_chunks = query_rows(&catalog,
        "MATCH (c:TreeKB_Chunk) RETURN c._uuid, c._text, c._parent_field, c._content_offset, c._start_char, c._end_char, c._source_entity, c._source_uuid"
    );
    eprintln!("TreeKB_Chunk: {}", tree_chunks.len());
    for row in &tree_chunks {
        eprintln!("  {:?}", row);
    }

    let file_rows = query_rows(&catalog,
        "MATCH (f:FileKB) RETURN f._uuid, f.title, f.content, f._content_hash, f._render_hash, f._source_entity, f._source_uuid"
    );
    eprintln!("FileKB rows: {}", file_rows.len());
    for row in &file_rows {
        eprintln!("  {:?}", row);
    }

    let file_chunks = query_rows(&catalog,
        "MATCH (c:FileKB_Chunk) RETURN c._uuid, c._text, c._parent_field, c._content_offset"
    );
    eprintln!("FileKB_Chunk: {}", file_chunks.len());
    for row in &file_chunks {
        eprintln!("  {:?}", row);
    }

    // Relations des dérivées : dérivée → racine, chunk → dérivée
    // (plus de liens `_SOURCED_` par contributrice)
    let tree_derived_from = query_rows(&catalog,
        "MATCH (t:TreeKB)-[:TreeKB_DERIVED_FROM]->(d:Directory) RETURN t._uuid, d.name"
    );
    eprintln!("TreeKB_DERIVED_FROM: {}", tree_derived_from.len());
    for row in &tree_derived_from { eprintln!("  {:?}", row); }

    let tree_chunked_from = query_rows(&catalog,
        "MATCH (c:TreeKB_Chunk)-[:TreeKB_CHUNKED_FROM]->(t:TreeKB) RETURN c._uuid, t.title, c._text"
    );
    eprintln!("TreeKB_CHUNKED_FROM: {}", tree_chunked_from.len());
    for row in &tree_chunked_from { eprintln!("  {:?}", row); }

    let file_derived_from = query_rows(&catalog,
        "MATCH (k:FileKB)-[:FileKB_DERIVED_FROM]->(f:File) RETURN k._uuid, f.name"
    );
    eprintln!("FileKB_DERIVED_FROM: {}", file_derived_from.len());
    for row in &file_derived_from { eprintln!("  {:?}", row); }

    let file_chunked_from = query_rows(&catalog,
        "MATCH (c:FileKB_Chunk)-[:FileKB_CHUNKED_FROM]->(k:FileKB) RETURN c._uuid, k.title, c._text"
    );
    eprintln!("FileKB_CHUNKED_FROM: {}", file_chunked_from.len());
    for row in &file_chunked_from { eprintln!("  {:?}", row); }

    // Try Lucivy raw query to check if FTS index has data
    eprintln!("\n══ RAW LUCIVY QUERY ══");
    let fts_result = catalog.execute_raw(
        "CALL QUERY_LUCIVY_INDEX('TreeKB', '{\"type\":\"parse\",\"fields\":[\"title\",\"content\"],\"value\":\"auth\"}', 10) RETURN node_id, score"
    );
    match fts_result {
        Ok(r) => {
            eprintln!("Lucivy 'auth' on TreeKB: {} results", r.rows.len());
            for row in &r.rows { eprintln!("  {:?}", row); }
        }
        Err(e) => eprintln!("Lucivy error: {e:?}"),
    }

    // Try search through Catalog API
    eprintln!("\n══ CATALOG SEARCH ══");
    let search_result = catalog.search(
        "TreeKB", "auth",
        SearchOptions { consistency: Consistency::Immediate, ..Default::default() },
    );
    match search_result {
        Ok(r) => eprintln!("Catalog search 'auth': {} results, bm25={}", r.results.len(), r.meta.bm25_count),
        Err(e) => eprintln!("Catalog search error: {e:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 14: Isolate Lucivy query modes — Contains vs Parse on same index
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore]
fn phase0b_lucivy_contains_vs_parse() {
    let mut catalog = make_catalog();
    catalog.initialize().unwrap();

    catalog.create("Directory", make_directory("src", "/repo/src/")).unwrap();
    let file_ref = catalog.create(
        "File",
        make_file("auth.ts", "/repo/src/auth.ts", "export function authenticate(req: Request) { return true; }"),
    ).unwrap();
    catalog.link("HAS_FILE", hashsafe_uuid("Directory", &["/repo/src/"]), file_ref.clone(), BTreeMap::new()).unwrap();

    let result = catalog.drain();
    eprintln!("drain: processed={}, failed={}", result.processed, result.failed);

    // Dump what's in the derived table
    let idx = query_rows(&catalog, "MATCH (t:TreeKB) RETURN t._uuid, t.title, t.content");
    eprintln!("\nTreeKB rows:");
    for row in &idx { eprintln!("  {:?}", row); }

    let chunks = query_rows(&catalog, "MATCH (c:TreeKB_Chunk) RETURN c._uuid, c._text, c._parent_field");
    eprintln!("TreeKB_Chunk rows:");
    for row in &chunks { eprintln!("  {:?}", row); }

    // ── Test raw Lucivy queries directly (champs `title`/`content` de la dérivée) ──
    let queries = vec![
        ("parse, fields=[title,content], 'auth'",
         r#"{"type":"parse","fields":["title","content"],"value":"auth"}"#),
        ("parse, field=content, 'auth'",
         r#"{"type":"parse","field":"content","value":"auth"}"#),
        ("parse, field=title, 'src'",
         r#"{"type":"parse","field":"title","value":"src"}"#),
        ("contains, field=content, 'auth', distance=1",
         r#"{"type":"contains","field":"content","value":"auth","distance":1}"#),
        ("contains, field=title, 'src', distance=1",
         r#"{"type":"contains","field":"title","value":"src","distance":1}"#),
        ("contains, field=content, 'auth', distance=0",
         r#"{"type":"contains","field":"content","value":"auth","distance":0}"#),
        ("boolean should [contains title + content], 'auth'",
         r#"{"type":"boolean","should":[{"type":"contains","field":"title","value":"auth","distance":1},{"type":"contains","field":"content","value":"auth","distance":1}]}"#),
        ("contains, field=content, 'authenticate', distance=1",
         r#"{"type":"contains","field":"content","value":"authenticate","distance":1}"#),
    ];

    eprintln!("\n══ RAW LUCIVY QUERY COMPARISON ══");
    for (label, json) in &queries {
        let escaped = json.replace('\'', "''");
        let cypher = format!(
            "CALL QUERY_LUCIVY_INDEX('TreeKB', '{}', 10) RETURN node_id, score, highlights",
            escaped,
        );
        match catalog.execute_raw(&cypher) {
            Ok(r) => {
                eprintln!("\n  {} → {} results", label, r.rows.len());
                for row in &r.rows { eprintln!("    {:?}", row); }
            }
            Err(e) => eprintln!("\n  {} → ERROR: {:?}", label, e),
        }
    }
}

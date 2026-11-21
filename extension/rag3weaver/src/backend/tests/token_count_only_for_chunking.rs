//! The witness of the lot `token-count-only-for-chunking` (21 November 2026): a build without `token-count` opens a
//! base under granite; only the chunks are refused.
use super::*;

/// **A build without `token-count` opens a base under granite and serves it; only the chunks are refused** (c7, 21
/// November 2026: the host posed that night, built without the feature, answered « il faut la feature token-count »
/// even to the chat). Reading needs no token; the chunking of an index does, and it is refused, naming the feature —
/// never a scope written without its chunks.
#[cfg(all(feature = "code", feature = "rag3db-native", not(feature = "token-count")))]
#[test]
fn a_build_without_token_count_opens_a_granite_base_and_refuses_only_its_chunks() {
    let (dir, p) = fixture_code_avec("working_tree", false, json!({}), &|m| {
        m["embeddings"] = json!({"model": "granite-278m", "dimensions": 768});
        m["workspace"]["index"] = json!("code");
        m["tools"]["read_file"] = json!({"graph": "read.mmd", "policy": {"read_files": true}});
    });
    let prepared = p.unwrap_or_else(|e| panic!("the manifest loads: {e}"));
    let conn = crate::Rag3dbConnection::in_memory().expect("in-memory base");
    conn.execute(&format!("LOAD EXTENSION '{}'", extension_vector_de_test())).unwrap();
    let backend = prepared.open(Box::new(conn), None).unwrap_or_else(|e| panic!("a base under granite opens without token-count: {e}"));
    let lu = backend.call("read_file", json!({"path": "main.rs"})).expect("reading needs no token");
    assert!(lu.to_string().contains("depart"), "{lu}");
    let fichiers = vec![("lib.rs".to_string(), "pub fn longue() -> usize {\n    let a = 1;\n    let b = 2;\n    a + b\n}\n".to_string())];
    let e = backend.catalog.lock().unwrap().ingest_code(&crate::code::analyze("/banc", fichiers)).err().expect("the chunks are refused");
    assert!(e.to_string().contains("token-count"), "the refusal names the feature: {e}");
    let sans_morceaux = backend
        .catalog
        .lock()
        .unwrap()
        .execute_read_with_params("MATCH (s:ScopeContent) RETURN count(s)", &[])
        .ok()
        .and_then(|r| r.rows.first().and_then(|l| l.first()).and_then(|v| v.as_i64()))
        .unwrap_or(0);
    drop(dir);
    assert_eq!(sans_morceaux, 0, "the refused ingestion writes no scope, so none without its chunks");
}

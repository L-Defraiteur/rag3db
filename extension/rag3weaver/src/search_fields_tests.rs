//! **A fuzzy `contains` over several fields pays a typo once** (lucivy 4.4.2, 22 November 2026, correction (b) of
//! lucivy's judgement): `build_contains_clauses` built one fuzzy `contains` per field in a boolean, which summed a
//! penalty per field — a document with the typo in its name and its body ranked below one with the typo in its body
//! only — and made lucivy warn. One `contains` with `fields` counts the errors of the best field and the BM25 of all.

use super::*;

#[test]
fn several_fields_are_one_fuzzy_contains() {
    let fields = vec!["content".to_string(), "name".to_string()];
    let v: serde_json::Value = serde_json::from_str(&build_bm25_query("memory", &fields, BM25Mode::Contains, 1)).unwrap();
    assert_eq!(v, serde_json::json!({ "type": "contains", "fields": ["content", "name"], "value": "memory", "distance": 1 }));
    // At distance 0 there is no penalty to share: lucivy reads `fields` only on a fuzzy `contains`.
    let v: serde_json::Value = serde_json::from_str(&build_bm25_query("memory safety", &fields, BM25Mode::Contains, 0)).unwrap();
    assert_eq!(v["type"], "boolean", "{v}");
}

/// On a real index: lucivy no longer warns, and the typo in two fields costs once.
#[test]
fn a_typo_in_two_fields_costs_once() {
    use lucivy_core::blob_store::MemBlobStore;
    use lucivy_core::query::QueryConfig;
    use lucivy_core::sharded_handle::{BlobShardStorage, ShardedHandle};
    let dir = tempfile::tempdir().unwrap();
    let fields = vec!["content".to_string(), "name".to_string()];
    let config = crate::fts_handle::build_schema_config(&fields, &[], 1).unwrap();
    let storage = BlobShardStorage::new(std::sync::Arc::new(MemBlobStore::new()), crate::fts_handle::fts_index_name("Fields"), dir.path());
    let handle = ShardedHandle::create_with_storage(Box::new(storage), &config).expect("the index");
    for (offset, content, name) in [(1_u64, "the memary model of the engine", "memary"), (2, "the memary model of the engine", "engine")] {
        crate::fts_handle::index_document(&handle, &[("content".into(), content.to_string()), ("name".into(), name.to_string())], offset).unwrap();
    }
    handle.commit().unwrap();
    let q: QueryConfig = serde_json::from_str(&build_bm25_query("memory", &fields, BM25Mode::Contains, 1)).unwrap();
    let warnings = handle.query_warnings(&q);
    assert!(!warnings.iter().any(|w| w.contains("contains_split") || w.contains("fuzzy")), "lucivy warns: {warnings:?}");
    let mut hits = crate::fts_handle::search_hits(&handle, &q, 10, None).unwrap();
    hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    assert_eq!(hits.first().map(|h| h.0), Some(1), "the typo in the name and the body is paid once: {:?}", hits.iter().map(|h| (h.0, h.1)).collect::<Vec<_>>());
}

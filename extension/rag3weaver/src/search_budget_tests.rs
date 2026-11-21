//! **`contains_split` counts one error budget per document** (lucivy 4.4.2, 22 November 2026: « partager un compte
//! d'erreur global plutôt que chacun »). `ContainsSplit` built a boolean of fuzzy `contains` by hand: each clause paid
//! its own penalty while a missing word cost nothing, so a document that found three words with a typo each ranked
//! below a document that found a single word exactly. lucivy 4.4.2 counts the budget only in its `contains_split`
//! type: the mode emits it.

use super::*;

/// The query of the mode: one field, one `contains_split` with the distance; several fields, one per field in a
/// `should` (each field its own budget, and no boolean of fuzzy `contains`, which lucivy warns about).
#[test]
fn contains_split_is_emitted_as_lucivys_type() {
    let one: serde_json::Value = serde_json::from_str(&build_bm25_query("memory safety borrowing", &["content".to_string()], BM25Mode::ContainsSplit, 1)).unwrap();
    assert_eq!(one, serde_json::json!({ "type": "contains_split", "field": "content", "value": "memory safety borrowing", "distance": 1 }));
    let two: serde_json::Value =
        serde_json::from_str(&build_bm25_query("memory safety", &["content".to_string(), "name".to_string()], BM25Mode::ContainsSplit, 1)).unwrap();
    assert_eq!(two, serde_json::json!({ "type": "boolean", "should": [
        { "type": "contains_split", "field": "content", "value": "memory safety", "distance": 1 },
        { "type": "contains_split", "field": "name", "value": "memory safety", "distance": 1 },
    ] }));
}

/// On a real index: three words with a typo each come before a single exact word.
#[test]
fn three_words_with_a_typo_each_come_before_a_single_exact_word() {
    use lucivy_core::blob_store::MemBlobStore;
    use lucivy_core::query::QueryConfig;
    use lucivy_core::sharded_handle::{BlobShardStorage, ShardedHandle};
    let dir = tempfile::tempdir().unwrap();
    let fields = vec!["content".to_string()];
    let config = crate::fts_handle::build_schema_config(&fields, &[], 1).unwrap();
    let storage = BlobShardStorage::new(std::sync::Arc::new(MemBlobStore::new()), crate::fts_handle::fts_index_name("Budget"), dir.path());
    let handle = ShardedHandle::create_with_storage(Box::new(storage), &config).expect("the index");
    for (offset, text) in [(1_u64, "memary safty borowing checker"), (2, "memory and nothing else at all"), (3, "unrelated words only here")] {
        crate::fts_handle::index_document(&handle, &[("content".into(), text.to_string())], offset).unwrap();
    }
    handle.commit().unwrap();
    let q: QueryConfig = serde_json::from_str(&build_bm25_query("memory safety borrowing", &fields, BM25Mode::ContainsSplit, 1)).unwrap();
    let mut hits = crate::fts_handle::search_hits(&handle, &q, 10, None).unwrap();
    hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    assert_eq!(hits.first().map(|h| h.0), Some(1), "three words with a typo each come first: {:?}", hits.iter().map(|h| (h.0, h.1)).collect::<Vec<_>>());
}

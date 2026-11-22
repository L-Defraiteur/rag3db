//! **The whole query is a copied text, found as it is** (lucivy, judge of its queries' use, 22 November 2026). A fuzzy
//! `contains` ignores the separators: a phrase searched at distance 1 becomes one glued word searched within a typo.
//! At distance 0, lucivy's relaxed mode still ignores them (an absent one too): only the typo goes.
//! Lucie's rule of 25 October wants a text copied in front: distance 0 for a query of several words, 1 only for a
//! single token.

use super::*;

#[test]
fn several_words_are_searched_exactly_a_single_token_within_a_typo() {
    assert_eq!(whole_query_distance("memory safety"), 0);
    assert_eq!(whole_query_distance("refresh index states"), 0);
    assert_eq!(whole_query_distance("kmalloc"), 1);
    assert_eq!(whole_query_distance("cpu_count"), 1);
    assert_eq!(whole_query_distance(&"x".repeat(MAX_FUZZY_WHOLE_QUERY_CHARS + 1)), 0, "a long token keeps its bound");
}

/// On a real lucivy index: the copied phrase is found through any separator — even none, as lucivy's relaxed mode
/// ignores them on both sides (lucivy, 22 November 2026: « spin lock » finds `spinlock`) — and a phrase with a typo
/// is not: the whole query no longer tolerates a typo over several words.
#[test]
fn a_copied_phrase_is_found_through_any_separator_but_not_with_a_typo() {
    use lucivy_core::blob_store::MemBlobStore;
    use lucivy_core::query::QueryConfig;
    use lucivy_core::sharded_handle::{BlobShardStorage, ShardedHandle};
    let dir = tempfile::tempdir().unwrap();
    let fields = vec!["content".to_string()];
    let config = crate::fts_handle::build_schema_config(&fields, &[], 1).unwrap();
    let storage = BlobShardStorage::new(std::sync::Arc::new(MemBlobStore::new()), crate::fts_handle::fts_index_name("Whole"), dir.path());
    let handle = ShardedHandle::create_with_storage(Box::new(storage), &config).expect("the index");
    let documents = [(1_u64, "the memorysafety rules apply here"), (2, "read the memory safety rules first"), (3, "read the memory safetx rules first")];
    for (offset, text) in documents {
        crate::fts_handle::index_document(&handle, &[("content".into(), text.to_string())], offset).unwrap();
    }
    handle.commit().unwrap();
    let query = "memory safety rules";
    let q: QueryConfig = serde_json::from_str(&build_bm25_query(query, &fields, BM25Mode::Contains, whole_query_distance(query))).unwrap();
    let found: Vec<u64> = crate::fts_handle::search_hits(&handle, &q, 10, None).unwrap().into_iter().map(|h| h.0).collect();
    assert!(found.contains(&2), "the copied phrase is found: {found:?}");
    assert!(found.contains(&1), "lucivy's relaxed mode ignores the separators, an absent one too: {found:?}");
    assert!(!found.contains(&3), "a phrase with a typo is not the copied text: {found:?}");
}

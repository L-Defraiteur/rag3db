//! Score fusion strategies for hybrid search.
//!
//! Combines vector similarity scores and BM25 keyword scores into a single
//! ranking. Three strategies are supported:
//!
//! - **Boost**: vector score amplified by keyword relevance
//! - **Weighted**: linear combination of vector and keyword scores
//! - **RRF**: Reciprocal Rank Fusion — rank-based, score-agnostic

use std::collections::HashMap;

/// Boost fusion: amplify vector score by normalized BM25 relevance.
///
/// `score = vector_score × (1 + bm25_normalized × boost_factor)`
///
/// Best when vector similarity is the primary signal and keywords provide
/// a bonus for exact matches.
pub fn boost_fuse(vector_score: f32, bm25_normalized: f32, boost_factor: f32) -> f32 {
    vector_score * (1.0 + bm25_normalized * boost_factor)
}

/// Weighted fusion: linear combination of vector and BM25 scores.
///
/// `score = (1 - keyword_weight) × vector_score + keyword_weight × bm25_score`
///
/// `keyword_weight` typically 0.2–0.4. At 0.0 = pure semantic, at 1.0 = pure keyword.
pub fn weighted_fuse(vector_score: f32, bm25_score: f32, keyword_weight: f32) -> f32 {
    (1.0 - keyword_weight) * vector_score + keyword_weight * bm25_score
}

/// Reciprocal Rank Fusion: combine multiple ranked lists by rank position.
///
/// `score(doc) = Σ 1 / (k + rank_i + 1)` for each list where doc appears.
///
/// Score-agnostic — only uses ranking order. `k` controls how much top ranks
/// dominate (typical value: 60).
///
/// Each input list is `Vec<(id, score)>` sorted by descending score.
/// Returns a merged list sorted by descending RRF score.
pub fn rrf_fuse(ranked_lists: &[&[(String, f32)]], k: f32) -> Vec<(String, f32)> {
    let mut scores: HashMap<String, f32> = HashMap::new();

    for list in ranked_lists {
        for (rank, (id, _score)) in list.iter().enumerate() {
            *scores.entry(id.clone()).or_default() += 1.0 / (k + rank as f32 + 1.0);
        }
    }

    let mut results: Vec<(String, f32)> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── boost ────────────────────────────────────────────────────────────

    #[test]
    fn boost_no_keyword() {
        // bm25 = 0 → no boost, pure vector score
        assert!((boost_fuse(0.8, 0.0, 2.0) - 0.8).abs() < 1e-6);
    }

    #[test]
    fn boost_with_keyword() {
        // bm25 = 1.0, boost = 2.0 → score = 0.8 * (1 + 1.0 * 2.0) = 0.8 * 3.0 = 2.4
        assert!((boost_fuse(0.8, 1.0, 2.0) - 2.4).abs() < 1e-6);
    }

    #[test]
    fn boost_zero_vector() {
        assert!((boost_fuse(0.0, 1.0, 5.0) - 0.0).abs() < 1e-6);
    }

    // ── weighted ─────────────────────────────────────────────────────────

    #[test]
    fn weighted_pure_semantic() {
        // keyword_weight = 0 → pure vector
        assert!((weighted_fuse(0.9, 0.5, 0.0) - 0.9).abs() < 1e-6);
    }

    #[test]
    fn weighted_pure_keyword() {
        // keyword_weight = 1 → pure BM25
        assert!((weighted_fuse(0.9, 0.5, 1.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn weighted_balanced() {
        // 0.3 keyword → 0.7 * 0.8 + 0.3 * 0.6 = 0.56 + 0.18 = 0.74
        assert!((weighted_fuse(0.8, 0.6, 0.3) - 0.74).abs() < 1e-6);
    }

    // ── RRF ──────────────────────────────────────────────────────────────

    #[test]
    fn rrf_single_list() {
        let list = vec![
            ("a".to_string(), 1.0),
            ("b".to_string(), 0.5),
        ];
        let results = rrf_fuse(&[&list], 60.0);
        assert_eq!(results[0].0, "a");
        assert_eq!(results[1].0, "b");
        // a: 1/(60+0+1) = 1/61
        assert!((results[0].1 - 1.0 / 61.0).abs() < 1e-6);
    }

    #[test]
    fn rrf_two_lists_shared_doc() {
        let vector = vec![
            ("doc1".to_string(), 0.9),
            ("doc2".to_string(), 0.7),
        ];
        let keyword = vec![
            ("doc2".to_string(), 5.0),
            ("doc1".to_string(), 3.0),
        ];
        let results = rrf_fuse(&[&vector, &keyword], 60.0);

        // doc1: 1/(60+0+1) + 1/(60+1+1) = 1/61 + 1/62
        // doc2: 1/(60+1+1) + 1/(60+0+1) = 1/62 + 1/61
        // Both should have the same score (symmetric)
        assert!((results[0].1 - results[1].1).abs() < 1e-6);
    }

    #[test]
    fn rrf_disjoint_lists() {
        let list1 = vec![("a".to_string(), 1.0)];
        let list2 = vec![("b".to_string(), 1.0)];
        let results = rrf_fuse(&[&list1, &list2], 60.0);
        assert_eq!(results.len(), 2);
        // Both have same RRF score: 1/61
        assert!((results[0].1 - results[1].1).abs() < 1e-6);
    }

    #[test]
    fn rrf_empty_lists() {
        let results = rrf_fuse(&[], 60.0);
        assert!(results.is_empty());
    }

    #[test]
    fn rrf_k_parameter_effect() {
        let list = vec![
            ("a".to_string(), 1.0),
            ("b".to_string(), 0.5),
        ];
        let results_low_k = rrf_fuse(&[&list], 1.0);
        let results_high_k = rrf_fuse(&[&list], 100.0);

        // With low k, rank difference matters more
        let gap_low = results_low_k[0].1 - results_low_k[1].1;
        let gap_high = results_high_k[0].1 - results_high_k[1].1;
        assert!(
            gap_low > gap_high,
            "low k should produce larger score gaps between ranks"
        );
    }
}

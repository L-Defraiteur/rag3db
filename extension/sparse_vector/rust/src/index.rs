//! In-memory sparse vector inverted index.
//!
//! Provides storage and dot-product search for sparse vectors (BM42-style).
//! HashMap-based posting lists — no compression, optimized for simplicity.
//! Uses u64 node_ids (rag3db internal offsets) instead of String UUIDs.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A sparse vector: parallel arrays of token IDs and weights.
///
/// Invariant: `indices.len() == values.len()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SparseVector {
    pub indices: Vec<u32>,
    pub values: Vec<f32>,
}

impl SparseVector {
    pub fn new(indices: Vec<u32>, values: Vec<f32>) -> Self {
        assert_eq!(
            indices.len(),
            values.len(),
            "indices and values must have same length"
        );
        Self { indices, values }
    }

    pub fn nnz(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// In-memory inverted index for sparse vectors.
///
/// Maps `token_id → [(node_id, weight)]`. Supports:
/// - `insert(node_id, sparse_vector)` — add/replace a document
/// - `remove(node_id)` — remove a document
/// - `search(query_vector, limit)` — top-k by dot product score
#[derive(Debug, Serialize, Deserialize)]
pub struct SparseIndex {
    postings: HashMap<u32, Vec<(u64, f32)>>,
    vectors: HashMap<u64, SparseVector>,
}

impl SparseIndex {
    pub fn new() -> Self {
        Self {
            postings: HashMap::new(),
            vectors: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Insert a document's sparse vector. Replaces if node_id already exists.
    pub fn insert(&mut self, node_id: u64, vector: &SparseVector) {
        if self.vectors.contains_key(&node_id) {
            self.remove(node_id);
        }

        for (i, &token_id) in vector.indices.iter().enumerate() {
            self.postings
                .entry(token_id)
                .or_default()
                .push((node_id, vector.values[i]));
        }

        self.vectors.insert(node_id, vector.clone());
    }

    /// Remove a document from the index. Returns true if it existed.
    pub fn remove(&mut self, node_id: u64) -> bool {
        if let Some(vector) = self.vectors.remove(&node_id) {
            for &token_id in &vector.indices {
                if let Some(postings) = self.postings.get_mut(&token_id) {
                    postings.retain(|(nid, _)| *nid != node_id);
                    if postings.is_empty() {
                        self.postings.remove(&token_id);
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Search: dot product of query against all indexed documents, return top-k.
    pub fn search(&self, query: &SparseVector, limit: usize) -> Vec<(u64, f32)> {
        let mut scores: HashMap<u64, f32> = HashMap::new();

        for (i, &token_id) in query.indices.iter().enumerate() {
            let q_weight = query.values[i];
            if let Some(postings) = self.postings.get(&token_id) {
                for &(doc_id, d_weight) in postings {
                    *scores.entry(doc_id).or_default() += q_weight * d_weight;
                }
            }
        }

        let mut results: Vec<(u64, f32)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Search with allowed_ids filter (intersection).
    pub fn search_filtered(
        &self,
        query: &SparseVector,
        limit: usize,
        allowed_ids: &[u64],
    ) -> Vec<(u64, f32)> {
        let allowed: std::collections::HashSet<u64> = allowed_ids.iter().copied().collect();
        let mut scores: HashMap<u64, f32> = HashMap::new();

        for (i, &token_id) in query.indices.iter().enumerate() {
            let q_weight = query.values[i];
            if let Some(postings) = self.postings.get(&token_id) {
                for &(doc_id, d_weight) in postings {
                    if allowed.contains(&doc_id) {
                        *scores.entry(doc_id).or_default() += q_weight * d_weight;
                    }
                }
            }
        }

        let mut results: Vec<(u64, f32)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Clear the entire index.
    pub fn clear(&mut self) {
        self.postings.clear();
        self.vectors.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_vector_basics() {
        let v = SparseVector::new(vec![1, 3, 5], vec![0.5, 0.3, 0.2]);
        assert_eq!(v.nnz(), 3);
        assert!(!v.is_empty());

        let empty = SparseVector::new(vec![], vec![]);
        assert!(empty.is_empty());
    }

    #[test]
    #[should_panic(expected = "indices and values must have same length")]
    fn sparse_vector_mismatched_lengths() {
        SparseVector::new(vec![1, 2], vec![0.5]);
    }

    #[test]
    fn index_insert_and_search() {
        let mut index = SparseIndex::new();
        index.insert(1, &SparseVector::new(vec![1, 2, 3], vec![0.5, 0.3, 0.2]));
        index.insert(2, &SparseVector::new(vec![2, 3, 4], vec![0.4, 0.6, 0.1]));
        index.insert(3, &SparseVector::new(vec![1, 4, 5], vec![0.9, 0.1, 0.1]));
        assert_eq!(index.len(), 3);

        let query = SparseVector::new(vec![1, 2], vec![1.0, 1.0]);
        let results = index.search(&query, 10);

        // doc1: 0.5 + 0.3 = 0.8
        // doc2: 0.4 = 0.4
        // doc3: 0.9 = 0.9
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, 3);
        assert!((results[0].1 - 0.9).abs() < 1e-6);
        assert_eq!(results[1].0, 1);
        assert!((results[1].1 - 0.8).abs() < 1e-6);
        assert_eq!(results[2].0, 2);
        assert!((results[2].1 - 0.4).abs() < 1e-6);
    }

    #[test]
    fn index_remove() {
        let mut index = SparseIndex::new();
        index.insert(1, &SparseVector::new(vec![1, 2], vec![0.5, 0.3]));
        index.insert(2, &SparseVector::new(vec![1, 3], vec![0.9, 0.1]));
        assert_eq!(index.len(), 2);

        assert!(index.remove(1));
        assert_eq!(index.len(), 1);
        assert!(!index.remove(1)); // already removed

        let query = SparseVector::new(vec![1], vec![1.0]);
        let results = index.search(&query, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 2);
    }

    #[test]
    fn index_insert_replaces() {
        let mut index = SparseIndex::new();
        index.insert(1, &SparseVector::new(vec![1], vec![0.5]));
        index.insert(1, &SparseVector::new(vec![2], vec![0.9]));
        assert_eq!(index.len(), 1);

        let query = SparseVector::new(vec![1, 2], vec![1.0, 1.0]);
        let results = index.search(&query, 10);
        assert_eq!(results.len(), 1);
        assert!((results[0].1 - 0.9).abs() < 1e-6);
    }

    #[test]
    fn index_search_limit() {
        let mut index = SparseIndex::new();
        for i in 0..100u64 {
            index.insert(i, &SparseVector::new(vec![1], vec![i as f32]));
        }
        let query = SparseVector::new(vec![1], vec![1.0]);
        let results = index.search(&query, 5);
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].0, 99);
    }

    #[test]
    fn index_search_disjoint() {
        let mut index = SparseIndex::new();
        index.insert(1, &SparseVector::new(vec![1, 2], vec![1.0, 1.0]));
        let query = SparseVector::new(vec![3, 4], vec![1.0, 1.0]);
        let results = index.search(&query, 10);
        assert!(results.is_empty());
    }

    #[test]
    fn index_empty_search() {
        let index = SparseIndex::new();
        let query = SparseVector::new(vec![1], vec![1.0]);
        assert!(index.search(&query, 10).is_empty());
    }

    #[test]
    fn index_clear() {
        let mut index = SparseIndex::new();
        index.insert(1, &SparseVector::new(vec![1], vec![0.5]));
        index.insert(2, &SparseVector::new(vec![2], vec![0.3]));
        assert_eq!(index.len(), 2);

        index.clear();
        assert!(index.is_empty());
        assert!(index.search(&SparseVector::new(vec![1], vec![1.0]), 10).is_empty());
    }

    #[test]
    fn index_remove_cleans_empty_postings() {
        let mut index = SparseIndex::new();
        index.insert(1, &SparseVector::new(vec![42], vec![1.0]));
        index.remove(1);
        assert!(index.postings.is_empty());
    }

    #[test]
    fn search_filtered_basic() {
        let mut index = SparseIndex::new();
        index.insert(1, &SparseVector::new(vec![1, 2], vec![0.5, 0.3]));
        index.insert(2, &SparseVector::new(vec![1, 3], vec![0.9, 0.1]));
        index.insert(3, &SparseVector::new(vec![1], vec![0.7]));

        let query = SparseVector::new(vec![1], vec![1.0]);

        // Only allow node 1 and 3
        let results = index.search_filtered(&query, 10, &[1, 3]);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 3); // 0.7
        assert_eq!(results[1].0, 1); // 0.5

        // Allow only node 2
        let results = index.search_filtered(&query, 10, &[2]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 2);
    }
}

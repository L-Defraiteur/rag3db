//! In-memory sparse vector inverted index.
//!
//! Provides storage and dot-product search for sparse vectors (BM42-style).
//! HashMap-based posting lists — no compression, optimized for simplicity.

use std::collections::HashMap;

/// A sparse vector: parallel arrays of token IDs and weights.
///
/// Invariant: `indices.len() == values.len()`.
/// Indices should be sorted for efficient dot product with another sparse vector.
#[derive(Debug, Clone, PartialEq)]
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
/// Maps `token_id → [(uuid, weight)]`. Supports:
/// - `insert(uuid, sparse_vector)` — add/replace a document
/// - `remove(uuid)` — remove a document
/// - `search(query_vector, limit)` — top-k by dot product score
pub struct SparseIndex {
    postings: HashMap<u32, Vec<(String, f32)>>,
    vectors: HashMap<String, SparseVector>,
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

    /// Insert a document's sparse vector. Replaces if uuid already exists.
    pub fn insert(&mut self, uuid: &str, vector: &SparseVector) {
        if self.vectors.contains_key(uuid) {
            self.remove(uuid);
        }

        for (i, &token_id) in vector.indices.iter().enumerate() {
            self.postings
                .entry(token_id)
                .or_default()
                .push((uuid.to_string(), vector.values[i]));
        }

        self.vectors.insert(uuid.to_string(), vector.clone());
    }

    /// Remove a document from the index. Returns true if it existed.
    pub fn remove(&mut self, uuid: &str) -> bool {
        if let Some(vector) = self.vectors.remove(uuid) {
            for &token_id in &vector.indices {
                if let Some(postings) = self.postings.get_mut(&token_id) {
                    postings.retain(|(u, _)| u != uuid);
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
    pub fn search(&self, query: &SparseVector, limit: usize) -> Vec<(String, f32)> {
        let mut scores: HashMap<&str, f32> = HashMap::new();

        for (i, &token_id) in query.indices.iter().enumerate() {
            let q_weight = query.values[i];
            if let Some(postings) = self.postings.get(&token_id) {
                for (doc_uuid, d_weight) in postings {
                    *scores.entry(doc_uuid).or_default() += q_weight * d_weight;
                }
            }
        }

        let mut results: Vec<(String, f32)> = scores
            .into_iter()
            .map(|(uuid, score)| (uuid.to_string(), score))
            .collect();
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
        index.insert("doc1", &SparseVector::new(vec![1, 2, 3], vec![0.5, 0.3, 0.2]));
        index.insert("doc2", &SparseVector::new(vec![2, 3, 4], vec![0.4, 0.6, 0.1]));
        index.insert("doc3", &SparseVector::new(vec![1, 4, 5], vec![0.9, 0.1, 0.1]));
        assert_eq!(index.len(), 3);

        let query = SparseVector::new(vec![1, 2], vec![1.0, 1.0]);
        let results = index.search(&query, 10);

        // doc1: 0.5 + 0.3 = 0.8
        // doc2: 0.4 = 0.4
        // doc3: 0.9 = 0.9
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, "doc3");
        assert!((results[0].1 - 0.9).abs() < 1e-6);
        assert_eq!(results[1].0, "doc1");
        assert!((results[1].1 - 0.8).abs() < 1e-6);
        assert_eq!(results[2].0, "doc2");
        assert!((results[2].1 - 0.4).abs() < 1e-6);
    }

    #[test]
    fn index_remove() {
        let mut index = SparseIndex::new();
        index.insert("doc1", &SparseVector::new(vec![1, 2], vec![0.5, 0.3]));
        index.insert("doc2", &SparseVector::new(vec![1, 3], vec![0.9, 0.1]));
        assert_eq!(index.len(), 2);

        assert!(index.remove("doc1"));
        assert_eq!(index.len(), 1);
        assert!(!index.remove("doc1")); // already removed

        let query = SparseVector::new(vec![1], vec![1.0]);
        let results = index.search(&query, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "doc2");
    }

    #[test]
    fn index_insert_replaces() {
        let mut index = SparseIndex::new();
        index.insert("doc1", &SparseVector::new(vec![1], vec![0.5]));
        index.insert("doc1", &SparseVector::new(vec![2], vec![0.9]));
        assert_eq!(index.len(), 1);

        // token 1 should be gone, only token 2 remains
        let query = SparseVector::new(vec![1, 2], vec![1.0, 1.0]);
        let results = index.search(&query, 10);
        assert_eq!(results.len(), 1);
        assert!((results[0].1 - 0.9).abs() < 1e-6);
    }

    #[test]
    fn index_search_limit() {
        let mut index = SparseIndex::new();
        for i in 0..100u32 {
            let key = format!("doc{i}");
            index.insert(&key, &SparseVector::new(vec![1], vec![i as f32]));
        }
        let query = SparseVector::new(vec![1], vec![1.0]);
        let results = index.search(&query, 5);
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].0, "doc99");
    }

    #[test]
    fn index_search_disjoint() {
        let mut index = SparseIndex::new();
        index.insert("doc1", &SparseVector::new(vec![1, 2], vec![1.0, 1.0]));
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
        index.insert("doc1", &SparseVector::new(vec![1], vec![0.5]));
        index.insert("doc2", &SparseVector::new(vec![2], vec![0.3]));
        assert_eq!(index.len(), 2);

        index.clear();
        assert!(index.is_empty());
        assert!(index.search(&SparseVector::new(vec![1], vec![1.0]), 10).is_empty());
    }

    #[test]
    fn index_remove_cleans_empty_postings() {
        let mut index = SparseIndex::new();
        index.insert("doc1", &SparseVector::new(vec![42], vec![1.0]));
        index.remove("doc1");
        // posting list for token 42 should be fully removed
        assert!(index.postings.is_empty());
    }
}

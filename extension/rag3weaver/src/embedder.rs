//! Embedding trait and error types.
//!
//! The [`Embedder`] trait abstracts over embedding providers. Rag3weaver is
//! deliberately provider-agnostic: the library has **zero ML dependencies**.
//! Users provide their own [`Embedder`] implementation — candle, API call,
//! Transformers.js via wasm-bindgen, or anything else.
//!
//! [`CallbackEmbedder`] wraps a closure for quick one-off usage without
//! defining a struct. [`MockEmbedder`] is provided for testing.
//!
//! The [`SparseEmbedder`] trait is separate for sparse vector generation
//! (BM42-style attention weights). [`MockSparseEmbedder`] and
//! [`CallbackSparseEmbedder`] follow the same patterns.

use std::collections::HashMap;

use thiserror::Error;

use crate::sparse_index::SparseVector;

/// Errors that can occur during embedding.
#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("provider error: {0}")]
    ProviderError(String),

    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("batch too large: max {max}, got {got}")]
    BatchTooLarge { max: usize, got: usize },

    #[error("embedding request timed out")]
    Timeout,
}

/// Trait for embedding text into vectors.
///
/// Implementations must be `Send + Sync` for use across async tasks.

pub trait Embedder: Send + Sync {
    /// Embed a batch of texts into vectors.
    ///
    /// Returns one vector per input text, each of dimension [`Self::dim()`].
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;

    /// The output dimension of the embedding model.
    fn dim(&self) -> usize;
}

/// Mock embedder for testing. Returns zero vectors of the configured dimension.
#[derive(Debug, Clone)]
pub struct MockEmbedder {
    dim: usize,
}

impl MockEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}


impl Embedder for MockEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(texts.iter().map(|_| vec![0.0_f32; self.dim]).collect())
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

// ─── HashEmbedder ────────────────────────────────────────────────────────────

/// Embedder de test **non dégénéré** : un vecteur unitaire pseudo-aléatoire,
/// déterministe, dérivé du hash du texte. Deux textes identiques → même
/// vecteur ; deux textes différents → vecteurs sans rapport.
///
/// À utiliser dès qu'un test ingère plus qu'une poignée de lignes :
/// [`MockEmbedder`] rend des vecteurs **nuls**, et l'index HNSW de l'extension
/// vectorielle **segfaute** (`shrinkForNode` → `computeDistance`) quand on lui
/// insère quelques centaines de points identiques (25 août 2026, 1 402 scopes
/// de code).
#[derive(Debug, Clone)]
pub struct HashEmbedder {
    dim: usize,
}

impl HashEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }

    fn vector(&self, text: &str) -> Vec<f32> {
        // xorshift ensemencé par le hash du texte : autant de flottants qu'il
        // faut, dans [-1, 1], puis normalisation.
        let hex = crate::hash::content_hash(text);
        let mut state = u64::from_str_radix(&hex[..16], 16).unwrap_or(0x9E37_79B9_7F4A_7C15) | 1;
        let mut v: Vec<f32> = (0..self.dim)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 11) as f32 / (1u64 << 53) as f32 * 2.0 - 1.0
            })
            .collect();
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-12);
        for x in &mut v {
            *x /= norm;
        }
        v
    }
}

impl Embedder for HashEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(texts.iter().map(|t| self.vector(t)).collect())
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

// ─── CallbackEmbedder ────────────────────────────────────────────────────────

/// Type alias for the embed callback.
///
/// `Fn(&[String])` → `Result<Vec<Vec<f32>>, EmbedError>`.
pub type EmbedFn = Box<
    dyn Fn(&[String]) -> Result<Vec<Vec<f32>>, EmbedError>
        + Send
        + Sync,
>;

/// Embedder backed by a user-provided closure.
///
/// Useful when you don't want to define a struct + impl — just pass a function.
///
/// ```ignore
/// use rag3weaver::{CallbackEmbedder, EmbedError};
///
/// let embedder = CallbackEmbedder::new(384, |texts| {
///     Box::pin(async move {
///         // your embedding logic here (candle, API, Transformers.js, …)
///         Ok(texts.iter().map(|_| vec![0.0f32; 384]).collect())
///     })
/// });
/// ```
pub struct CallbackEmbedder {
    embed_fn: EmbedFn,
    dim: usize,
}

impl CallbackEmbedder {
    pub fn new<F>(dim: usize, f: F) -> Self
    where
        F: Fn(&[String]) -> Result<Vec<Vec<f32>>, EmbedError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            embed_fn: Box::new(f),
            dim,
        }
    }
}


impl Embedder for CallbackEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        (self.embed_fn)(texts)
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

// ─── SparseEmbedder ─────────────────────────────────────────────────────────

/// Trait for embedding text into sparse vectors.
///
/// Separate from [`Embedder`] to allow independent implementations.
/// BGE-M3 (`bge_m3_embedder`, `burn_bge_m3_embedder`) is the learned-sparse implementation.

pub trait SparseEmbedder: Send + Sync {
    /// Embed a batch of texts into sparse vectors.
    fn embed_sparse(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError>;
}

/// Mock sparse embedder for testing.
///
/// Generates deterministic sparse vectors: each whitespace-separated word is
/// hashed (djb2) to a token ID in `[0, 30000)`, with weight `1/num_words`.
#[derive(Debug, Clone)]
pub struct MockSparseEmbedder;

impl MockSparseEmbedder {
    pub fn new() -> Self {
        Self
    }

    fn word_to_token(word: &str) -> u32 {
        let mut hash: u32 = 5381;
        for byte in word.as_bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(*byte as u32);
        }
        hash % 30000
    }
}


impl SparseEmbedder for MockSparseEmbedder {
    fn embed_sparse(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError> {
        Ok(texts
            .iter()
            .map(|text| {
                let words: Vec<&str> = text.split_whitespace().collect();
                if words.is_empty() {
                    return SparseVector::new(vec![], vec![]);
                }
                let mut token_weights: HashMap<u32, f32> = HashMap::new();
                for word in &words {
                    let token = Self::word_to_token(word);
                    *token_weights.entry(token).or_default() += 1.0 / words.len() as f32;
                }
                let mut pairs: Vec<(u32, f32)> = token_weights.into_iter().collect();
                pairs.sort_by_key(|(idx, _)| *idx);
                let (indices, values): (Vec<u32>, Vec<f32>) = pairs.into_iter().unzip();
                SparseVector::new(indices, values)
            })
            .collect())
    }
}

// ─── CallbackSparseEmbedder ─────────────────────────────────────────────────

/// Type alias for the sparse embed callback.
pub type SparseEmbedFn = Box<
    dyn Fn(&[String]) -> Result<Vec<SparseVector>, EmbedError>
        + Send
        + Sync,
>;

/// Sparse embedder backed by a user-provided closure.
pub struct CallbackSparseEmbedder {
    embed_fn: SparseEmbedFn,
}

impl CallbackSparseEmbedder {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&[String]) -> Result<Vec<SparseVector>, EmbedError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            embed_fn: Box::new(f),
        }
    }
}


impl SparseEmbedder for CallbackSparseEmbedder {
    fn embed_sparse(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError> {
        (self.embed_fn)(texts)
    }
}

// ─── DualEmbedder ──────────────────────────────────────────────────────────

/// Trait for models that produce both dense and sparse embeddings in a single
/// forward pass (e.g. BGE-M3).
///
/// Implementing this trait allows the pipeline to avoid redundant forward passes
/// when both dense and sparse signals are active.

pub trait DualEmbedder: Send + Sync {
    /// Embed a batch of texts into dense + sparse vectors in one forward pass.
    fn embed_dual(
        &self,
        texts: &[String],
    ) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError>;

    /// The output dimension of the dense embedding.
    fn dim(&self) -> usize;
}

/// Mock dual embedder for testing. Returns zero dense vectors + word-hash sparse vectors.
#[derive(Debug, Clone)]
pub struct MockDualEmbedder {
    dim: usize,
}

impl MockDualEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}


impl DualEmbedder for MockDualEmbedder {
    fn embed_dual(
        &self,
        texts: &[String],
    ) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError> {
        let dense: Vec<Vec<f32>> = texts.iter().map(|_| vec![0.0_f32; self.dim]).collect();
        let sparse_embedder = MockSparseEmbedder::new();
        let sparse = sparse_embedder.embed_sparse(texts)?;
        Ok((dense, sparse))
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

/// Type alias for the dual embed callback.
pub type DualEmbedFn = Box<
    dyn Fn(&[String]) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError>
        + Send
        + Sync,
>;

/// Dual embedder backed by a user-provided closure (for WASM FFI or custom pipelines).
pub struct CallbackDualEmbedder {
    embed_fn: DualEmbedFn,
    dim: usize,
}

impl CallbackDualEmbedder {
    pub fn new<F>(dim: usize, f: F) -> Self
    where
        F: Fn(&[String]) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            embed_fn: Box::new(f),
            dim,
        }
    }
}


impl DualEmbedder for CallbackDualEmbedder {
    fn embed_dual(
        &self,
        texts: &[String],
    ) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError> {
        (self.embed_fn)(texts)
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_embedder_dimensions() {
        let embedder = MockEmbedder::new(384);
        assert_eq!(embedder.dim(), 384);

        let texts = vec!["hello world".into(), "foo bar".into()];
        let results = embedder.embed(&texts).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].len(), 384);
        assert_eq!(results[1].len(), 384);
    }

    #[test]
    fn mock_embedder_zero_vectors() {
        let embedder = MockEmbedder::new(3);
        let texts = vec!["test".into()];
        let results = embedder.embed(&texts).unwrap();
        assert_eq!(results[0], vec![0.0_f32; 3]);
    }

    #[test]
    fn mock_embedder_empty_batch() {
        let embedder = MockEmbedder::new(128);
        let results = embedder.embed(&[]).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn embed_error_display() {
        assert_eq!(
            EmbedError::ProviderError("connection refused".into()).to_string(),
            "provider error: connection refused"
        );
        assert_eq!(
            EmbedError::DimensionMismatch {
                expected: 384,
                got: 768
            }
            .to_string(),
            "dimension mismatch: expected 384, got 768"
        );
        assert_eq!(
            EmbedError::BatchTooLarge { max: 32, got: 100 }.to_string(),
            "batch too large: max 32, got 100"
        );
        assert_eq!(EmbedError::Timeout.to_string(), "embedding request timed out");
    }

    #[test]
    fn embedder_as_trait_object() {
        let embedder: Box<dyn Embedder> = Box::new(MockEmbedder::new(64));
        assert_eq!(embedder.dim(), 64);
        let result = embedder.embed(&["test".into()]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 64);
    }

    // ── CallbackEmbedder ────────────────────────────────────────────────

    #[test]
    fn callback_embedder_basic() {
        let embedder = CallbackEmbedder::new(3, |texts| {
            Ok(vec![vec![1.0_f32, 2.0, 3.0]; texts.len()])
        });

        assert_eq!(embedder.dim(), 3);
        let result = embedder.embed(&["hello".into(), "world".into()]).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn callback_embedder_error() {
        let embedder = CallbackEmbedder::new(384, |_texts| {
            Err(EmbedError::ProviderError("connection refused".into()))
        });

        let err = embedder.embed(&["test".into()]).unwrap_err();
        assert!(matches!(err, EmbedError::ProviderError(_)));
    }

    #[test]
    fn callback_embedder_as_trait_object() {
        let embedder: Box<dyn Embedder> = Box::new(CallbackEmbedder::new(5, |texts| {
            Ok(vec![vec![0.5_f32; 5]; texts.len()])
        }));

        assert_eq!(embedder.dim(), 5);
        let result = embedder.embed(&["a".into()]).unwrap();
        assert_eq!(result[0].len(), 5);
    }

    #[test]
    fn callback_embedder_empty_batch() {
        let embedder = CallbackEmbedder::new(3, |texts| {
            Ok(vec![vec![0.0_f32; 3]; texts.len()])
        });

        let result = embedder.embed(&[]).unwrap();
        assert!(result.is_empty());
    }

    // ── SparseEmbedder ────────────────────────────────────────────────

    #[test]
    fn mock_sparse_embedder_basic() {
        let embedder = MockSparseEmbedder::new();
        let results = embedder
            .embed_sparse(&["hello world".into()])
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].is_empty());
        // 2 words → at most 2 tokens (could be 1 if hash collision)
        assert!(results[0].nnz() <= 2);
        // weights should sum to ~1.0
        let sum: f32 = results[0].values.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn mock_sparse_embedder_deterministic() {
        let embedder = MockSparseEmbedder::new();
        let r1 = embedder.embed_sparse(&["test".into()]).unwrap();
        let r2 = embedder.embed_sparse(&["test".into()]).unwrap();
        assert_eq!(r1[0], r2[0]);
    }

    #[test]
    fn mock_sparse_embedder_empty() {
        let embedder = MockSparseEmbedder::new();
        let results = embedder.embed_sparse(&["".into()]).unwrap();
        assert!(results[0].is_empty());
    }

    #[test]
    fn mock_sparse_embedder_indices_sorted() {
        let embedder = MockSparseEmbedder::new();
        let results = embedder
            .embed_sparse(&["the quick brown fox jumps".into()])
            .unwrap();
        let indices = &results[0].indices;
        for w in indices.windows(2) {
            assert!(w[0] < w[1], "indices should be sorted");
        }
    }

    #[test]
    fn callback_sparse_embedder_basic() {
        let embedder = CallbackSparseEmbedder::new(|texts| {
            Ok(vec![SparseVector::new(vec![1, 2], vec![0.5, 0.5]); texts.len()])
        });

        let results = embedder
            .embed_sparse(&["hello".into()])
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].indices, vec![1, 2]);
    }

    #[test]
    fn sparse_embedder_as_trait_object() {
        let embedder: Box<dyn SparseEmbedder> = Box::new(MockSparseEmbedder::new());
        let results = embedder.embed_sparse(&["test".into()]).unwrap();
        assert_eq!(results.len(), 1);
    }

    // ── DualEmbedder ────────────────────────────────────────────────

    #[test]
    fn mock_dual_embedder_basic() {
        let embedder = MockDualEmbedder::new(384);
        assert_eq!(embedder.dim(), 384);

        let texts = vec!["hello world".into(), "foo bar".into()];
        let (dense, sparse) = embedder.embed_dual(&texts).unwrap();

        assert_eq!(dense.len(), 2);
        assert_eq!(dense[0].len(), 384);
        assert_eq!(sparse.len(), 2);
        assert!(!sparse[0].is_empty());
    }

    #[test]
    fn mock_dual_embedder_empty_batch() {
        let embedder = MockDualEmbedder::new(128);
        let (dense, sparse) = embedder.embed_dual(&[]).unwrap();
        assert!(dense.is_empty());
        assert!(sparse.is_empty());
    }

    #[test]
    fn callback_dual_embedder_basic() {
        let embedder = CallbackDualEmbedder::new(3, |texts| {
            let len = texts.len();
            let dense = vec![vec![1.0_f32, 2.0, 3.0]; len];
            let sparse = vec![SparseVector::new(vec![1, 2], vec![0.5, 0.5]); len];
            Ok((dense, sparse))
        });

        assert_eq!(embedder.dim(), 3);
        let (dense, sparse) = embedder.embed_dual(&["hello".into()]).unwrap();
        assert_eq!(dense.len(), 1);
        assert_eq!(dense[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(sparse[0].indices, vec![1, 2]);
    }

    #[test]
    fn dual_embedder_as_trait_object() {
        let embedder: Box<dyn DualEmbedder> = Box::new(MockDualEmbedder::new(64));
        assert_eq!(embedder.dim(), 64);
        let (dense, sparse) = embedder.embed_dual(&["test".into()]).unwrap();
        assert_eq!(dense.len(), 1);
        assert_eq!(sparse.len(), 1);
    }
}

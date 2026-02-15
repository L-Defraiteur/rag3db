//! Embedding trait and error types.
//!
//! The [`Embedder`] trait abstracts over embedding providers. Rag3weaver is
//! deliberately provider-agnostic: the library has **zero ML dependencies**.
//! Users provide their own [`Embedder`] implementation — candle, API call,
//! Transformers.js via wasm-bindgen, or anything else.
//!
//! [`CallbackEmbedder`] wraps a closure for quick one-off usage without
//! defining a struct. [`MockEmbedder`] is provided for testing.

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use thiserror::Error;

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
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Embed a batch of texts into vectors.
    ///
    /// Returns one vector per input text, each of dimension [`Self::dim()`].
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;

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

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Ok(texts.iter().map(|_| vec![0.0_f32; self.dim]).collect())
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

// ─── CallbackEmbedder ────────────────────────────────────────────────────────

/// Type alias for the async embed callback.
///
/// `Fn(&[String])` → `Future<Output = Result<Vec<Vec<f32>>, EmbedError>>`.
pub type EmbedFn = Box<
    dyn Fn(&[String]) -> Pin<Box<dyn Future<Output = Result<Vec<Vec<f32>>, EmbedError>> + Send>>
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
        F: Fn(&[String]) -> Pin<Box<dyn Future<Output = Result<Vec<Vec<f32>>, EmbedError>> + Send>>
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

#[async_trait]
impl Embedder for CallbackEmbedder {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        (self.embed_fn)(texts).await
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_embedder_dimensions() {
        let embedder = MockEmbedder::new(384);
        assert_eq!(embedder.dim(), 384);

        let texts = vec!["hello world".into(), "foo bar".into()];
        let results = embedder.embed(&texts).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].len(), 384);
        assert_eq!(results[1].len(), 384);
    }

    #[tokio::test]
    async fn mock_embedder_zero_vectors() {
        let embedder = MockEmbedder::new(3);
        let texts = vec!["test".into()];
        let results = embedder.embed(&texts).await.unwrap();
        assert_eq!(results[0], vec![0.0_f32; 3]);
    }

    #[tokio::test]
    async fn mock_embedder_empty_batch() {
        let embedder = MockEmbedder::new(128);
        let results = embedder.embed(&[]).await.unwrap();
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

    #[tokio::test]
    async fn embedder_as_trait_object() {
        let embedder: Box<dyn Embedder> = Box::new(MockEmbedder::new(64));
        assert_eq!(embedder.dim(), 64);
        let result = embedder.embed(&["test".into()]).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 64);
    }

    // ── CallbackEmbedder ────────────────────────────────────────────────

    #[tokio::test]
    async fn callback_embedder_basic() {
        let embedder = CallbackEmbedder::new(3, |texts| {
            let len = texts.len();
            Box::pin(async move { Ok(vec![vec![1.0_f32, 2.0, 3.0]; len]) })
        });

        assert_eq!(embedder.dim(), 3);
        let result = embedder.embed(&["hello".into(), "world".into()]).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec![1.0, 2.0, 3.0]);
    }

    #[tokio::test]
    async fn callback_embedder_error() {
        let embedder = CallbackEmbedder::new(384, |_texts| {
            Box::pin(async move {
                Err(EmbedError::ProviderError("connection refused".into()))
            })
        });

        let err = embedder.embed(&["test".into()]).await.unwrap_err();
        assert!(matches!(err, EmbedError::ProviderError(_)));
    }

    #[tokio::test]
    async fn callback_embedder_as_trait_object() {
        let embedder: Box<dyn Embedder> = Box::new(CallbackEmbedder::new(5, |texts| {
            let len = texts.len();
            Box::pin(async move { Ok(vec![vec![0.5_f32; 5]; len]) })
        }));

        assert_eq!(embedder.dim(), 5);
        let result = embedder.embed(&["a".into()]).await.unwrap();
        assert_eq!(result[0].len(), 5);
    }

    #[tokio::test]
    async fn callback_embedder_empty_batch() {
        let embedder = CallbackEmbedder::new(3, |texts| {
            let len = texts.len();
            Box::pin(async move { Ok(vec![vec![0.0_f32; 3]; len]) })
        });

        let result = embedder.embed(&[]).await.unwrap();
        assert!(result.is_empty());
    }
}

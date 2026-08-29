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

    /// **Est-ce un embedder factice ?**
    ///
    /// Un factice rend des vecteurs déterministes tirés d'un hash : parfait
    /// pour éprouver la plomberie, sans aucun sens sémantique. Le savoir
    /// permet au catalogue de refuser deux montages qui produisent des scores
    /// **plausibles et faux** — c'est la famille de défauts la plus coûteuse,
    /// puisque rien n'échoue.
    ///
    /// Défaut `false` : un embedder qui ne dit rien est supposé vrai.
    fn is_mock(&self) -> bool {
        false
    }

    /// Comment il s'appelle, pour les journaux et les refus.
    fn name(&self) -> &str {
        "?"
    }
}

/// **Un embedder partagé en est un.**
///
/// `Catalog::new` prend un `Box<dyn Embedder>` tandis qu'un modèle chargé une
/// fois vit derrière un `Arc`. Sans ce pont, un appelant qui veut le **même**
/// modèle pour indexer et pour chercher devait écrire une façade — et la
/// plupart ont préféré passer un factice « juste pour satisfaire la
/// signature », ce qui a produit exactement le montage que le catalogue refuse
/// maintenant (issue du 29 août 2026).
impl<T: Embedder + ?Sized> Embedder for std::sync::Arc<T> {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        (**self).embed(texts)
    }
    fn dim(&self) -> usize {
        (**self).dim()
    }
    fn is_mock(&self) -> bool {
        (**self).is_mock()
    }
    fn name(&self) -> &str {
        (**self).name()
    }
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


impl MockEmbedder {
    /// Il se déclare : voir [`Embedder::is_mock`].
    pub const NAME: &'static str = "MockEmbedder";
}

impl Embedder for MockEmbedder {
    fn is_mock(&self) -> bool {
        true
    }
    fn name(&self) -> &str {
        Self::NAME
    }

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

impl HashEmbedder {
    /// Il se déclare : voir [`Embedder::is_mock`].
    pub const NAME: &'static str = "HashEmbedder";
}

impl Embedder for HashEmbedder {
    fn is_mock(&self) -> bool {
        true
    }
    fn name(&self) -> &str {
        Self::NAME
    }

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

// ─── À quel rythme on prend la carte ─────────────────────────────────────────
//
// **Pourquoi ici et pas dans `burn_device`** — qui serait le foyer naturel :
// ce module-là n'est compilé que sous `burn-embedder` ou `burn-ocr`, alors que
// l'ingestion, elle, tourne avec n'importe quel `Embedder`. Une pause après un
// appel qui attend vraiment la carte vaut quel que soit le fournisseur.
//
// **Hissé depuis `dataflow::record_nodes` le 29 août 2026.** Ces deux leviers
// n'étaient branchés que sur le chemin d'ingestion : le démon d'embedding et
// tout appelant direct de `Embedder::embed` les contournaient. Mesuré ce
// jour-là, une passe E2E tenait la carte d'affichage à 100 % et 18,9 Go pour
// cette raison exacte. Le mécanisme était bon ; il était juste au mauvais
// étage.

/// **Le rapport cyclique du GPU, en pourcentage** (`RAG3WEAVER_GPU_DUTY`).
///
/// `100` (le défaut) : on enchaîne les lots, la carte est à 99 % et c'est ce
/// qu'on veut sur une machine dédiée. `70` : après chaque lot on dort assez
/// pour que la carte soit occupée sept dixièmes du temps — le reste va au
/// compositeur, à la fenêtre qu'on déplace, à la vidéo qu'on regarde pendant
/// l'ingestion.
///
/// **Pourquoi une pause et pas un quota.** Ni wgpu ni Vulkan n'exposent de
/// priorité de file ni de part garantie ; `VK_EXT_global_priority` existe dans
/// Vulkan, wgpu ne le remonte pas. Le seul levier qui reste à un programme est
/// de *ne pas soumettre* — et il est honnête : on ne prétend pas partager la
/// carte, on lui laisse des trous.
///
/// Ça marche parce que `embed()` rend des vecteurs, donc il lit le tenseur en
/// retour, donc il attend vraiment la carte. Une pause après lui est un vrai
/// trou, pas une file qui continue de se remplir derrière.
pub fn gpu_duty() -> u32 {
    std::env::var("RAG3WEAVER_GPU_DUTY")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .map(|v| v.clamp(5, 100))
        .unwrap_or_else(|| crate::regime::Regime::courant().duty())
}

/// Dort ce qu'il faut après un lot de `travail` pour tenir le rapport cyclique.
///
/// Rendue séparément de `gpu_duty()` pour être testable sans toucher au GPU :
/// c'est une règle de trois, et c'est tout ce qu'on veut vérifier.
pub fn pause_pour(travail: std::time::Duration, duty: u32) -> std::time::Duration {
    if duty >= 100 {
        return std::time::Duration::ZERO;
    }
    travail.mul_f64(f64::from(100 - duty) / f64::from(duty))
}

/// Le tour complet : chronomètre l'appel, puis souffle.
pub fn souffler(travail: std::time::Duration) {
    let pause = pause_pour(travail, gpu_duty());
    if !pause.is_zero() {
        std::thread::sleep(pause);
    }
}

// ─── La taille d'un lot ──────────────────────────────────────────────────────
//
// **Hissé depuis `dataflow::record_nodes` le 29 août 2026**, pour la même
// raison que le rythme dans `burn_device` : borner les lots ne servait qu'à
// l'ingestion, alors que c'est la carte qu'on protège — et le démon la prend
// par un autre chemin. Voir `crate::burn_device::souffler` pour l'autre moitié :
// celle-ci règle *combien de temps d'affilée* la carte ne t'appartient pas,
// l'autre *combien de temps* elle t'appartient.

/// Internal work item for embedding.
/// Budget de texte par forward, en **caractères**.
///
/// Mesuré le 27 août 2026 sur BGE-M3 / Radeon R9700
/// (`examples/burn_throughput.rs`) : le débit culmine autour de **2 048 jetons
/// par passe**, quelle que soit la répartition — 64×32, 16×128 et 4×512 jetons
/// donnent 7 550, 7 417 et 6 210 tok/s. Au-delà il **redescend** : 5 507 à
/// 8 192 jetons, 5 378 à 32 768. Ce n'est donc pas « plus gros c'est mieux ».
///
/// Le budget est en caractères et pas en jetons parce que **le tokenizer vit
/// dans l'embedder, pas ici** : le lui demander coûterait une passe avant la
/// passe. Quatre caractères par jeton est l'ordre de grandeur de ce corpus —
/// c'est un garde-fou, pas une science, et c'est pour ça qu'il est large.
pub const EMBED_CHAR_BUDGET: usize = 8_192;

/// **La longueur d'une saccade**, en caractères de texte par appel GPU
/// (`RAG3WEAVER_EMBED_CHAR_BUDGET`).
///
/// Le défaut, 8 192 (~2 048 jetons), est l'optimum de débit mesuré le 27 août
/// 2026. Il coûte des rafales GPU de 295 ms en moyenne et 560 ms au pire
/// (mesuré le 28) : pendant ce temps la carte ne rend pas la main, et si elle
/// porte aussi l'affichage, ça se voit.
///
/// **Le plancher est un chunk**, pas ce nombre. `budget_batches` met un
/// élément qui dépasse le budget dans son propre lot — on ne peut pas couper
/// un chunk ici sans changer ce qui est embarqué. Avec un `max_size` de 1 500
/// (256 pour le titre préfixé), descendre sous 1 500 ne raccourcit donc plus
/// rien. La plage qui a un effet va de la taille de chunk au défaut.
///
/// À distinguer de `RAG3WEAVER_GPU_DUTY`, qui écarte les rafales sans les
/// raccourcir : l'un règle *combien de temps* la carte t'appartient, l'autre
/// *combien de temps d'affilée* elle ne t'appartient pas.
pub fn embed_char_budget() -> usize {
    std::env::var("RAG3WEAVER_EMBED_CHAR_BUDGET")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|v| *v > 0)
        // Le régime après la variable, jamais devant : un réglage explicite
        // gagne toujours (`crate::regime`).
        .unwrap_or_else(|| crate::regime::Regime::courant().budget_caracteres())
}

/// Découpe une liste de travaux en sous-lots bornés **par la quantité de
/// texte**, et pas seulement par le nombre d'éléments.
///
/// Un compte fixe de documents est structurellement faux sur un corpus
/// hétérogène : « 32 » vaut six cents jetons pour des titres et trente mille
/// pour des pages. Le premier lot passe inaperçu, le second demande d'un coup
/// une allocation que la carte n'a pas — et le symptôme n'est pas une erreur
/// claire, c'est un poste qui s'écroule.
///
/// **Un élément seul dépassant le budget forme son propre lot** : on ne peut
/// pas le couper ici sans changer ce qui est embarqué, donc on le laisse
/// passer plutôt que de mentir sur le contenu.
pub fn budget_batches(lens: &[usize], max_items: usize, max_chars: usize) -> Vec<std::ops::Range<usize>> {
    let mut out: Vec<std::ops::Range<usize>> = Vec::new();
    let mut start = 0usize;
    let mut chars = 0usize;
    for i in 0..lens.len() {
        let plein = i > start && (i - start >= max_items || chars + lens[i] > max_chars);
        if plein {
            out.push(start..i);
            start = i;
            chars = 0;
        }
        chars += lens[i];
    }
    if start < lens.len() {
        out.push(start..lens.len());
    }
    out
}

#[cfg(test)]
mod tests_rythme_et_lots {
    use super::*;

    /// **Le rapport cyclique, en règle de trois.**
    ///
    /// 99 % d'occupation n'est pas un défaut en soi ; c'en est un quand la
    /// carte porte aussi l'affichage. Ni wgpu ni Vulkan n'exposent de quota,
    /// donc le seul levier honnête est de ne pas soumettre : on chronomètre
    /// le lot, on dort la fraction qui manque.
    #[test]
    fn le_rapport_cyclique_est_une_regle_de_trois() {
        use std::time::Duration;
        let lot = Duration::from_millis(100);

        // 100 % : on n'attend pas. C'est le défaut, et il ne coûte rien —
        // ni appel d'horloge inutile, ni branchement au milieu d'une boucle.
        assert_eq!(pause_pour(lot, 100), Duration::ZERO);

        // 70 % voulu : 100 ms de calcul veulent 43 ms de pause, parce que
        // 100 / 143 ≈ 0,70.
        let p = pause_pour(lot, 70);
        assert_eq!(p.as_millis(), 42);
        let cycle = lot + p;
        let obtenu = 100.0 * lot.as_secs_f64() / cycle.as_secs_f64();
        assert!((obtenu - 70.0).abs() < 1.0, "obtenu {obtenu:.1} %");

        // 50 % : autant de pause que de calcul.
        assert_eq!(pause_pour(lot, 50), lot);

        // Un lot court donne une pause courte : la règle est proportionnelle,
        // donc elle ne dépend pas de la taille des lots — c'est ce qui la rend
        // composable avec le budget en caractères.
        assert_eq!(pause_pour(Duration::from_millis(10), 50).as_millis(), 10);
    }

    fn plages(lens: &[usize], max_items: usize, max_chars: usize) -> Vec<(usize, usize)> {
        budget_batches(lens, max_items, max_chars)
            .into_iter()
            .map(|r| (r.start, r.end))
            .collect()
    }

    #[test]
    fn le_budget_ferme_le_lot_sur_le_texte_pas_sur_le_nombre() {
        // Quatre textes de 3 000 caractères : le compte en autoriserait 32,
        // le budget n'en laisse passer que deux (3 × 3 000 = 9 000 > 8 192).
        // C'est tout l'objet du changement.
        let lens = [3_000, 3_000, 3_000, 3_000];
        assert_eq!(plages(&lens, 32, 8_192), vec![(0, 2), (2, 4)]);
    }

    #[test]
    fn le_compte_reste_une_borne_dure() {
        // Des textes minuscules : c'est le nombre qui ferme, pas le budget.
        let lens = [10; 10];
        assert_eq!(plages(&lens, 4, 8_192), vec![(0, 4), (4, 8), (8, 10)]);
    }

    #[test]
    fn un_element_seul_trop_gros_forme_son_lot() {
        // On ne peut pas le couper sans changer ce qui est embarqué : il passe
        // seul plutôt qu'on mente sur le contenu.
        let lens = [100, 99_999, 100];
        assert_eq!(plages(&lens, 32, 8_192), vec![(0, 1), (1, 2), (2, 3)]);
    }

    // ── Le budget de lot (doc 08) ───────────────────────────────────────


    #[test]
    fn rien_a_faire_ne_fait_rien() {
        assert!(plages(&[], 32, 8_192).is_empty());
    }

    #[test]
    fn les_plages_couvrent_tout_sans_trou_ni_recouvrement() {
        // L'invariant qui compte vraiment : aucun texte perdu, aucun embarqué
        // deux fois. Un découpage qui saute une entrée la laisse sans vecteur,
        // et rien ne le dirait.
        for max_items in [1usize, 3, 7, 64] {
            for max_chars in [1usize, 500, 8_192] {
                let lens: Vec<usize> = (0..37).map(|i| (i * 137) % 4_000).collect();
                let p = plages(&lens, max_items, max_chars);
                assert_eq!(p.first().map(|r| r.0), Some(0), "{max_items}/{max_chars}");
                assert_eq!(p.last().map(|r| r.1), Some(lens.len()), "{max_items}/{max_chars}");
                for w in p.windows(2) {
                    assert_eq!(w[0].1, w[1].0, "trou ou recouvrement : {p:?}");
                }
                assert!(p.iter().all(|r| r.1 > r.0), "lot vide : {p:?}");
                assert!(
                    p.iter().all(|r| r.1 - r.0 <= max_items),
                    "borne de compte dépassée : {p:?}"
                );
            }
        }
    }
}

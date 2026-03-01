//! BGE-M3 multi-representation embedder powered by candle.
//!
//! Uses BAAI/bge-m3 (XLM-RoBERTa-large, ~568M params) to produce both dense
//! and sparse embeddings. Dense = CLS pooling + L2 normalize (1024 dims).
//! Sparse = learned linear layer (1024→1) + ReLU, scattered by token_id.
//!
//! The sparse representation is *learned* (unlike BM42's attention hack),
//! providing higher quality lexical matching with implicit term expansion.
//!
//! Native only — too large (~2.2GB) for WASM. Use BM42 + MiniLM for WASM.
//!
//! # Example
//!
//! ```ignore
//! use rag3weaver::bge_m3_embedder::BgeM3Embedder;
//!
//! let embedder = BgeM3Embedder::new()?;
//! let dense = embedder.embed(&["hello world".into()]).await?;
//! let sparse = embedder.embed_sparse(&["hello world".into()]).await?;
//! assert_eq!(dense[0].len(), 1024);
//! assert!(!sparse[0].is_empty());
//! ```

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use candle_core::{DType, Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::xlm_roberta::{Config, XLMRobertaModel};
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer};

use crate::embedder::{DualEmbedder, EmbedError, Embedder, SparseEmbedder};
use crate::sparse_index::SparseVector;

const DTYPE: DType = DType::F32;

// XLM-RoBERTa special token IDs (standard across all XLM-R models)
const CLS_TOKEN_ID: u32 = 0; // <s>
const PAD_TOKEN_ID: u32 = 1; // <pad>
const EOS_TOKEN_ID: u32 = 2; // </s>
const UNK_TOKEN_ID: u32 = 3; // <unk>

/// BGE-M3 embedder: dense (1024d) + sparse (learned lexical weights).
///
/// Implements both [`Embedder`] and [`SparseEmbedder`] traits.
pub struct BgeM3Embedder {
    model: XLMRobertaModel,
    sparse_linear: candle_nn::Linear, // hidden_size → 1
    tokenizer: Mutex<Tokenizer>,
    device: Device,
    dim: usize,
}

impl BgeM3Embedder {
    fn new_inner(
        config: Config,
        mut tokenizer: Tokenizer,
        model_vb: VarBuilder,
        sparse_vb: VarBuilder,
        device: Device,
    ) -> Result<Self, EmbedError> {
        let dim = config.hidden_size;

        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            pad_token: "<pad>".to_string(),
            pad_id: PAD_TOKEN_ID,
            ..Default::default()
        }));

        // Load XLM-RoBERTa backbone — try bare paths, then "roberta" prefix
        let model = match XLMRobertaModel::new(&config, model_vb.clone()) {
            Ok(m) => m,
            Err(_) => XLMRobertaModel::new(&config, model_vb.pp("roberta"))
                .map_err(|e| EmbedError::ProviderError(format!("load XLM-RoBERTa: {e}")))?,
        };

        // Load sparse linear layer (hidden_size → 1)
        let sparse_linear = candle_nn::linear(dim, 1, sparse_vb)
            .map_err(|e| EmbedError::ProviderError(format!("load sparse_linear: {e}")))?;

        Ok(Self {
            model,
            sparse_linear,
            tokenizer: Mutex::new(tokenizer),
            device,
            dim,
        })
    }

    /// Load BGE-M3 from HuggingFace Hub (cached locally, ~2.2GB first download).
    #[cfg(feature = "bge-m3")]
    pub fn new() -> Result<Self, EmbedError> {
        Self::from_repo("BAAI/bge-m3", None)
    }

    /// Load a custom BGE-M3 compatible model from HuggingFace Hub.
    ///
    /// Expects: `config.json`, `tokenizer.json`, `pytorch_model.bin`, `sparse_linear.pt`.
    #[cfg(feature = "bge-m3")]
    pub fn from_repo(model_id: &str, revision: Option<&str>) -> Result<Self, EmbedError> {
        use hf_hub::api::sync::Api;

        let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);
        let api =
            Api::new().map_err(|e| EmbedError::ProviderError(format!("hf-hub init: {e}")))?;
        let repo = if let Some(rev) = revision {
            api.repo(hf_hub::Repo::with_revision(
                model_id.into(),
                hf_hub::RepoType::Model,
                rev.into(),
            ))
        } else {
            api.model(model_id.into())
        };

        let config_path = repo
            .get("config.json")
            .map_err(|e| EmbedError::ProviderError(format!("download config.json: {e}")))?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .map_err(|e| EmbedError::ProviderError(format!("download tokenizer.json: {e}")))?;
        let weights_path = repo
            .get("pytorch_model.bin")
            .map_err(|e| EmbedError::ProviderError(format!("download pytorch_model.bin: {e}")))?;
        let sparse_path = repo
            .get("sparse_linear.pt")
            .map_err(|e| EmbedError::ProviderError(format!("download sparse_linear.pt: {e}")))?;

        let config: Config = serde_json::from_str(
            &std::fs::read_to_string(&config_path)
                .map_err(|e| EmbedError::ProviderError(format!("read config: {e}")))?,
        )
        .map_err(|e| EmbedError::ProviderError(format!("parse config: {e}")))?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;

        // Load main model weights (PyTorch pickle — no safetensors available for BGE-M3)
        let model_vb = VarBuilder::from_pth(&weights_path, DTYPE, &device)
            .map_err(|e| EmbedError::ProviderError(format!("load weights: {e}")))?;

        // Load sparse linear weights (separate PyTorch pickle, ~3.5KB)
        let sparse_vb = VarBuilder::from_pth(&sparse_path, DTYPE, &device)
            .map_err(|e| EmbedError::ProviderError(format!("load sparse_linear: {e}")))?;

        Self::new_inner(config, tokenizer, model_vb, sparse_vb, device)
    }

    /// Tokenize + forward pass through XLM-RoBERTa.
    ///
    /// Returns `(hidden_states [batch, seq, dim], input_ids per sample, attention_mask)`.
    fn forward_pass(
        &self,
        texts: &[String],
    ) -> Result<(Tensor, Vec<Vec<u32>>, Tensor), EmbedError> {
        let tokenizer = self.tokenizer.lock().unwrap();
        let str_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let encodings = tokenizer
            .encode_batch(str_refs, true)
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;

        let input_ids_vecs: Vec<Vec<u32>> =
            encodings.iter().map(|e| e.get_ids().to_vec()).collect();

        let token_ids: Vec<Tensor> = encodings
            .iter()
            .map(|e| Tensor::new(e.get_ids(), &self.device))
            .collect::<Result<_, _>>()
            .map_err(|e| EmbedError::ProviderError(format!("tensor: {e}")))?;

        let attention_masks: Vec<Tensor> = encodings
            .iter()
            .map(|e| Tensor::new(e.get_attention_mask(), &self.device))
            .collect::<Result<_, _>>()
            .map_err(|e| EmbedError::ProviderError(format!("tensor: {e}")))?;

        let input_ids_t = Tensor::stack(&token_ids, 0)
            .map_err(|e| EmbedError::ProviderError(format!("stack: {e}")))?;
        let attention_mask_t = Tensor::stack(&attention_masks, 0)
            .map_err(|e| EmbedError::ProviderError(format!("stack: {e}")))?;
        let token_type_ids = input_ids_t
            .zeros_like()
            .map_err(|e| EmbedError::ProviderError(format!("zeros: {e}")))?;

        // XLMRobertaModel::forward → [batch, seq_len, hidden_size]
        let hidden_states = self
            .model
            .forward(
                &input_ids_t,
                &attention_mask_t,
                &token_type_ids,
                None,
                None,
                None,
            )
            .map_err(|e| EmbedError::ProviderError(format!("forward: {e}")))?;

        Ok((hidden_states, input_ids_vecs, attention_mask_t))
    }

    /// Extract dense embeddings from pre-computed hidden states.
    /// CLS token (position 0) + L2 normalize → Vec<f32> (1024 dims).
    fn extract_dense(&self, hidden_states: &Tensor, batch_size: usize) -> Result<Vec<Vec<f32>>, EmbedError> {
        let cls = hidden_states
            .narrow(1, 0, 1)
            .and_then(|t| t.squeeze(1))
            .map_err(|e| EmbedError::ProviderError(format!("cls: {e}")))?;

        let norms = cls
            .sqr()
            .and_then(|s| s.sum_keepdim(1))
            .and_then(|s| s.sqrt())
            .map_err(|e| EmbedError::ProviderError(format!("norm: {e}")))?;
        let normalized = cls
            .broadcast_div(&norms)
            .map_err(|e| EmbedError::ProviderError(format!("normalize: {e}")))?;

        let mut results = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let vec = normalized
                .get(i)
                .and_then(|t| t.to_vec1::<f32>())
                .map_err(|e| EmbedError::ProviderError(format!("extract: {e}")))?;
            results.push(vec);
        }
        Ok(results)
    }

    /// Extract sparse embeddings from pre-computed hidden states.
    /// W_lex(hidden) → ReLU → scatter by token_id (max) → SparseVector.
    fn extract_sparse(&self, hidden_states: &Tensor, input_ids_vecs: &[Vec<u32>]) -> Result<Vec<SparseVector>, EmbedError> {
        let token_weights = self
            .sparse_linear
            .forward(hidden_states)
            .and_then(|t| t.squeeze(2))
            .and_then(|t| t.relu())
            .map_err(|e| EmbedError::ProviderError(format!("sparse linear: {e}")))?;

        let batch_size = input_ids_vecs.len();
        let mut results = Vec::with_capacity(batch_size);

        for i in 0..batch_size {
            let weights: Vec<f32> = token_weights
                .get(i)
                .and_then(|t| t.to_vec1::<f32>())
                .map_err(|e| EmbedError::ProviderError(format!("extract: {e}")))?;

            let ids = &input_ids_vecs[i];

            let mut token_max: HashMap<u32, f32> = HashMap::new();
            for (pos, &token_id) in ids.iter().enumerate() {
                if token_id == CLS_TOKEN_ID
                    || token_id == EOS_TOKEN_ID
                    || token_id == PAD_TOKEN_ID
                    || token_id == UNK_TOKEN_ID
                {
                    continue;
                }
                if pos >= weights.len() {
                    break;
                }
                let w = weights[pos];
                if w > 0.0 {
                    let entry = token_max.entry(token_id).or_insert(0.0);
                    if w > *entry {
                        *entry = w;
                    }
                }
            }

            let mut entries: Vec<(u32, f32)> = token_max.into_iter().collect();
            entries.sort_by_key(|(id, _)| *id);

            let indices: Vec<u32> = entries.iter().map(|(id, _)| *id).collect();
            let values: Vec<f32> = entries.iter().map(|(_, w)| *w).collect();

            results.push(SparseVector::new(indices, values));
        }

        Ok(results)
    }

    /// Dense embedding: forward pass + CLS + L2 normalize.
    pub fn embed_dense_sync(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        let (hidden_states, _, _) = self.forward_pass(texts)?;
        self.extract_dense(&hidden_states, texts.len())
    }

    /// Sparse embedding: forward pass + sparse_linear + ReLU + scatter.
    pub fn embed_sparse_sync(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        let (hidden_states, input_ids_vecs, _) = self.forward_pass(texts)?;
        self.extract_sparse(&hidden_states, &input_ids_vecs)
    }

    /// Dual embedding: single forward pass → dense + sparse.
    pub fn embed_dual_sync(&self, texts: &[String]) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError> {
        if texts.is_empty() {
            return Ok((vec![], vec![]));
        }
        let (hidden_states, input_ids_vecs, _) = self.forward_pass(texts)?;
        let dense = self.extract_dense(&hidden_states, texts.len())?;
        let sparse = self.extract_sparse(&hidden_states, &input_ids_vecs)?;
        Ok((dense, sparse))
    }
}

#[async_trait]
impl Embedder for BgeM3Embedder {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        self.embed_dense_sync(texts)
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

#[async_trait]
impl SparseEmbedder for BgeM3Embedder {
    async fn embed_sparse(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError> {
        self.embed_sparse_sync(texts)
    }
}

#[async_trait]
impl DualEmbedder for BgeM3Embedder {
    async fn embed_dual(
        &self,
        texts: &[String],
    ) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError> {
        self.embed_dual_sync(texts)
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // All tests require ~2.2GB model download: cargo test --features bge-m3 bge_m3 -- --ignored
    // Model is loaded once and shared across all tests via OnceLock.

    #[cfg(feature = "bge-m3")]
    fn shared_embedder() -> &'static BgeM3Embedder {
        use std::sync::OnceLock;
        static EMBEDDER: OnceLock<BgeM3Embedder> = OnceLock::new();
        EMBEDDER.get_or_init(|| {
            let start = std::time::Instant::now();
            let e = BgeM3Embedder::new().unwrap();
            eprintln!("[bge-m3] model loaded in {:.2}s", start.elapsed().as_secs_f64());
            e
        })
    }

    #[cfg(feature = "bge-m3")]
    #[tokio::test]
    #[ignore]
    async fn bge_m3_dense_basic() {
        let embedder = shared_embedder();
        assert_eq!(embedder.dim(), 1024);

        let start = std::time::Instant::now();
        let results = embedder.embed(&["hello world".into()]).await.unwrap();
        eprintln!("[dense 1 text] {:.2}s", start.elapsed().as_secs_f64());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].len(), 1024);

        // Should be L2-normalized (norm ≈ 1.0)
        let norm: f32 = results[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "norm={norm}, expected ~1.0");
    }

    #[cfg(feature = "bge-m3")]
    #[tokio::test]
    #[ignore]
    async fn bge_m3_sparse_basic() {
        let embedder = shared_embedder();

        let start = std::time::Instant::now();
        let results = embedder
            .embed_sparse(&["hello world".into()])
            .await
            .unwrap();
        eprintln!("[sparse 1 text] {:.2}s, nnz={}", start.elapsed().as_secs_f64(), results[0].nnz());

        assert_eq!(results.len(), 1);
        assert!(!results[0].is_empty(), "sparse vector should not be empty");
        assert!(results[0].nnz() >= 2, "nnz={}", results[0].nnz());

        // All weights should be positive (after ReLU)
        for &w in &results[0].values {
            assert!(w > 0.0, "weight should be positive, got {w}");
        }

        // No special token IDs should appear
        for &id in &results[0].indices {
            assert!(
                id != CLS_TOKEN_ID && id != PAD_TOKEN_ID && id != EOS_TOKEN_ID && id != UNK_TOKEN_ID,
                "special token {id} should be excluded"
            );
        }
    }

    #[cfg(feature = "bge-m3")]
    #[tokio::test]
    #[ignore]
    async fn bge_m3_batch() {
        let embedder = shared_embedder();

        let start = std::time::Instant::now();
        let dense = embedder
            .embed(&[
                "rust programming".into(),
                "cooking pasta".into(),
            ])
            .await
            .unwrap();
        eprintln!("[dense 2 texts] {:.2}s", start.elapsed().as_secs_f64());

        assert_eq!(dense.len(), 2);
        assert_eq!(dense[0].len(), 1024);
        assert_eq!(dense[1].len(), 1024);

        let start = std::time::Instant::now();
        let sparse = embedder
            .embed_sparse(&[
                "rust programming".into(),
                "cooking pasta".into(),
            ])
            .await
            .unwrap();
        eprintln!("[sparse 2 texts] {:.2}s", start.elapsed().as_secs_f64());

        assert_eq!(sparse.len(), 2);
        assert!(!sparse[0].is_empty());
        assert!(!sparse[1].is_empty());
    }

    #[cfg(feature = "bge-m3")]
    #[tokio::test]
    #[ignore]
    async fn bge_m3_empty_batch() {
        let embedder = shared_embedder();
        assert!(embedder.embed(&[]).await.unwrap().is_empty());
        assert!(embedder.embed_sparse(&[]).await.unwrap().is_empty());
    }

    #[cfg(feature = "bge-m3")]
    #[tokio::test]
    #[ignore]
    async fn bge_m3_dense_cosine_similarity() {
        let embedder = shared_embedder();
        let vecs = embedder
            .embed(&[
                "Rust programming language".into(),
                "Rust systems programming".into(),
                "cooking pasta recipes".into(),
            ])
            .await
            .unwrap();

        let sim_related = cosine(&vecs[0], &vecs[1]);
        let sim_unrelated = cosine(&vecs[0], &vecs[2]);

        eprintln!("[cosine] related={sim_related:.4}, unrelated={sim_unrelated:.4}");
        assert!(
            sim_related > sim_unrelated,
            "related={sim_related:.4}, unrelated={sim_unrelated:.4}"
        );
    }

    #[cfg(feature = "bge-m3")]
    #[tokio::test]
    #[ignore]
    async fn bge_m3_sparse_shared_tokens() {
        let embedder = shared_embedder();
        let results = embedder
            .embed_sparse(&[
                "rust programming".into(),
                "rust systems programming".into(),
            ])
            .await
            .unwrap();

        let set0: std::collections::HashSet<u32> =
            results[0].indices.iter().copied().collect();
        let set1: std::collections::HashSet<u32> =
            results[1].indices.iter().copied().collect();
        let overlap: usize = set0.intersection(&set1).count();
        eprintln!("[sparse overlap] {overlap} shared tokens out of {} / {}", set0.len(), set1.len());
        assert!(
            overlap >= 1,
            "similar texts should share at least 1 token, got {overlap}"
        );
    }

    #[cfg(feature = "bge-m3")]
    #[tokio::test]
    #[ignore]
    async fn bge_m3_deterministic() {
        let embedder = shared_embedder();

        let d1 = embedder.embed(&["test".into()]).await.unwrap();
        let d2 = embedder.embed(&["test".into()]).await.unwrap();
        for (a, b) in d1[0].iter().zip(d2[0].iter()) {
            assert!((a - b).abs() < 1e-6, "dense should be deterministic");
        }

        let s1 = embedder.embed_sparse(&["test".into()]).await.unwrap();
        let s2 = embedder.embed_sparse(&["test".into()]).await.unwrap();
        assert_eq!(s1[0].indices, s2[0].indices);
        for (a, b) in s1[0].values.iter().zip(s2[0].values.iter()) {
            assert!((a - b).abs() < 1e-6, "sparse should be deterministic");
        }
    }

    #[cfg(feature = "bge-m3")]
    #[tokio::test]
    #[ignore]
    async fn bge_m3_as_embedder_trait() {
        let embedder: &dyn Embedder = shared_embedder();
        assert_eq!(embedder.dim(), 1024);
        let result = embedder.embed(&["test".into()]).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1024);
    }

    #[cfg(feature = "bge-m3")]
    #[tokio::test]
    #[ignore]
    async fn bge_m3_dual_matches_separate() {
        let embedder = shared_embedder();
        let texts = vec!["rust programming".into(), "cooking pasta".into()];

        let (dual_dense, dual_sparse) = embedder.embed_dual(&texts).await.unwrap();
        let sep_dense = embedder.embed(&texts).await.unwrap();
        let sep_sparse = embedder.embed_sparse(&texts).await.unwrap();

        // Dense results should be identical (same forward pass extraction)
        for (i, (dd, sd)) in dual_dense.iter().zip(sep_dense.iter()).enumerate() {
            for (a, b) in dd.iter().zip(sd.iter()) {
                assert!((a - b).abs() < 1e-5, "dense mismatch at text {i}");
            }
        }

        // Sparse results should be identical
        for (i, (ds, ss)) in dual_sparse.iter().zip(sep_sparse.iter()).enumerate() {
            assert_eq!(ds.indices, ss.indices, "sparse indices mismatch at text {i}");
            for (a, b) in ds.values.iter().zip(ss.values.iter()) {
                assert!((a - b).abs() < 1e-5, "sparse values mismatch at text {i}");
            }
        }
    }

    #[cfg(feature = "bge-m3")]
    #[tokio::test]
    #[ignore]
    async fn bge_m3_as_sparse_embedder_trait() {
        let embedder: &dyn SparseEmbedder = shared_embedder();
        let result = embedder.embed_sparse(&["test".into()]).await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(!result[0].is_empty());
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }
}

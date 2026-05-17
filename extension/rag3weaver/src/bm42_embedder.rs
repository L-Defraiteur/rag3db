//! BM42-style sparse embedder powered by candle.
//!
//! Uses the same BERT model as [`CandleEmbedder`] (e.g. all-MiniLM-L6-v2) but
//! extracts CLS attention weights instead of mean-pooled hidden states.
//!
//! One forward pass produces a sparse vector where each dimension is a
//! WordPiece token ID and the weight is the CLS→token attention from the last
//! layer (averaged across heads). Special tokens ([CLS], [SEP], [PAD]) are
//! excluded. Duplicate sub-words have their weights summed.
//!
//! # Example
//!
//! ```ignore
//! use rag3weaver::bm42_embedder::Bm42Embedder;
//! use rag3weaver::candle_embedder::DefaultModel;
//!
//! let embedder = Bm42Embedder::new(DefaultModel::MiniLM)?;
//! let sparse = embedder.embed_sparse(&["hello world".into()])?;
//! assert!(!sparse[0].is_empty());
//! ```

use std::collections::HashMap;
use std::sync::Mutex;

use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer};

use crate::bm42_model::{Bm42Model, Config, DTYPE};
use crate::candle_embedder::DefaultModel;
use crate::embedder::{EmbedError, SparseEmbedder};
use crate::sparse_index::SparseVector;

/// BM42-style sparse embedder: CLS attention weights as sparse vectors.
pub struct Bm42Embedder {
    model: Bm42Model,
    tokenizer: Mutex<Tokenizer>,
    device: Device,
}

impl Bm42Embedder {
    fn new_inner(
        config: Config,
        mut tokenizer: Tokenizer,
        vb: VarBuilder,
        device: Device,
    ) -> Result<Self, EmbedError> {
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            ..Default::default()
        }));
        let model = Bm42Model::load(vb, &config)
            .map_err(|e| EmbedError::ProviderError(format!("load BM42 model: {e}")))?;
        Ok(Self {
            model,
            tokenizer: Mutex::new(tokenizer),
            device,
        })
    }

    /// Load a pre-configured model from HuggingFace Hub (cached locally).
    #[cfg(feature = "candle-embedder")]
    pub fn new(default_model: DefaultModel) -> Result<Self, EmbedError> {
        Self::from_repo(default_model.model_id(), None)
    }

    /// Load a custom BERT-compatible model from HuggingFace Hub.
    #[cfg(feature = "candle-embedder")]
    pub fn from_repo(model_id: &str, revision: Option<&str>) -> Result<Self, EmbedError> {
        use hf_hub::api::sync::Api;

        let device = Device::Cpu;
        let api = Api::new()
            .map_err(|e| EmbedError::ProviderError(format!("hf-hub init: {e}")))?;
        let repo = if let Some(rev) = revision {
            api.repo(hf_hub::Repo::with_revision(
                model_id.into(),
                hf_hub::RepoType::Model,
                rev.into(),
            ))
        } else {
            api.model(model_id.into())
        };

        let config_path = repo.get("config.json")
            .map_err(|e| EmbedError::ProviderError(format!("download config.json: {e}")))?;
        let tokenizer_path = repo.get("tokenizer.json")
            .map_err(|e| EmbedError::ProviderError(format!("download tokenizer.json: {e}")))?;
        let weights_path = repo.get("model.safetensors")
            .map_err(|e| EmbedError::ProviderError(format!("download model.safetensors: {e}")))?;

        let config: Config = serde_json::from_str(
            &std::fs::read_to_string(&config_path)
                .map_err(|e| EmbedError::ProviderError(format!("read config: {e}")))?,
        ).map_err(|e| EmbedError::ProviderError(format!("parse config: {e}")))?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &device)
                .map_err(|e| EmbedError::ProviderError(format!("load weights: {e}")))?
        };

        Self::new_inner(config, tokenizer, vb, device)
    }

    /// Load from in-memory bytes (WASM-compatible, no filesystem).
    pub fn from_bytes(
        config_json: &[u8],
        tokenizer_json: &[u8],
        weights: Vec<u8>,
    ) -> Result<Self, EmbedError> {
        let config: Config = serde_json::from_slice(config_json)
            .map_err(|e| EmbedError::ProviderError(format!("config parse: {e}")))?;
        let tokenizer = Tokenizer::from_bytes(tokenizer_json)
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer parse: {e}")))?;
        let device = Device::Cpu;
        let vb = VarBuilder::from_buffered_safetensors(weights, DTYPE, &device)
            .map_err(|e| EmbedError::ProviderError(format!("weights load: {e}")))?;
        Self::new_inner(config, tokenizer, vb, device)
    }

    /// Synchronous sparse embedding — extract CLS attention weights.
    pub fn embed_sparse_sync(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let tokenizer = self.tokenizer.lock().unwrap();
        let str_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let encodings = tokenizer
            .encode_batch(str_refs, true)
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;

        // Build tensor inputs
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

        let token_ids_t = Tensor::stack(&token_ids, 0)
            .map_err(|e| EmbedError::ProviderError(format!("stack: {e}")))?;
        let attention_mask_t = Tensor::stack(&attention_masks, 0)
            .map_err(|e| EmbedError::ProviderError(format!("stack: {e}")))?;
        let token_type_ids = token_ids_t.zeros_like()
            .map_err(|e| EmbedError::ProviderError(format!("zeros: {e}")))?;

        // Forward pass → (hidden_states, last_attention_probs)
        // attention_probs shape: [batch, num_heads, seq_len, seq_len]
        let (_hidden, attn_probs) = self.model
            .forward(&token_ids_t, &token_type_ids, Some(&attention_mask_t))
            .map_err(|e| EmbedError::ProviderError(format!("forward: {e}")))?;

        // Average across heads → [batch, seq_len, seq_len]
        let avg_attn = attn_probs.mean(1)
            .map_err(|e| EmbedError::ProviderError(format!("mean heads: {e}")))?;

        // CLS row (position 0) → [batch, seq_len]
        let cls_attn = avg_attn
            .narrow(1, 0, 1)
            .and_then(|t| t.squeeze(1))
            .map_err(|e| EmbedError::ProviderError(format!("cls row: {e}")))?;

        // Build sparse vectors
        let batch_size = texts.len();
        let mut results = Vec::with_capacity(batch_size);

        for i in 0..batch_size {
            let weights: Vec<f32> = cls_attn
                .get(i)
                .and_then(|t| t.to_vec1::<f32>())
                .map_err(|e| EmbedError::ProviderError(format!("extract: {e}")))?;

            let ids = encodings[i].get_ids();
            let special_mask = encodings[i].get_special_tokens_mask();

            // Accumulate weights per token_id, skipping specials + padding
            let mut token_weights: HashMap<u32, f32> = HashMap::new();
            for (pos, (&token_id, &is_special)) in
                ids.iter().zip(special_mask.iter()).enumerate()
            {
                if is_special == 1 || pos >= weights.len() {
                    continue;
                }
                let w = weights[pos];
                if w > 0.0 {
                    *token_weights.entry(token_id).or_default() += w;
                }
            }

            // Sort by token_id for consistent ordering
            let mut entries: Vec<(u32, f32)> = token_weights.into_iter().collect();
            entries.sort_by_key(|(id, _)| *id);

            let indices: Vec<u32> = entries.iter().map(|(id, _)| *id).collect();
            let values: Vec<f32> = entries.iter().map(|(_, w)| *w).collect();

            results.push(SparseVector::new(indices, values));
        }

        Ok(results)
    }
}


impl SparseEmbedder for Bm42Embedder {
    fn embed_sparse(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError> {
        self.embed_sparse_sync(texts)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore] // requires network + model download: cargo test -- --ignored
    fn bm42_minilm_sparse_basic() {
        let embedder = Bm42Embedder::new(DefaultModel::MiniLM).unwrap();
        let results = embedder
            .embed_sparse(&["hello world".into()])
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(!results[0].is_empty(), "sparse vector should not be empty");
        // "hello world" has ~2-4 sub-word tokens → sparse vector should have entries
        assert!(results[0].nnz() >= 2, "nnz={}", results[0].nnz());

        // All weights should be positive (attention weights after softmax)
        for &w in &results[0].values {
            assert!(w > 0.0, "weight should be positive, got {w}");
        }
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore]
    fn bm42_minilm_batch() {
        let embedder = Bm42Embedder::new(DefaultModel::MiniLM).unwrap();
        let results = embedder
            .embed_sparse(&[
                "rust programming language".into(),
                "cooking pasta recipes".into(),
            ])
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(!results[0].is_empty());
        assert!(!results[1].is_empty());
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore]
    fn bm42_minilm_deterministic() {
        let embedder = Bm42Embedder::new(DefaultModel::MiniLM).unwrap();
        let r1 = embedder
            .embed_sparse(&["test".into()])
            .unwrap();
        let r2 = embedder
            .embed_sparse(&["test".into()])
            .unwrap();

        assert_eq!(r1[0].indices, r2[0].indices);
        for (a, b) in r1[0].values.iter().zip(r2[0].values.iter()) {
            assert!((a - b).abs() < 1e-6, "weights should be deterministic");
        }
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore]
    fn bm42_minilm_empty_batch() {
        let embedder = Bm42Embedder::new(DefaultModel::MiniLM).unwrap();
        let results = embedder.embed_sparse(&[]).unwrap();
        assert!(results.is_empty());
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore]
    fn bm42_minilm_similar_texts_share_tokens() {
        let embedder = Bm42Embedder::new(DefaultModel::MiniLM).unwrap();
        let results = embedder
            .embed_sparse(&[
                "rust programming".into(),
                "rust systems programming".into(),
            ])
            .unwrap();

        // Both mention "rust" → should share at least the same sub-word token IDs
        let set0: std::collections::HashSet<u32> =
            results[0].indices.iter().copied().collect();
        let set1: std::collections::HashSet<u32> =
            results[1].indices.iter().copied().collect();
        let overlap: usize = set0.intersection(&set1).count();
        assert!(
            overlap >= 1,
            "similar texts should share at least 1 token, got {overlap}"
        );
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore]
    fn bm42_as_sparse_embedder_trait() {
        let embedder: Box<dyn SparseEmbedder> =
            Box::new(Bm42Embedder::new(DefaultModel::MiniLM).unwrap());
        let results = embedder.embed_sparse(&["test".into()]).unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].is_empty());
    }
}

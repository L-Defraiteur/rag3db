//! Candle-based local embedder (feature: `candle-embedder`).
//!
//! Provides [`CandleEmbedder`] for running sentence-transformer models locally
//! via candle. No API server required — works on CPU, native and WASM.
//!
//! Enabled by default via the `candle-embedder` feature flag. Disable with:
//! ```toml
//! rag3weaver = { version = "0.1", default-features = false }
//! ```

use std::sync::Mutex;

use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
#[cfg(feature = "candle-embedder")]
use hf_hub::api::sync::Api;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer};

use crate::embedder::{EmbedError, Embedder};

/// Select the best available device: CUDA GPU if available, otherwise CPU.
///
/// Seuls les constructeurs `from_repo` l'utilisent, et ils sont gatés sur
/// `candle-embedder` (hf-hub). Sous `candle-wasm` le device vient de l'appelant.
#[cfg(feature = "candle-embedder")]
fn best_device() -> Device {
    #[cfg(feature = "cuda")]
    {
        match Device::new_cuda(0) {
            Ok(device) => {
                eprintln!("[candle] using CUDA device 0");
                return device;
            }
            Err(e) => {
                eprintln!("[candle] CUDA unavailable ({e}), falling back to CPU");
            }
        }
    }
    #[cfg(not(feature = "cuda"))]
    {
        eprintln!("[candle] using CPU (cuda feature disabled)");
    }
    Device::Cpu
}

/// Pre-configured sentence-transformer models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultModel {
    /// `sentence-transformers/all-MiniLM-L6-v2` — 384 dims, ~23MB.
    /// Fast and lightweight. Good default for most use cases and WASM.
    MiniLM,
    /// `BAAI/bge-base-en-v1.5` — 768 dims, ~110MB.
    /// Higher quality embeddings. Compatible with TEI server vectors.
    BgeBase,
    /// `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2` — 384 dims, ~471MB.
    /// Multilingual (50+ languages). Recommended WASM default for non-English.
    MultilingualMiniLM,
}

impl DefaultModel {
    /// HuggingFace model identifier.
    pub fn model_id(&self) -> &'static str {
        match self {
            Self::MiniLM => "sentence-transformers/all-MiniLM-L6-v2",
            Self::BgeBase => "BAAI/bge-base-en-v1.5",
            Self::MultilingualMiniLM => "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2",
        }
    }

    /// Output embedding dimension.
    pub fn dim(&self) -> usize {
        match self {
            Self::MiniLM => 384,
            Self::BgeBase => 768,
            Self::MultilingualMiniLM => 384,
        }
    }
}

impl Default for DefaultModel {
    fn default() -> Self {
        Self::BgeBase
    }
}

/// Local sentence-transformer embedder powered by candle.
///
/// Downloads the model from HuggingFace Hub on first use (cached in
/// `~/.cache/huggingface/hub/`), then runs inference locally on CPU.
///
/// ```ignore
/// use rag3weaver::candle_embedder::{CandleEmbedder, DefaultModel};
///
/// let embedder = CandleEmbedder::new(DefaultModel::MiniLM)?;
/// let vectors = embedder.embed(&["hello world".into()])?;
/// assert_eq!(vectors[0].len(), 384);
/// ```
pub struct CandleEmbedder {
    model: BertModel,
    tokenizer: Mutex<Tokenizer>,
    dim: usize,
    device: Device,
}

impl CandleEmbedder {
    /// Internal constructor from pre-loaded components.
    fn new_inner(
        config: Config,
        mut tokenizer: Tokenizer,
        vb: VarBuilder,
        device: Device,
    ) -> Result<Self, EmbedError> {
        let dim = config.hidden_size;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            ..Default::default()
        }));
        let model = BertModel::load(vb, &config)
            .map_err(|e| EmbedError::ProviderError(format!("load model: {e}")))?;
        Ok(Self {
            model,
            tokenizer: Mutex::new(tokenizer),
            dim,
            device,
        })
    }

    /// Load a pre-configured model.
    ///
    /// Downloads from HuggingFace Hub on first call, cached afterwards.
    #[cfg(feature = "candle-embedder")]
    pub fn new(default_model: DefaultModel) -> Result<Self, EmbedError> {
        Self::from_repo(default_model.model_id(), None)
    }

    /// Load a custom BERT-compatible model from HuggingFace Hub.
    ///
    /// The model must have `config.json`, `tokenizer.json`, and
    /// `model.safetensors` in the repository.
    #[cfg(feature = "candle-embedder")]
    pub fn from_repo(model_id: &str, revision: Option<&str>) -> Result<Self, EmbedError> {
        let device = best_device();

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
            .get("model.safetensors")
            .map_err(|e| EmbedError::ProviderError(format!("download model.safetensors: {e}")))?;

        let config: Config = serde_json::from_str(
            &std::fs::read_to_string(&config_path)
                .map_err(|e| EmbedError::ProviderError(format!("read config: {e}")))?,
        )
        .map_err(|e| EmbedError::ProviderError(format!("parse config: {e}")))?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &device)
                .map_err(|e| EmbedError::ProviderError(format!("load weights: {e}")))?
        };

        Self::new_inner(config, tokenizer, vb, device)
    }

    /// Load a BERT-compatible model from in-memory bytes (WASM-compatible).
    ///
    /// Accepts raw bytes for config.json, tokenizer.json, and model.safetensors.
    /// No filesystem or network access required — ideal for WASM where JS fetches
    /// the model files and passes them as byte arrays.
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

    /// Synchronous embed — called from the async trait impl and WASM FFI.
    pub fn embed_sync(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let tokenizer = self.tokenizer.lock().unwrap();
        let str_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let encodings = tokenizer
            .encode_batch(str_refs, true)
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;

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

        let token_ids = Tensor::stack(&token_ids, 0)
            .map_err(|e| EmbedError::ProviderError(format!("stack: {e}")))?;
        let attention_mask = Tensor::stack(&attention_masks, 0)
            .map_err(|e| EmbedError::ProviderError(format!("stack: {e}")))?;
        let token_type_ids = token_ids
            .zeros_like()
            .map_err(|e| EmbedError::ProviderError(format!("zeros: {e}")))?;

        // Forward pass → [batch, seq_len, hidden_size]
        let embeddings = self
            .model
            .forward(&token_ids, &token_type_ids, Some(&attention_mask))
            .map_err(|e| EmbedError::ProviderError(format!("forward: {e}")))?;

        // Mean pooling with attention mask
        let mask = attention_mask
            .unsqueeze(2)
            .and_then(|m| m.to_dtype(DTYPE))
            .map_err(|e| EmbedError::ProviderError(format!("mask: {e}")))?;

        let masked = embeddings
            .broadcast_mul(&mask)
            .map_err(|e| EmbedError::ProviderError(format!("mul: {e}")))?;
        let summed = masked
            .sum(1)
            .map_err(|e| EmbedError::ProviderError(format!("sum: {e}")))?;
        let mask_sum = mask
            .sum(1)
            .map_err(|e| EmbedError::ProviderError(format!("mask_sum: {e}")))?;
        let pooled = summed
            .broadcast_div(&mask_sum)
            .map_err(|e| EmbedError::ProviderError(format!("div: {e}")))?;

        // L2 normalize
        let norms = pooled
            .sqr()
            .and_then(|s| s.sum_keepdim(1))
            .and_then(|s| s.sqrt())
            .map_err(|e| EmbedError::ProviderError(format!("norm: {e}")))?;
        let normalized = pooled
            .broadcast_div(&norms)
            .map_err(|e| EmbedError::ProviderError(format!("normalize: {e}")))?;

        // Extract Vec<Vec<f32>>
        let batch_size = texts.len();
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
}


impl Embedder for CandleEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        self.embed_sync(texts)
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_minilm_properties() {
        let m = DefaultModel::MiniLM;
        assert_eq!(m.model_id(), "sentence-transformers/all-MiniLM-L6-v2");
        assert_eq!(m.dim(), 384);
    }

    #[test]
    fn default_model_bge_base_properties() {
        let m = DefaultModel::BgeBase;
        assert_eq!(m.model_id(), "BAAI/bge-base-en-v1.5");
        assert_eq!(m.dim(), 768);
    }

    #[test]
    fn default_model_default_is_bge_base() {
        assert_eq!(DefaultModel::default(), DefaultModel::BgeBase);
    }

    // ── Integration tests (require network + model download + candle-embedder feature) ──
    // Models are shared via OnceLock to avoid concurrent HF Hub download lock conflicts.

    #[cfg(feature = "candle-embedder")]
    fn shared_minilm() -> &'static CandleEmbedder {
        use std::sync::OnceLock;
        static E: OnceLock<CandleEmbedder> = OnceLock::new();
        E.get_or_init(|| CandleEmbedder::new(DefaultModel::MiniLM).unwrap())
    }

    #[cfg(feature = "candle-embedder")]
    fn shared_multilingual() -> &'static CandleEmbedder {
        use std::sync::OnceLock;
        static E: OnceLock<CandleEmbedder> = OnceLock::new();
        E.get_or_init(|| CandleEmbedder::new(DefaultModel::MultilingualMiniLM).unwrap())
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore] // run with: cargo test -- --ignored
    fn candle_embedder_minilm_embed() {
        let embedder = shared_minilm();
        assert_eq!(embedder.dim(), 384);

        let texts = vec!["hello world".into(), "foo bar".into()];
        let vectors = embedder.embed(&texts).unwrap();

        assert_eq!(vectors.len(), 2);
        assert_eq!(vectors[0].len(), 384);
        assert_eq!(vectors[1].len(), 384);

        let norm: f32 = vectors[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "norm={norm}, expected ~1.0");
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore]
    fn candle_embedder_bge_base_embed() {
        let embedder = CandleEmbedder::new(DefaultModel::BgeBase).unwrap();
        assert_eq!(embedder.dim(), 768);

        let texts = vec!["hello world".into()];
        let vectors = embedder.embed(&texts).unwrap();

        assert_eq!(vectors.len(), 1);
        assert_eq!(vectors[0].len(), 768);

        let norm: f32 = vectors[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "norm={norm}, expected ~1.0");
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore]
    fn candle_embedder_empty_batch() {
        let embedder = shared_minilm();
        let vectors = embedder.embed(&[]).unwrap();
        assert!(vectors.is_empty());
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore]
    fn candle_embedder_cosine_similarity() {
        let embedder = shared_minilm();
        let texts = vec![
            "Rust programming language".into(),
            "Rust systems programming".into(),
            "cooking pasta recipes".into(),
        ];
        let vecs = embedder.embed(&texts).unwrap();

        let sim_related = cosine(&vecs[0], &vecs[1]);
        let sim_unrelated = cosine(&vecs[0], &vecs[2]);

        assert!(
            sim_related > sim_unrelated,
            "related={sim_related:.4}, unrelated={sim_unrelated:.4}"
        );
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore]
    fn candle_embedder_as_trait_object() {
        let embedder: &dyn Embedder = shared_minilm();
        assert_eq!(embedder.dim(), 384);
        let result = embedder.embed(&["test".into()]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 384);
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore]
    fn candle_embedder_multilingual_minilm_embed() {
        let embedder = shared_multilingual();
        assert_eq!(embedder.dim(), 384);

        let texts = vec!["hello world".into(), "bonjour le monde".into()];
        let vectors = embedder.embed(&texts).unwrap();

        assert_eq!(vectors.len(), 2);
        assert_eq!(vectors[0].len(), 384);
        assert_eq!(vectors[1].len(), 384);

        let norm: f32 = vectors[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "norm={norm}, expected ~1.0");
    }

    #[cfg(feature = "candle-embedder")]
    #[test]
    #[ignore]
    fn candle_embedder_multilingual_cosine() {
        let embedder = shared_multilingual();
        let vecs = embedder
            .embed(&[
                "Rust programming language".into(),
                "Langage de programmation Rust".into(),
                "recetas de cocina".into(),
            ])
            .unwrap();

        let sim_translation = cosine(&vecs[0], &vecs[1]);
        let sim_unrelated = cosine(&vecs[0], &vecs[2]);

        eprintln!("[multilingual] en↔fr={sim_translation:.4}, en↔es_unrelated={sim_unrelated:.4}");
        assert!(
            sim_translation > sim_unrelated,
            "translation={sim_translation:.4} should be > unrelated={sim_unrelated:.4}"
        );
    }

    // Utilisé seulement par les tests à vrai modèle (candle-embedder) ; sous
    // candle-wasm seul il était compilé sans appelant, refusé par -D warnings.
    #[cfg(feature = "candle-embedder")]
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

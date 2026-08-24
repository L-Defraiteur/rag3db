//! paraphrase-multilingual-MiniLM-L12-v2 embedder powered by [burn](https://burn.dev) —
//! no candle, no hf-hub.
//!
//! Same model and same output as [`crate::candle_embedder::CandleEmbedder`] with
//! [`DefaultModel::MultilingualMiniLM`](crate::candle_embedder::DefaultModel::MultilingualMiniLM):
//! 384-dim, **mean pooling under the attention mask, then L2-normalised**. The
//! generated graph only exposes `last_hidden_state`, so the pooling lives here,
//! mirroring candle's.
//!
//! This is the multilingual sibling of [`crate::burn_minilm_embedder::BurnMiniLmEmbedder`]:
//! the same 384-wide BERT body (12 layers instead of 6), but a **SentencePiece Unigram
//! tokenizer with the XLM-R vocabulary** (250 002 ids). One query in French finds a
//! document written in English, and the reverse — paraphrase-mined across 50+
//! languages by knowledge distillation ([Reimers & Gurevych, 2020](https://arxiv.org/abs/2004.09813)).
//! 470 MB of weights, the embedding table alone being 384 MB of it.
//!
//! Compared to the English MiniLM:
//! - `<s>` = 0, `<pad>` = **1**, `</s>` = 2, `<unk>` = 3 (pad is not 0);
//! - single template `<s> text </s>`, every `token_type_id` 0 (the body still takes
//!   the input, `type_vocab_size` 2);
//! - `tokenizer.json` ships its own presets — `BatchLongest` padding with id 1 and a
//!   **128**-token truncation, which is sentence-transformers' `max_seq_length` for
//!   this model. The wrapper keeps that 128 so its output matches the upstream
//!   semantics and the candle oracle; [`BurnMultilingualMiniLmEmbedder::with_max_length`]
//!   lets a caller raise it up to the 512 positions the body actually has, as a
//!   documented deviation from upstream.
//!
//! Upstream has **no normalisation module** (`modules.json`: Transformer + Pooling
//! only). The vectors are L2-normalised here anyway, as the candle path does, so
//! cosine and dot product agree in the vector index.
//!
//! Runs on burn's wgpu backend — Vulkan on AMD/NVIDIA/Intel, Metal on Apple, WebGPU
//! in the browser — from one implementation.
//!
//! # Weights
//!
//! Not bundled. `model.bpk` is produced by `burn-onnx` from
//! `sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2` `onnx/model.onnx`
//! (see `generated/README.md`) and handed to [`BurnMultilingualMiniLmEmbedder::from_bytes`]
//! — `LoadStrategy::Bytes`, so in the browser JS supplies the bytes; natively, read
//! them from disk.
//!
//! # Example
//!
//! ```ignore
//! let embedder = BurnMultilingualMiniLmEmbedder::from_files("model.bpk", "tokenizer.json", Default::default())?;
//! let dense = embedder.embed(&["le chat dort sur le canapé".into()])?; // Vec<Vec<f32>>, dim 384
//! ```

use std::sync::Mutex;

use burn::prelude::*;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use crate::burn_bge_m3_embedder::BurnDevice;
use crate::embedder::{EmbedError, Embedder};
use crate::multilingual_minilm_onnx::Model as MultilingualMiniLmGraph;

/// Name reported in logs and diagnostics.
pub const MODEL_NAME: &str = "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2 (burn)";

/// XLM-R SentencePiece `<pad>`, from `tokenizer.json` (`pad_id: 1`). Not the
/// `pad_token_id: 0` of `config.json`, which is `<s>` in this vocabulary — the
/// attention mask hides padded positions either way, but the tokenizer's own
/// value is the one upstream uses.
const PAD_TOKEN_ID: u32 = 1;

const HIDDEN_SIZE: usize = 384;

/// sentence-transformers' `max_seq_length` for this model (`sentence_bert_config.json`),
/// and the truncation preset shipped in its `tokenizer.json`.
const UPSTREAM_MAX_SEQ_LEN: usize = 128;

/// Positions beyond this are undefined for the model (`max_position_embeddings`).
const MAX_SEQ_LEN: usize = 512;

/// paraphrase-multilingual-MiniLM-L12-v2 on burn. Implements [`Embedder`] only —
/// this model has no sparse head.
pub struct BurnMultilingualMiniLmEmbedder {
    graph: MultilingualMiniLmGraph,
    tokenizer: Mutex<Tokenizer>,
    device: Device,
}

impl BurnMultilingualMiniLmEmbedder {
    /// Build from burnpack bytes and a tokenizer file.
    ///
    /// Truncation is the upstream 128 tokens; see [`Self::with_max_length`].
    pub fn from_bytes(
        weights: &[u8],
        tokenizer_path: impl AsRef<std::path::Path>,
        device: BurnDevice,
    ) -> Result<Self, EmbedError> {
        let device = device.resolve();

        let mut tokenizer = Tokenizer::from_file(tokenizer_path.as_ref())
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;
        // Same values as the presets in `tokenizer.json`, set explicitly so a
        // tokenizer file stripped of its presets behaves the same.
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            pad_token: "<pad>".to_string(),
            pad_id: PAD_TOKEN_ID,
            ..Default::default()
        }));
        set_truncation(&mut tokenizer, UPSTREAM_MAX_SEQ_LEN)?;

        let graph = MultilingualMiniLmGraph::from_bytes(
            burn::tensor::Bytes::from_bytes_vec(weights.to_vec()),
            &device,
        );

        Ok(Self {
            graph,
            tokenizer: Mutex::new(tokenizer),
            device,
        })
    }

    /// Convenience: read the burnpack from disk, then [`Self::from_bytes`].
    pub fn from_files(
        weights_path: impl AsRef<std::path::Path>,
        tokenizer_path: impl AsRef<std::path::Path>,
        device: BurnDevice,
    ) -> Result<Self, EmbedError> {
        let bytes = std::fs::read(weights_path.as_ref()).map_err(|e| {
            EmbedError::ProviderError(format!(
                "read {}: {e}",
                weights_path.as_ref().display()
            ))
        })?;
        Self::from_bytes(&bytes, tokenizer_path, device)
    }

    /// Raise (or lower) the truncation length, in tokens including `<s>` and `</s>`.
    ///
    /// Upstream truncates at 128 and that is what [`Self::from_bytes`] does; the
    /// body has 512 positions, so up to 512 is valid input for the graph — but the
    /// model was trained and evaluated on 128, so embeddings of longer texts are a
    /// **deviation from upstream**, not checked against the candle oracle. Values
    /// of 0 or above 512 are rejected.
    pub fn with_max_length(self, max_length: usize) -> Result<Self, EmbedError> {
        if max_length == 0 || max_length > MAX_SEQ_LEN {
            return Err(EmbedError::ProviderError(format!(
                "max_length {max_length} out of range 1..={MAX_SEQ_LEN}"
            )));
        }
        {
            let mut tokenizer = self
                .tokenizer
                .lock()
                .map_err(|_| EmbedError::ProviderError("tokenizer mutex poisoned".into()))?;
            set_truncation(&mut tokenizer, max_length)?;
        }
        Ok(self)
    }

    /// Current truncation length, in tokens.
    pub fn max_length(&self) -> usize {
        self.tokenizer
            .lock()
            .ok()
            .and_then(|t| t.get_truncation().map(|p| p.max_length))
            .unwrap_or(UPSTREAM_MAX_SEQ_LEN)
    }

    /// Name reported in logs and diagnostics.
    pub fn name(&self) -> &'static str {
        MODEL_NAME
    }

    /// Tokenize, forward, mean-pool under the mask, L2-normalise.
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let tokenizer = self
            .tokenizer
            .lock()
            .map_err(|_| EmbedError::ProviderError("tokenizer mutex poisoned".into()))?;
        let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let encodings = tokenizer
            .encode_batch(refs, true)
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;
        drop(tokenizer);

        let batch = encodings.len();
        let seq = encodings
            .first()
            .map(|e| e.get_ids().len())
            .ok_or_else(|| EmbedError::ProviderError("empty batch".into()))?;

        let ids: Vec<i32> = encodings
            .iter()
            .flat_map(|e| e.get_ids().iter().map(|&x| x as i32))
            .collect();
        let mask: Vec<i32> = encodings
            .iter()
            .flat_map(|e| e.get_attention_mask().iter().map(|&x| x as i32))
            .collect();
        // Single-segment input: every token_type_id is 0, as candle does with
        // `zeros_like` (and as the tokenizer's template emits).
        let type_ids: Vec<i32> = vec![0; batch * seq];

        let input_ids =
            Tensor::<2, Int>::from_data(TensorData::new(ids, [batch, seq]), &self.device);
        let attention_mask =
            Tensor::<2, Int>::from_data(TensorData::new(mask, [batch, seq]), &self.device);
        let token_type_ids =
            Tensor::<2, Int>::from_data(TensorData::new(type_ids, [batch, seq]), &self.device);

        // `Model::forward(input_ids, attention_mask, token_type_ids)` → `last_hidden_state`
        // [B, S, 384]. (The generated file has one `forward` per submodule; only this
        // one is the graph's.)
        let hidden: Tensor<3> = self
            .graph
            .forward(input_ids, attention_mask.clone(), token_type_ids);

        // Mean pooling with the attention mask — same arithmetic as the candle path:
        // sum(hidden * mask) / sum(mask), then divide by the L2 norm.
        let mask_f: Tensor<3> = attention_mask.float().unsqueeze_dim(2); // [B, S, 1]
        let summed: Tensor<2> = (hidden * mask_f.clone()).sum_dim(1).squeeze_dim(1); // [B, 384]
        let counts: Tensor<2> = mask_f.sum_dim(1).squeeze_dim(1); // [B, 1]
        let pooled = summed / counts;
        let norms = pooled.clone().powf_scalar(2.0).sum_dim(1).sqrt(); // [B, 1]
        let normalized = pooled / norms;

        let data = normalized.to_data();
        let [b, dim] = [data.shape[0], data.shape[1]];
        let flat: Vec<f32> = data
            .to_vec()
            .map_err(|e| EmbedError::ProviderError(format!("dense to_vec: {e:?}")))?;
        Ok((0..b).map(|i| flat[i * dim..(i + 1) * dim].to_vec()).collect())
    }
}

/// Without truncation a long text would index positions the model has no
/// embedding for; the candle path truncates too (via the same preset).
fn set_truncation(tokenizer: &mut Tokenizer, max_length: usize) -> Result<(), EmbedError> {
    tokenizer
        .with_truncation(Some(TruncationParams {
            max_length,
            ..Default::default()
        }))
        .map_err(|e| EmbedError::ProviderError(format!("tokenizer truncation: {e}")))?;
    Ok(())
}

impl Embedder for BurnMultilingualMiniLmEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        self.embed_batch(texts)
    }

    fn dim(&self) -> usize {
        HIDDEN_SIZE
    }
}

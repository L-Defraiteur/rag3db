//! all-MiniLM-L6-v2 embedder powered by [burn](https://burn.dev) — no candle, no hf-hub.
//!
//! Same model and same output as [`crate::candle_embedder::CandleEmbedder`] with
//! [`DefaultModel::MiniLM`](crate::candle_embedder::DefaultModel::MiniLM): 384-dim,
//! **mean pooling under the attention mask, then L2-normalised**. The generated graph
//! only exposes `last_hidden_state`, so the pooling lives here, mirroring candle's.
//!
//! Runs on burn's wgpu backend — Vulkan on AMD/NVIDIA/Intel, Metal on Apple, WebGPU
//! in the browser — from one implementation. This is the model meant to become the
//! browser default: the burnpack is ~90 MB where BGE-M3's is 2.2 GB.
//!
//! # Weights
//!
//! Not bundled. `model.bpk` is produced by `burn-onnx` from
//! `sentence-transformers/all-MiniLM-L6-v2` `onnx/model.onnx` (see `generated/README.md`)
//! and handed to [`BurnMiniLmEmbedder::from_bytes`] — `LoadStrategy::Bytes`, so in the
//! browser JS supplies the bytes; natively, read them from disk.
//!
//! # Example
//!
//! ```ignore
//! let embedder = BurnMiniLmEmbedder::from_files("model.bpk", "tokenizer.json", Default::default())?;
//! let dense = embedder.embed(&["hello world".into()])?; // Vec<Vec<f32>>, dim 384
//! ```

use std::sync::Mutex;

use burn::prelude::*;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer};

use crate::burn_bge_m3_embedder::BurnDevice;
use crate::embedder::{EmbedError, Embedder};
use crate::minilm_onnx::Model as MiniLmGraph;

/// BERT WordPiece `[PAD]`, from `config.json` (`pad_token_id: 0`).
const PAD_TOKEN_ID: u32 = 0;

const HIDDEN_SIZE: usize = 384;

/// Positions beyond this are undefined for the model (`max_position_embeddings`).
const MAX_SEQ_LEN: usize = 512;

/// all-MiniLM-L6-v2 on burn. Implements [`Embedder`] only — this model has no sparse head.
pub struct BurnMiniLmEmbedder {
    graph: MiniLmGraph,
    tokenizer: Mutex<Tokenizer>,
    device: Device,
}

impl BurnMiniLmEmbedder {
    /// Build from burnpack bytes and a tokenizer file.
    pub fn from_bytes(
        weights: &[u8],
        tokenizer_path: impl AsRef<std::path::Path>,
        device: BurnDevice,
    ) -> Result<Self, EmbedError> {
        let device = device.resolve();

        let mut tokenizer = Tokenizer::from_file(tokenizer_path.as_ref())
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            pad_token: "[PAD]".to_string(),
            pad_id: PAD_TOKEN_ID,
            ..Default::default()
        }));
        // The candle path truncates too; without this a long text would index
        // positions the model has no embedding for.
        tokenizer
            .with_truncation(Some(tokenizers::TruncationParams {
                max_length: MAX_SEQ_LEN,
                ..Default::default()
            }))
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer truncation: {e}")))?;

        let graph = MiniLmGraph::from_bytes(
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
        // `zeros_like`.
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

impl Embedder for BurnMiniLmEmbedder {
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

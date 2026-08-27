//! cross-encoder/ms-marco-MiniLM-L-6-v2 reranker powered by [burn](https://burn.dev) —
//! no candle, no hf-hub.
//!
//! A cross-encoder reads the **pair** `(query, passage)` in one sequence
//! (`[CLS] query [SEP] passage [SEP]`, segment ids 0/1) and emits a single relevance
//! logit per pair. That is what makes it sharper than a bi-encoder for the final
//! ordering, and also why it can only score a bounded pool (one forward per pair).
//! The `Catalog` calls it on the fused pool when `SearchOptions.rerank` is set.
//!
//! The generated graph is the full `BertForSequenceClassification`: encoder → CLS →
//! pooler (dense 384→384 + tanh) → classifier (384→1). The model card's default
//! activation is `Identity`, so [`Reranker::rerank`] returns **raw logits**: higher
//! is more relevant, the range is unbounded (typically −11 … +11 on MS MARCO).
//! A sigmoid would turn them into a probability; it is deliberately not applied —
//! only the order is the contract, and the monotonic map would change nothing.
//!
//! Runs on burn's wgpu backend — Vulkan on AMD/NVIDIA/Intel, Metal on Apple, WebGPU
//! in the browser — from one implementation. Same family and same 90 MB footprint
//! as [`crate::burn_minilm_embedder::BurnMiniLmEmbedder`].
//!
//! # Weights
//!
//! Not bundled. `model.bpk` is produced by `burn-onnx` from
//! `cross-encoder/ms-marco-MiniLM-L-6-v2` `onnx/model.onnx` (see `generated/README.md`)
//! and handed to [`BurnMiniLmReranker::from_bytes`] — `LoadStrategy::Bytes`, so in the
//! browser JS supplies the bytes; natively, read them from disk.
//!
//! # Example
//!
//! ```ignore
//! let reranker = BurnMiniLmReranker::from_files("model.bpk", "tokenizer.json", Default::default())?;
//! let logits = reranker.rerank("how many people live in berlin", &passages)?; // one f32 per passage
//! ```

use std::sync::Mutex;

use burn::prelude::*;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy};

use crate::burn_bge_m3_embedder::BurnDevice;
use crate::embedder::EmbedError;
use crate::msmarco_minilm_onnx::Model as MsMarcoGraph;
use crate::reranker::Reranker;

/// BERT WordPiece `[PAD]`, from `config.json` (`pad_token_id: 0`).
const PAD_TOKEN_ID: u32 = 0;

/// Positions beyond this are undefined for the model (`max_position_embeddings`).
const MAX_SEQ_LEN: usize = 512;

/// Pairs per forward. Bounds the padded `[B, S, 384]` activations on the GPU:
/// at 16 × 512 tokens the largest intermediate is 16 × 12 heads × 512² f32 ≈ 200 MB.
const CHUNK: usize = 16;

const MODEL_NAME: &str = "cross-encoder/ms-marco-MiniLM-L-6-v2 (burn)";

/// ms-marco-MiniLM-L-6-v2 on burn. Implements [`Reranker`].
pub struct BurnMiniLmReranker {
    graph: MsMarcoGraph,
    tokenizer: Mutex<Tokenizer>,
    device: Device,
}

impl BurnMiniLmReranker {
    /// Build from burnpack bytes and a tokenizer file.
    pub fn from_bytes(
        weights: &[u8],
        tokenizer_path: impl AsRef<std::path::Path>,
        device: BurnDevice,
    ) -> Result<Self, EmbedError> {
        let device = device.or_role(crate::burn_device::BurnRole::Reranker).resolve();

        let mut tokenizer = Tokenizer::from_file(tokenizer_path.as_ref())
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            pad_token: "[PAD]".to_string(),
            pad_id: PAD_TOKEN_ID,
            ..Default::default()
        }));
        // Pair truncation. sentence-transformers' CrossEncoder uses the tokenizer
        // default, `LongestFirst`, which trims whichever segment is longer. We use
        // `OnlySecond`: the passage is the thing that can be long, the query must
        // survive whole or the score means nothing. The price: a query that alone
        // exceeds the window makes `encode_batch` fail ("sequence too short to
        // respect max_length"), surfaced as an `EmbedError` — the `Catalog` turns
        // that into a warning and keeps the fusion order. Queries of 500+ tokens are
        // not a reranking use case.
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: MAX_SEQ_LEN,
                strategy: TruncationStrategy::OnlySecond,
                ..Default::default()
            }))
            .map_err(|e| EmbedError::ProviderError(format!("tokenizer truncation: {e}")))?;

        let graph = MsMarcoGraph::from_bytes(
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

    /// Tokenize the pairs of one chunk, forward, flatten `[B, 1]` → `B` logits.
    fn score_chunk(&self, query: &str, passages: &[String]) -> Result<Vec<f32>, EmbedError> {
        let tokenizer = self
            .tokenizer
            .lock()
            .map_err(|_| EmbedError::ProviderError("tokenizer mutex poisoned".into()))?;
        // `(query, passage)` → `EncodeInput::Dual`: `[CLS] q [SEP] p [SEP]`, type ids
        // 0 over the query segment and 1 over the passage — as at training time.
        let pairs: Vec<(&str, &str)> = passages.iter().map(|p| (query, p.as_str())).collect();
        let encodings = tokenizer
            .encode_batch(pairs, true)
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
        let type_ids: Vec<i32> = encodings
            .iter()
            .flat_map(|e| e.get_type_ids().iter().map(|&x| x as i32))
            .collect();

        let input_ids =
            Tensor::<2, Int>::from_data(TensorData::new(ids, [batch, seq]), &self.device);
        let attention_mask =
            Tensor::<2, Int>::from_data(TensorData::new(mask, [batch, seq]), &self.device);
        let token_type_ids =
            Tensor::<2, Int>::from_data(TensorData::new(type_ids, [batch, seq]), &self.device);

        // `Model::forward(input_ids, attention_mask, token_type_ids)` → logits `[B, 1]`.
        // (The generated file has one `forward` per submodule; only this one is the
        // graph's.)
        let logits: Tensor<2> = self.graph.forward(input_ids, attention_mask, token_type_ids);

        let data = logits.to_data();
        // Rank is 2 by type; only the extents can surprise.
        let (rows, cols) = (data.shape[0], data.shape[1]);
        if (rows, cols) != (batch, 1) {
            return Err(EmbedError::ProviderError(format!(
                "logits: expected shape [{batch}, 1], got [{rows}, {cols}]"
            )));
        }
        data.to_vec()
            .map_err(|e| EmbedError::ProviderError(format!("logits to_vec: {e:?}")))
    }
}

impl Reranker for BurnMiniLmReranker {
    /// One raw logit per passage, in passage order. Higher = more relevant.
    fn rerank(&self, query: &str, passages: &[String]) -> Result<Vec<f32>, EmbedError> {
        let mut scores = Vec::with_capacity(passages.len());
        for chunk in passages.chunks(CHUNK) {
            scores.extend(self.score_chunk(query, chunk)?);
        }
        Ok(scores)
    }

    fn name(&self) -> &str {
        MODEL_NAME
    }
}

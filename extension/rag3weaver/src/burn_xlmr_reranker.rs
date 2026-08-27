//! Multilingual cross-encoder rerankers on the XLM-RoBERTa architecture, powered by
//! [burn](https://burn.dev) — no candle, no hf-hub.
//!
//! Two models share this file, the same tokenizer family and the same graph shape:
//!
//! | struct | model | layers / hidden | weights |
//! |---|---|---|---|
//! | [`BurnMMiniLmReranker`] | `cross-encoder/mmarco-mMiniLMv2-L12-H384-v1` | 12 / 384 | 470 MB |
//! | [`BurnBgeRerankerV2M3`] | `BAAI/bge-reranker-v2-m3` | 24 / 1024 | 2.2 GB |
//!
//! Both are `XLMRobertaForSequenceClassification` with one label and an identity
//! activation: encoder → `<s>` hidden state → dense + tanh → out_proj (→ 1), so
//! [`Reranker::rerank`] returns **raw logits** (higher = more relevant, unbounded).
//! The mMiniLM one is the small, fast multilingual reranker (trained on
//! `unicamp-dl/mmarco`, MS MARCO machine-translated into 14 languages, French
//! included); the BGE one is the strong-but-heavy option from the same family as
//! [`crate::burn_bge_m3_embedder::BurnBgeM3Embedder`].
//!
//! Compared to the English [`crate::burn_reranker::BurnMiniLmReranker`] (BERT):
//! - SentencePiece Unigram tokenizer, 250 002 ids, `<pad>` = **1** (not 0);
//! - pair template `<s> query </s></s> passage </s>`, every type id 0 — the graph
//!   takes **no `token_type_ids`**, only `(input_ids, attention_mask)`;
//! - `tokenizer.json` ships neither a truncation nor a padding preset: both are set
//!   here (`OnlySecond` at 512, `BatchLongest` with id 1).
//!
//! Everything that is not the graph type lives in a private generic
//! [`XlmrCrossEncoder`]: tokenizer setup, pair encoding into `(ids, mask)` tensors,
//! chunking and the `[B, 1]` → `Vec<f32>` flattening. Adding a third XLM-R
//! cross-encoder is one `impl XlmrGraph` plus one public newtype.
//!
//! # Weights
//!
//! Not bundled. Each `model.bpk` is produced by `burn-onnx` from the model's ONNX
//! export (see `generated/README.md`) and handed to `from_bytes` — `LoadStrategy::Bytes`,
//! so in the browser JS supplies the bytes; natively, `from_files` reads them from disk.
//!
//! # Example
//!
//! ```ignore
//! let reranker = BurnMMiniLmReranker::from_files("model.bpk", "tokenizer.json", Default::default())?;
//! let logits = reranker.rerank("combien de personnes vivent à berlin", &passages)?;
//! ```

use std::sync::Mutex;

use burn::prelude::*;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy};

use crate::bge_reranker_v2_m3_onnx::Model as BgeRerankerGraph;
use crate::burn_bge_m3_embedder::BurnDevice;
use crate::embedder::EmbedError;
use crate::mmarco_mminilm_onnx::Model as MMarcoGraph;
use crate::reranker::Reranker;

/// XLM-RoBERTa `<pad>`, from `config.json` (`pad_token_id: 1`). `<s>` = 0, `</s>` = 2.
const PAD_TOKEN_ID: u32 = 1;

/// Sequence budget per pair. mMiniLM has 514 positions (512 usable, the first two
/// are the padding offset of RoBERTa). BGE-reranker-v2-m3 has 8 194 and would
/// accept much longer passages, but attention is quadratic in the length and a
/// reranking passage is a chunk, not a document: 512 for both.
const MAX_SEQ_LEN: usize = 512;

/// Pairs per forward. Bounds the padded `[B, S, H]` activations on the GPU: at
/// 16 × 512 tokens the largest intermediate is 16 × heads × 512² f32 — 200 MB for
/// 12 heads (mMiniLM), 270 MB for 16 (BGE).
const CHUNK: usize = 16;

/// What the two generated graphs have in common, seen from the wrapper.
trait XlmrGraph: Send + Sync + Sized {
    /// `Model::from_bytes` of the generated module.
    fn load(bytes: burn::tensor::Bytes, device: &Device) -> Self;
    /// `Model::forward(input_ids, attention_mask)` → logits `[B, 1]`.
    fn logits(&self, input_ids: Tensor<2, Int>, attention_mask: Tensor<2, Int>) -> Tensor<2>;
}

impl XlmrGraph for MMarcoGraph {
    fn load(bytes: burn::tensor::Bytes, device: &Device) -> Self {
        MMarcoGraph::from_bytes(bytes, device)
    }
    fn logits(&self, input_ids: Tensor<2, Int>, attention_mask: Tensor<2, Int>) -> Tensor<2> {
        // (The generated file has one `forward` per submodule; only this one is the
        // graph's.)
        self.forward(input_ids, attention_mask)
    }
}

impl XlmrGraph for BgeRerankerGraph {
    fn load(bytes: burn::tensor::Bytes, device: &Device) -> Self {
        BgeRerankerGraph::from_bytes(bytes, device)
    }
    fn logits(&self, input_ids: Tensor<2, Int>, attention_mask: Tensor<2, Int>) -> Tensor<2> {
        self.forward(input_ids, attention_mask)
    }
}

/// The shared machinery: one XLM-R tokenizer, one graph, one device.
struct XlmrCrossEncoder<G: XlmrGraph> {
    graph: G,
    tokenizer: Mutex<Tokenizer>,
    device: Device,
}

/// Load `tokenizer.json` and give it the pair presets it does not ship with.
fn xlmr_pair_tokenizer(path: &std::path::Path) -> Result<Tokenizer, EmbedError> {
    let mut tokenizer = Tokenizer::from_file(path)
        .map_err(|e| EmbedError::ProviderError(format!("tokenizer: {e}")))?;
    tokenizer.with_padding(Some(PaddingParams {
        strategy: PaddingStrategy::BatchLongest,
        pad_token: "<pad>".to_string(),
        pad_id: PAD_TOKEN_ID,
        ..Default::default()
    }));
    // Same choice as `BurnMiniLmReranker`: the passage is the segment that can be
    // long, the query must survive whole. A query that alone exceeds the window
    // makes `encode_batch` fail, surfaced as an `EmbedError` — the `Catalog` turns
    // that into a warning and keeps the fusion order.
    tokenizer
        .with_truncation(Some(TruncationParams {
            max_length: MAX_SEQ_LEN,
            strategy: TruncationStrategy::OnlySecond,
            ..Default::default()
        }))
        .map_err(|e| EmbedError::ProviderError(format!("tokenizer truncation: {e}")))?;
    Ok(tokenizer)
}

/// Encode `(query, passage)` pairs into padded `input_ids` / `attention_mask`
/// tensors. Returns the batch size with them. The `<s> q </s></s> p </s>` template
/// comes from the tokenizer's post-processor; there are no type ids to build.
fn encode_pairs(
    tokenizer: &Mutex<Tokenizer>,
    device: &Device,
    query: &str,
    passages: &[String],
) -> Result<(Tensor<2, Int>, Tensor<2, Int>, usize), EmbedError> {
    let tokenizer = tokenizer
        .lock()
        .map_err(|_| EmbedError::ProviderError("tokenizer mutex poisoned".into()))?;
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

    let input_ids = Tensor::<2, Int>::from_data(TensorData::new(ids, [batch, seq]), device);
    let attention_mask = Tensor::<2, Int>::from_data(TensorData::new(mask, [batch, seq]), device);
    Ok((input_ids, attention_mask, batch))
}

/// `[B, 1]` logits → `B` values, with the shape check.
fn flatten_logits(logits: Tensor<2>, batch: usize) -> Result<Vec<f32>, EmbedError> {
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

impl<G: XlmrGraph> XlmrCrossEncoder<G> {
    fn from_bytes(
        weights: &[u8],
        tokenizer_path: &std::path::Path,
        device: BurnDevice,
    ) -> Result<Self, EmbedError> {
        let device = device.or_role(crate::burn_device::BurnRole::Reranker).resolve();
        let tokenizer = xlmr_pair_tokenizer(tokenizer_path)?;
        let graph = G::load(burn::tensor::Bytes::from_bytes_vec(weights.to_vec()), &device);
        Ok(Self {
            graph,
            tokenizer: Mutex::new(tokenizer),
            device,
        })
    }

    fn from_files(
        weights_path: &std::path::Path,
        tokenizer_path: &std::path::Path,
        device: BurnDevice,
    ) -> Result<Self, EmbedError> {
        let bytes = std::fs::read(weights_path).map_err(|e| {
            EmbedError::ProviderError(format!("read {}: {e}", weights_path.display()))
        })?;
        Self::from_bytes(&bytes, tokenizer_path, device)
    }

    /// Tokenize the pairs of one chunk, forward, flatten `[B, 1]` → `B` logits.
    fn score_chunk(&self, query: &str, passages: &[String]) -> Result<Vec<f32>, EmbedError> {
        let (input_ids, attention_mask, batch) =
            encode_pairs(&self.tokenizer, &self.device, query, passages)?;
        flatten_logits(self.graph.logits(input_ids, attention_mask), batch)
    }

    /// One raw logit per passage, in passage order.
    fn rerank(&self, query: &str, passages: &[String]) -> Result<Vec<f32>, EmbedError> {
        let mut scores = Vec::with_capacity(passages.len());
        for chunk in passages.chunks(CHUNK) {
            scores.extend(self.score_chunk(query, chunk)?);
        }
        Ok(scores)
    }
}

/// Public surface of one model: newtype over [`XlmrCrossEncoder`], constructors,
/// [`Reranker`] with the model name.
macro_rules! xlmr_reranker {
    ($(#[$doc:meta])* $name:ident, $graph:ty, $model_name:expr) => {
        $(#[$doc])*
        pub struct $name(XlmrCrossEncoder<$graph>);

        impl $name {
            /// Build from burnpack bytes and a tokenizer file.
            pub fn from_bytes(
                weights: &[u8],
                tokenizer_path: impl AsRef<std::path::Path>,
                device: BurnDevice,
            ) -> Result<Self, EmbedError> {
                XlmrCrossEncoder::from_bytes(weights, tokenizer_path.as_ref(), device).map(Self)
            }

            /// Convenience: read the burnpack from disk, then [`Self::from_bytes`].
            pub fn from_files(
                weights_path: impl AsRef<std::path::Path>,
                tokenizer_path: impl AsRef<std::path::Path>,
                device: BurnDevice,
            ) -> Result<Self, EmbedError> {
                XlmrCrossEncoder::from_files(weights_path.as_ref(), tokenizer_path.as_ref(), device)
                    .map(Self)
            }
        }

        impl Reranker for $name {
            /// One raw logit per passage, in passage order. Higher = more relevant.
            fn rerank(&self, query: &str, passages: &[String]) -> Result<Vec<f32>, EmbedError> {
                self.0.rerank(query, passages)
            }

            fn name(&self) -> &str {
                $model_name
            }
        }
    };
}

xlmr_reranker!(
    /// cross-encoder/mmarco-mMiniLMv2-L12-H384-v1 on burn — 12 layers, hidden 384,
    /// multilingual (mMARCO, 14 languages). Implements [`Reranker`].
    BurnMMiniLmReranker,
    MMarcoGraph,
    "cross-encoder/mmarco-mMiniLMv2-L12-H384-v1 (burn)"
);

xlmr_reranker!(
    /// BAAI/bge-reranker-v2-m3 on burn — 24 layers, hidden 1024, multilingual,
    /// 2.2 GB of weights. Implements [`Reranker`]. Sequences are capped at 512
    /// tokens per pair like the others (the model itself goes to 8 192).
    BurnBgeRerankerV2M3,
    BgeRerankerGraph,
    "BAAI/bge-reranker-v2-m3 (burn)"
);

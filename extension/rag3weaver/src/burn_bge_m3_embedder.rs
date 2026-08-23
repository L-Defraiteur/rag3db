//! BGE-M3 embedder powered by [burn](https://burn.dev) — no candle, no CUDA, no hf-hub.
//!
//! Same model and same outputs as [`crate::bge_m3_embedder::BgeM3Embedder`], but running
//! on burn's wgpu backend. That means Vulkan on AMD/NVIDIA/Intel, Metal on Apple, and
//! WebGPU in the browser — from a single implementation, with no vendor-specific build.
//!
//! Dense = CLS pooling + L2 normalize (1024 dims), produced directly by the graph.
//! Sparse = learned linear layer (1024→1) + ReLU, scattered by token id (max).
//!
//! # Weights
//!
//! The backbone weights are **not** bundled: they are a 2.2 GB burnpack file, published at
//! <https://huggingface.co/Lucie666/bge-m3-burnpack>. Fetch it once (plain anonymous
//! HTTPS, no token) and hand the bytes to [`BurnBgeM3Embedder::from_bytes`].
//!
//! The sparse head is only 1025 f32 values, so it *is* bundled
//! (`generated/bge_m3_sparse_linear.bin`, 4 KB).
//!
//! # Parity
//!
//! Checked against the candle implementation: dense cosine 1.00000000 (max |Δ| 3.5e-07),
//! sparse identical token ids with weights within 6e-06 relative. See
//! `docs/23-aout-2026-20h33/02-spike-burn-vulkan-amd.md` (relatif au crate).
//!
//! # Example
//!
//! ```ignore
//! let bytes = std::fs::read("model.bpk")?;
//! let embedder = BurnBgeM3Embedder::from_bytes(&bytes, "tokenizer.json", Default::default())?;
//! let (dense, sparse) = embedder.embed_dual(&["hello world".into()])?;
//! ```

use std::collections::HashMap;
use std::sync::Mutex;

use burn::nn::{Linear, LinearConfig};
use burn::prelude::*;
use burn::tensor::activation::relu;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer};

use crate::bge_m3_onnx::Model as BgeM3Graph;
use crate::embedder::{DualEmbedder, EmbedError, Embedder, SparseEmbedder};
use crate::sparse_index::SparseVector;

/// XLM-RoBERTa special token ids, excluded from the sparse vector.
/// Kept in sync with [`crate::bge_m3_embedder`].
const CLS_TOKEN_ID: u32 = 0; // <s>
const PAD_TOKEN_ID: u32 = 1; // <pad>
const EOS_TOKEN_ID: u32 = 2; // </s>
const UNK_TOKEN_ID: u32 = 3; // <unk>

const HIDDEN_SIZE: usize = 1024;

/// Learned sparse head, extracted from BAAI's `sparse_linear.pt` (originally f16,
/// widened to f32 — which is what candle does at load time too).
/// Layout: 1024 little-endian f32 weights, then 1 f32 bias.
const SPARSE_LINEAR: &[u8] = include_bytes!("../generated/bge_m3_sparse_linear.bin");

/// Which GPU burn should run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BurnDevice {
    /// Best available device (discrete GPU if present).
    #[default]
    Default,
    /// Nth discrete GPU — useful for sharding across several cards.
    DiscreteGpu(usize),
    /// Integrated GPU.
    IntegratedGpu(usize),
    /// CPU fallback. Correct but slow; handy for reproducible reference output.
    Cpu,
}

impl BurnDevice {
    fn resolve(self) -> Device {
        match self {
            BurnDevice::Default => Device::default(),
            BurnDevice::DiscreteGpu(i) => Device::wgpu(DeviceKind::DiscreteGpu(i)),
            BurnDevice::IntegratedGpu(i) => Device::wgpu(DeviceKind::IntegratedGpu(i)),
            BurnDevice::Cpu => Device::wgpu(DeviceKind::Cpu),
        }
    }
}

/// The learned sparse projection: `Linear(1024 → 1)` followed by ReLU.
#[derive(Module, Debug)]
struct SparseHead {
    linear: Linear,
}

impl SparseHead {
    /// Build from the bundled weights. PyTorch stores `Linear` as `[out, in]`,
    /// burn expects `[in, out]` — the bundled file is already in burn's layout.
    fn from_embedded(device: &Device) -> Result<Self, EmbedError> {
        let expected = (HIDDEN_SIZE + 1) * 4;
        if SPARSE_LINEAR.len() != expected {
            return Err(EmbedError::ProviderError(format!(
                "sparse head: expected {expected} bytes, got {}",
                SPARSE_LINEAR.len()
            )));
        }

        let floats: Vec<f32> = SPARSE_LINEAR
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let (w, b) = floats.split_at(HIDDEN_SIZE);

        let mut head = SparseHead {
            linear: LinearConfig::new(HIDDEN_SIZE, 1).init(device),
        };
        head.linear.weight = burn::module::Param::from_tensor(Tensor::<2>::from_data(
            TensorData::new(w.to_vec(), [HIDDEN_SIZE, 1]),
            device,
        ));
        head.linear.bias = Some(burn::module::Param::from_tensor(Tensor::<1>::from_data(
            TensorData::new(b.to_vec(), [1]),
            device,
        )));
        Ok(head)
    }
}

/// BGE-M3 embedder running on burn.
///
/// Implements [`Embedder`], [`SparseEmbedder`] and [`DualEmbedder`], so it is a drop-in
/// replacement for the candle-based one wherever those traits are used.
pub struct BurnBgeM3Embedder {
    graph: BgeM3Graph,
    sparse_head: SparseHead,
    tokenizer: Mutex<Tokenizer>,
    device: Device,
}

impl BurnBgeM3Embedder {
    /// Build from burnpack bytes and a tokenizer file.
    ///
    /// `weights` is the content of `model.bpk` (see module docs for where to get it).
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
            pad_token: "<pad>".to_string(),
            pad_id: PAD_TOKEN_ID,
            ..Default::default()
        }));

        let graph = BgeM3Graph::from_bytes(
            burn::tensor::Bytes::from_bytes_vec(weights.to_vec()),
            &device,
        );
        let sparse_head = SparseHead::from_embedded(&device)?;

        Ok(Self {
            graph,
            sparse_head,
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

    /// Tokenize + forward. Returns `(token_embeddings, sentence_embedding, ids per row)`.
    fn forward_pass(
        &self,
        texts: &[String],
    ) -> Result<(Tensor<3>, Tensor<2>, Vec<Vec<u32>>), EmbedError> {
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

        let ids_per_row: Vec<Vec<u32>> = encodings.iter().map(|e| e.get_ids().to_vec()).collect();

        let ids: Vec<i32> = encodings
            .iter()
            .flat_map(|e| e.get_ids().iter().map(|&x| x as i32))
            .collect();
        let mask: Vec<i32> = encodings
            .iter()
            .flat_map(|e| e.get_attention_mask().iter().map(|&x| x as i32))
            .collect();

        let input_ids =
            Tensor::<2, Int>::from_data(TensorData::new(ids, [batch, seq]), &self.device);
        let attention_mask =
            Tensor::<2, Int>::from_data(TensorData::new(mask, [batch, seq]), &self.device);

        let (token_embeddings, sentence_embedding) =
            self.graph.forward(input_ids, attention_mask);
        Ok((token_embeddings, sentence_embedding, ids_per_row))
    }

    /// `sentence_embedding` is already CLS-pooled and L2-normalized by the graph.
    fn extract_dense(sentence_embedding: Tensor<2>) -> Result<Vec<Vec<f32>>, EmbedError> {
        let data = sentence_embedding.to_data();
        let [batch, dim] = [data.shape[0], data.shape[1]];
        let flat: Vec<f32> = data
            .to_vec()
            .map_err(|e| EmbedError::ProviderError(format!("dense to_vec: {e:?}")))?;
        Ok((0..batch)
            .map(|i| flat[i * dim..(i + 1) * dim].to_vec())
            .collect())
    }

    /// `W_lex(hidden) → ReLU → scatter by token id (max)`, mirroring
    /// [`crate::bge_m3_embedder`]'s `extract_sparse`.
    fn extract_sparse(
        &self,
        token_embeddings: Tensor<3>,
        ids_per_row: &[Vec<u32>],
    ) -> Result<Vec<SparseVector>, EmbedError> {
        let scores = relu(
            self.sparse_head
                .linear
                .forward(token_embeddings)
                .squeeze_dim::<2>(2),
        );
        let data = scores.to_data();
        let seq = data.shape[1];
        let flat: Vec<f32> = data
            .to_vec()
            .map_err(|e| EmbedError::ProviderError(format!("sparse to_vec: {e:?}")))?;

        let mut out = Vec::with_capacity(ids_per_row.len());
        for (i, ids) in ids_per_row.iter().enumerate() {
            let mut per_token: HashMap<u32, f32> = HashMap::new();
            for (pos, &token_id) in ids.iter().enumerate() {
                if matches!(
                    token_id,
                    CLS_TOKEN_ID | PAD_TOKEN_ID | EOS_TOKEN_ID | UNK_TOKEN_ID
                ) || pos >= seq
                {
                    continue;
                }
                let w = flat[i * seq + pos];
                if w > 0.0 {
                    let e = per_token.entry(token_id).or_insert(0.0);
                    if w > *e {
                        *e = w;
                    }
                }
            }
            let mut entries: Vec<(u32, f32)> = per_token.into_iter().collect();
            entries.sort_by_key(|(id, _)| *id);
            out.push(SparseVector::new(
                entries.iter().map(|(id, _)| *id).collect(),
                entries.iter().map(|(_, w)| *w).collect(),
            ));
        }
        Ok(out)
    }
}

impl Embedder for BurnBgeM3Embedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        let (_, sentence, _) = self.forward_pass(texts)?;
        Self::extract_dense(sentence)
    }

    fn dim(&self) -> usize {
        HIDDEN_SIZE
    }
}

impl SparseEmbedder for BurnBgeM3Embedder {
    fn embed_sparse(&self, texts: &[String]) -> Result<Vec<SparseVector>, EmbedError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        let (tokens, _, ids) = self.forward_pass(texts)?;
        self.extract_sparse(tokens, &ids)
    }
}

impl DualEmbedder for BurnBgeM3Embedder {
    /// Single forward pass for both representations — the whole point of BGE-M3.
    fn embed_dual(
        &self,
        texts: &[String],
    ) -> Result<(Vec<Vec<f32>>, Vec<SparseVector>), EmbedError> {
        if texts.is_empty() {
            return Ok((vec![], vec![]));
        }
        let (tokens, sentence, ids) = self.forward_pass(texts)?;
        let dense = Self::extract_dense(sentence)?;
        let sparse = self.extract_sparse(tokens, &ids)?;
        Ok((dense, sparse))
    }

    fn dim(&self) -> usize {
        HIDDEN_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_head_weights_are_bundled_and_well_formed() {
        assert_eq!(SPARSE_LINEAR.len(), (HIDDEN_SIZE + 1) * 4, "4100 bytes expected");
        let floats: Vec<f32> = SPARSE_LINEAR
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(floats.len(), HIDDEN_SIZE + 1);
        assert!(floats.iter().all(|f| f.is_finite()), "no NaN/inf in the head");
        // Bias from BAAI's sparse_linear.pt, widened from f16.
        let bias = floats[HIDDEN_SIZE];
        assert!(
            (bias - 0.045_196_533).abs() < 1e-6,
            "unexpected bias {bias} — wrong file?"
        );
    }

    #[test]
    fn special_ids_match_the_candle_implementation() {
        assert_eq!((CLS_TOKEN_ID, PAD_TOKEN_ID, EOS_TOKEN_ID, UNK_TOKEN_ID), (0, 1, 2, 3));
    }
}

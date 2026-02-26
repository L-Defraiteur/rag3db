//! Modified BERT model that returns attention weights alongside hidden states.
//!
//! Replicates the candle-transformers BERT architecture, but each layer's
//! self-attention also outputs attention probabilities. Used by [`Bm42Embedder`]
//! to extract CLS→token attention weights for BM42-style sparse vectors.
//!
//! Compatible with standard BERT safetensors (same weight names).

use candle_core::{DType, Device, Result, Tensor};
use candle_nn::{embedding, layer_norm, linear, Embedding, LayerNorm, Linear, Module, VarBuilder};
pub use candle_transformers::models::bert::{Config, HiddenAct, DTYPE};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn get_extended_attention_mask(mask: &Tensor, dtype: DType) -> Result<Tensor> {
    let mask = mask.unsqueeze(1)?.unsqueeze(1)?.to_dtype(dtype)?;
    (mask.ones_like()? - &mask)?
        .broadcast_mul(&Tensor::try_from(f32::MIN)?.to_device(mask.device())?)
}

fn activation(act: HiddenAct, xs: &Tensor) -> Result<Tensor> {
    match act {
        HiddenAct::Gelu => xs.gelu_erf(),
        HiddenAct::GeluApproximate => xs.gelu(),
        HiddenAct::Relu => xs.relu(),
    }
}

fn ln(size: usize, eps: f64, vb: VarBuilder) -> Result<LayerNorm> {
    layer_norm(size, candle_nn::LayerNormConfig { eps, ..Default::default() }, vb)
}

// ── Embeddings ──────────────────────────────────────────────────────────────

struct Embeddings {
    word: Embedding,
    position: Embedding,
    token_type: Embedding,
    layer_norm: LayerNorm,
}

impl Embeddings {
    fn load(vb: VarBuilder, c: &Config) -> Result<Self> {
        Ok(Self {
            word: embedding(c.vocab_size, c.hidden_size, vb.pp("word_embeddings"))?,
            position: embedding(c.max_position_embeddings, c.hidden_size, vb.pp("position_embeddings"))?,
            token_type: embedding(c.type_vocab_size, c.hidden_size, vb.pp("token_type_embeddings"))?,
            layer_norm: ln(c.hidden_size, c.layer_norm_eps, vb.pp("LayerNorm"))?,
        })
    }

    fn forward(&self, input_ids: &Tensor, token_type_ids: &Tensor) -> Result<Tensor> {
        let (_b, seq_len) = input_ids.dims2()?;
        let pos_ids = Tensor::arange(0u32, seq_len as u32, input_ids.device())?;
        let emb = (self.word.forward(input_ids)?
            + self.token_type.forward(token_type_ids)?)?
            .broadcast_add(&self.position.forward(&pos_ids)?)?;
        self.layer_norm.forward(&emb)
    }
}

// ── Self-Attention (returns attention_probs) ────────────────────────────────

struct SelfAttention {
    query: Linear,
    key: Linear,
    value: Linear,
    num_heads: usize,
    head_size: usize,
}

impl SelfAttention {
    fn load(vb: VarBuilder, c: &Config) -> Result<Self> {
        let head_size = c.hidden_size / c.num_attention_heads;
        let all_head_size = c.num_attention_heads * head_size;
        Ok(Self {
            query: linear(c.hidden_size, all_head_size, vb.pp("query"))?,
            key: linear(c.hidden_size, all_head_size, vb.pp("key"))?,
            value: linear(c.hidden_size, all_head_size, vb.pp("value"))?,
            num_heads: c.num_attention_heads,
            head_size,
        })
    }

    fn reshape(&self, xs: &Tensor) -> Result<Tensor> {
        let mut shape = xs.dims().to_vec();
        shape.pop();
        shape.push(self.num_heads);
        shape.push(self.head_size);
        xs.reshape(shape)?.transpose(1, 2)?.contiguous()
    }

    /// Returns `(context_layer, attention_probs)`.
    /// attention_probs shape: `[batch, num_heads, seq_len, seq_len]`.
    fn forward(&self, hidden: &Tensor, mask: &Tensor) -> Result<(Tensor, Tensor)> {
        let q = self.reshape(&self.query.forward(hidden)?)?;
        let k = self.reshape(&self.key.forward(hidden)?)?;
        let v = self.reshape(&self.value.forward(hidden)?)?;

        let scores = q.matmul(&k.t()?)?;
        let scores = (scores / (self.head_size as f64).sqrt())?.broadcast_add(mask)?;
        let attn_probs = candle_nn::ops::softmax(&scores, candle_core::D::Minus1)?;

        let ctx = attn_probs.matmul(&v)?.transpose(1, 2)?.contiguous()?;
        let ctx = ctx.flatten_from(candle_core::D::Minus2)?;
        Ok((ctx, attn_probs))
    }
}

// ── Attention block ─────────────────────────────────────────────────────────

struct Attention {
    self_attn: SelfAttention,
    dense: Linear,
    layer_norm: LayerNorm,
}

impl Attention {
    fn load(vb: VarBuilder, c: &Config) -> Result<Self> {
        Ok(Self {
            self_attn: SelfAttention::load(vb.pp("self"), c)?,
            dense: linear(c.hidden_size, c.hidden_size, vb.pp("output").pp("dense"))?,
            layer_norm: ln(c.hidden_size, c.layer_norm_eps, vb.pp("output").pp("LayerNorm"))?,
        })
    }

    fn forward(&self, hidden: &Tensor, mask: &Tensor) -> Result<(Tensor, Tensor)> {
        let (ctx, attn_probs) = self.self_attn.forward(hidden, mask)?;
        let out = self.layer_norm.forward(&(self.dense.forward(&ctx)? + hidden)?)?;
        Ok((out, attn_probs))
    }
}

// ── Feed-Forward ────────────────────────────────────────────────────────────

struct FeedForward {
    dense_in: Linear,
    dense_out: Linear,
    layer_norm: LayerNorm,
    act: HiddenAct,
}

impl FeedForward {
    fn load(vb: VarBuilder, c: &Config) -> Result<Self> {
        Ok(Self {
            dense_in: linear(c.hidden_size, c.intermediate_size, vb.pp("intermediate").pp("dense"))?,
            dense_out: linear(c.intermediate_size, c.hidden_size, vb.pp("output").pp("dense"))?,
            layer_norm: ln(c.hidden_size, c.layer_norm_eps, vb.pp("output").pp("LayerNorm"))?,
            act: c.hidden_act,
        })
    }

    fn forward(&self, hidden: &Tensor) -> Result<Tensor> {
        let h = activation(self.act, &self.dense_in.forward(hidden)?)?;
        self.layer_norm.forward(&(self.dense_out.forward(&h)? + hidden)?)
    }
}

// ── Layer ───────────────────────────────────────────────────────────────────

struct Layer {
    attention: Attention,
    ff: FeedForward,
}

impl Layer {
    fn load(vb: VarBuilder, c: &Config) -> Result<Self> {
        Ok(Self {
            attention: Attention::load(vb.pp("attention"), c)?,
            ff: FeedForward::load(vb, c)?,
        })
    }

    fn forward(&self, hidden: &Tensor, mask: &Tensor) -> Result<(Tensor, Tensor)> {
        let (attn_out, attn_probs) = self.attention.forward(hidden, mask)?;
        let out = self.ff.forward(&attn_out)?;
        Ok((out, attn_probs))
    }
}

// ── Encoder ─────────────────────────────────────────────────────────────────

struct Encoder {
    layers: Vec<Layer>,
}

impl Encoder {
    fn load(vb: VarBuilder, c: &Config) -> Result<Self> {
        let layers = (0..c.num_hidden_layers)
            .map(|i| Layer::load(vb.pp(format!("layer.{i}")), c))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { layers })
    }

    /// Returns `(hidden_states, last_layer_attention_probs)`.
    fn forward(&self, hidden: &Tensor, mask: &Tensor) -> Result<(Tensor, Tensor)> {
        let mut h = hidden.clone();
        let mut last_attn = Tensor::zeros(&[1], DType::F32, hidden.device())?;
        for layer in &self.layers {
            let (out, attn) = layer.forward(&h, mask)?;
            h = out;
            last_attn = attn;
        }
        Ok((h, last_attn))
    }
}

// ── Bm42Model ───────────────────────────────────────────────────────────────

/// BERT model that returns both hidden states and last-layer attention weights.
///
/// Loads from the same safetensors as `BertModel` (identical weight names).
pub struct Bm42Model {
    embeddings: Embeddings,
    encoder: Encoder,
    pub device: Device,
}

impl Bm42Model {
    pub fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        // Try bare paths first, then model_type prefix (same as candle BertModel)
        let (embeddings, encoder) = match (
            Embeddings::load(vb.pp("embeddings"), config),
            Encoder::load(vb.pp("encoder"), config),
        ) {
            (Ok(e), Ok(enc)) => (e, enc),
            (Err(err), _) | (_, Err(err)) => {
                if let Some(ref model_type) = config.model_type {
                    let e = Embeddings::load(vb.pp(format!("{model_type}.embeddings")), config)?;
                    let enc = Encoder::load(vb.pp(format!("{model_type}.encoder")), config)?;
                    (e, enc)
                } else {
                    return Err(err);
                }
            }
        };
        Ok(Self {
            embeddings,
            encoder,
            device: vb.device().clone(),
        })
    }

    /// Forward pass returning `(hidden_states, last_attention_probs)`.
    ///
    /// - `hidden_states`: `[batch, seq_len, hidden_size]`
    /// - `last_attention_probs`: `[batch, num_heads, seq_len, seq_len]`
    pub fn forward(
        &self,
        input_ids: &Tensor,
        token_type_ids: &Tensor,
        attention_mask: Option<&Tensor>,
    ) -> Result<(Tensor, Tensor)> {
        let emb = self.embeddings.forward(input_ids, token_type_ids)?;
        let mask = match attention_mask {
            Some(m) => m.clone(),
            None => input_ids.ones_like()?,
        };
        let ext_mask = get_extended_attention_mask(&mask, DType::F32)?;
        self.encoder.forward(&emb, &ext_mask)
    }
}

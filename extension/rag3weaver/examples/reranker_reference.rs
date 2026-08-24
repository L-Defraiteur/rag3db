//! Reference logits for the reranker parity check: candle (CPU) running
//! `cross-encoder/ms-marco-MiniLM-L-6-v2` on a fixed set of (query, passage) pairs,
//! dumped as JSON for `examples/burn_reranker_vs_candle.rs`.
//!
//! The HF checkpoint is a `BertForSequenceClassification`: `BertModel` (with pooler)
//! + classifier `Linear(384 → 1)`. candle's `BertModel` stops at the encoder, so the
//! pooler (`bert.pooler.dense`, tanh) and the classifier are loaded here from the
//! same safetensors and applied on the `[CLS]` hidden state — exactly what the ONNX
//! export, hence the burn graph, does.
//!
//! ```bash
//! cargo run --release --example reranker_reference --features candle-embedder -- \
//!     /tmp/reranker_reference.json
//! ```

use candle_core::{DType, Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::api::sync::Api;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy};

const MODEL_ID: &str = "cross-encoder/ms-marco-MiniLM-L-6-v2";

/// Must stay identical to `examples/burn_reranker_vs_candle.rs`.
const PAIRS: &[(&str, &str)] = &[
    ("how many people live in berlin", "Berlin has a population of 3.5 million registered inhabitants"),
    ("how many people live in berlin", "New York City is famous for the Metropolitan Museum of Art"),
    ("how many people live in berlin", "The Berlin Wall fell in 1989"),
    ("what does the borrow checker do", "Rust's borrow checker enforces ownership and lifetimes at compile time"),
    ("what does the borrow checker do", "Whisk the eggs with sugar, then fold in the flour and bake for twenty minutes"),
];

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/reranker_reference.json".to_string());

    let device = Device::Cpu;
    let repo = Api::new()?.model(MODEL_ID.to_string());
    let config_path = repo.get("config.json")?;
    let tokenizer_path = repo.get("tokenizer.json")?;
    let weights_path = repo.get("model.safetensors")?;

    let config: Config = serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;

    // Same pair tokenisation as `BurnMiniLmReranker`: batch-longest padding, the
    // passage (second segment) is the one truncated at 512.
    let mut tokenizer = Tokenizer::from_file(&tokenizer_path)?;
    tokenizer.with_padding(Some(PaddingParams {
        strategy: PaddingStrategy::BatchLongest,
        pad_token: "[PAD]".to_string(),
        pad_id: config.pad_token_id as u32,
        ..Default::default()
    }));
    tokenizer.with_truncation(Some(TruncationParams {
        max_length: config.max_position_embeddings,
        strategy: TruncationStrategy::OnlySecond,
        ..Default::default()
    }))?;

    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &device)? };
    let bert = BertModel::load(vb.pp("bert"), &config)?;
    let pooler = candle_nn::linear(config.hidden_size, config.hidden_size, vb.pp("bert.pooler.dense"))?;
    let classifier = candle_nn::linear(config.hidden_size, 1, vb.pp("classifier"))?;

    let pairs: Vec<(String, String)> = PAIRS.iter().map(|(q, p)| (q.to_string(), p.to_string())).collect();
    let encodings = tokenizer.encode_batch(pairs, true)?;

    let stack = |f: fn(&tokenizers::Encoding) -> &[u32]| -> Result<Tensor, candle_core::Error> {
        let rows: Vec<Tensor> = encodings
            .iter()
            .map(|e| Tensor::new(f(e), &device))
            .collect::<Result<_, _>>()?;
        Tensor::stack(&rows, 0)
    };
    let input_ids = stack(|e| e.get_ids())?;
    let attention_mask = stack(|e| e.get_attention_mask())?;
    let token_type_ids = stack(|e| e.get_type_ids())?;

    // [B, S, 384] → CLS [B, 384] → pooler dense + tanh → classifier → [B]
    let hidden = bert.forward(&input_ids, &token_type_ids, Some(&attention_mask))?;
    let cls = hidden.narrow(1, 0, 1)?.squeeze(1)?;
    let pooled = pooler.forward(&cls)?.tanh()?;
    let logits = classifier.forward(&pooled)?.squeeze(1)?.to_dtype(DType::F32)?.to_vec1::<f32>()?;

    let items: Vec<String> = logits.iter().map(|x| format!("{x:.9}")).collect();
    std::fs::write(&out, format!("{{\"logits\":[{}]}}\n", items.join(",")))?;
    for ((q, p), l) in PAIRS.iter().zip(&logits) {
        eprintln!("{l:>9.4}  {q:?} / {p:?}");
    }
    eprintln!("wrote {} logits to {out}", logits.len());
    Ok(())
}

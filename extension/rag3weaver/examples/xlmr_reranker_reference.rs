//! Reference logits for the multilingual reranker parity check: candle (CPU) running
//! an XLM-RoBERTa cross-encoder on a fixed set of (query, passage) pairs, dumped as
//! JSON for `examples/burn_xlmr_reranker_vs_candle.rs`.
//!
//! Two models, chosen by the second argument:
//! - `mmarco` (default): `cross-encoder/mmarco-mMiniLMv2-L12-H384-v1` (12 × 384);
//! - `bge`: `BAAI/bge-reranker-v2-m3` (24 × 1024, 2.2 GB of safetensors).
//!
//! Both HF checkpoints are `XLMRobertaForSequenceClassification`. candle-transformers
//! 0.8 ships that struct, but its classification head applies `GeluPytorchTanh`
//! where the reference implementation (`RobertaClassificationHead`) applies
//! `torch.tanh` — the logits would be wrong by a few tenths. So the backbone is
//! `XLMRobertaModel` (prefix `roberta`) and the head is loaded here from the same
//! safetensors: `classifier.dense.{weight,bias}` (H → H) + tanh +
//! `classifier.out_proj.{weight,bias}` (H → 1), applied on the `<s>` hidden state —
//! exactly what the ONNX export, hence the burn graph, does.
//!
//! ```bash
//! cargo run --release --example xlmr_reranker_reference --features candle-embedder -- \
//!     /tmp/xlmr_reranker_reference.json [mmarco|bge]
//! ```

use candle_core::{DType, Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::xlm_roberta::{Config, XLMRobertaModel};
use hf_hub::api::sync::Api;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy};

/// Same query/passage set for both models. Must stay identical to
/// `examples/burn_xlmr_reranker_vs_candle.rs`. Grouped by query: the burn side
/// scores one query against its passages and stitches back in this order.
const PAIRS: &[(&str, &str)] = &[
    ("how many people live in berlin", "Berlin has a population of 3.5 million registered inhabitants"),
    ("how many people live in berlin", "New York City is famous for the Metropolitan Museum of Art"),
    ("how many people live in berlin", "The Berlin Wall fell in 1989"),
    ("combien de personnes vivent à berlin", "Berlin compte 3,5 millions d'habitants"),
    ("combien de personnes vivent à berlin", "New York est célèbre pour le Metropolitan Museum of Art"),
    ("combien de personnes vivent à berlin", "Le mur de Berlin est tombé en 1989"),
    // Cross-language: French query, English answer.
    ("combien de personnes vivent à berlin", "Berlin has a population of 3.5 million registered inhabitants"),
];

/// Same cap as `src/burn_xlmr_reranker.rs` (BGE's config says 8 194).
const MAX_SEQ_LEN: usize = 512;

fn model_id(choice: &str) -> &'static str {
    match choice {
        "mmarco" => "cross-encoder/mmarco-mMiniLMv2-L12-H384-v1",
        "bge" => "BAAI/bge-reranker-v2-m3",
        other => panic!("unknown model {other:?}: expected `mmarco` or `bge`"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "/tmp/xlmr_reranker_reference.json".to_string());
    let choice = args.next().unwrap_or_else(|| "mmarco".to_string());
    let model_id = model_id(&choice);

    let device = Device::Cpu;
    let repo = Api::new()?.model(model_id.to_string());
    let config_path = repo.get("config.json")?;
    let tokenizer_path = repo.get("tokenizer.json")?;
    let weights_path = repo.get("model.safetensors")?;

    let config: Config = serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;
    let hidden = config.hidden_size;

    // Same pair tokenisation as the burn wrapper: `<pad>` = 1, batch-longest
    // padding, the passage (second segment) is the one truncated.
    let mut tokenizer = Tokenizer::from_file(&tokenizer_path)?;
    tokenizer.with_padding(Some(PaddingParams {
        strategy: PaddingStrategy::BatchLongest,
        pad_token: "<pad>".to_string(),
        pad_id: config.pad_token_id,
        ..Default::default()
    }));
    tokenizer.with_truncation(Some(TruncationParams {
        max_length: MAX_SEQ_LEN,
        strategy: TruncationStrategy::OnlySecond,
        ..Default::default()
    }))?;

    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)? };
    let roberta = XLMRobertaModel::new(&config, vb.pp("roberta"))?;
    let dense = candle_nn::linear(hidden, hidden, vb.pp("classifier.dense"))?;
    let out_proj = candle_nn::linear(hidden, 1, vb.pp("classifier.out_proj"))?;

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
    // XLM-R has a single segment type; the tokenizer emits zeros and the burn graph
    // takes none at all.
    let token_type_ids = input_ids.zeros_like()?;

    // [B, S, H] → <s> [B, H] → dense + tanh → out_proj → [B]
    let hidden_states = roberta.forward(&input_ids, &attention_mask, &token_type_ids, None, None, None)?;
    let cls = hidden_states.narrow(1, 0, 1)?.squeeze(1)?;
    let pooled = dense.forward(&cls)?.tanh()?;
    let logits = out_proj.forward(&pooled)?.squeeze(1)?.to_dtype(DType::F32)?.to_vec1::<f32>()?;

    let items: Vec<String> = logits.iter().map(|x| format!("{x:.9}")).collect();
    std::fs::write(&out, format!("{{\"model\":\"{choice}\",\"logits\":[{}]}}\n", items.join(",")))?;
    for ((q, p), l) in PAIRS.iter().zip(&logits) {
        eprintln!("{l:>9.4}  {q:?} / {p:?}");
    }
    eprintln!("[{model_id}] wrote {} logits to {out}", logits.len());
    Ok(())
}

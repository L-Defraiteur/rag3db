//! **The tokens of a chunk** (ticket of embarquement, 20 November 2026: 72,5 % of rag3weaver's chunks passed the
//! 512-token window of granite-278m, so the end of each was out of reach of the vector).
//!
//! A chunk is bounded by the base's smallest embedder, in two stages:
//! - **in bytes, first**: window × margin ÷ the model's declared tokens per character. Cheap, and right for most
//!   code;
//! - **in tokens, then**: the tokenizer the embedder names counts a chunk that could pass its window, and the chunker
//!   narrows it. A ratio alone does not hold the window: the dense tables of the engine's C++ headers reach 0,8 token
//!   per character (`docs/everything-declarative/20-11-2026/01-la-borne-des-morceaux-mesuree.md`).
//!
//! The tokenizer is a capability the embedder declares, like its weights: a build or a machine without it refuses to
//! chunk for that model, it never falls back to the bytes alone. Two builds would otherwise cut the same text in two
//! ways under the same version of the analysis.

use serde::{Deserialize, Serialize};

/// **What a chunk must keep under, in tokens**: the model whose tokenizer counts, and its window. Part of the chunking
/// configuration, so of the version of the analysis.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenBound {
    pub model: String,
    pub window: usize,
}

/// **What a known embedder declares for its chunks**: its window in tokens, and its tokens per character measured on
/// code (the median of the stage in bytes; the stage in tokens holds the rest).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmbedderChunking {
    pub window: usize,
    pub tokens_per_char: f64,
}

/// The share of the window a chunk takes, by default: room for what the measure does not see.
pub const DEFAULT_MARGIN: f64 = 0.9;

/// **The declaration of each known embedder.** granite: 512 tokens, 0,45 token per character (embarquement's measure
/// on 8 843 chunks of rag3weaver, 20 November 2026). bge-m3: 8 192 tokens, whose bound constrains nothing at the
/// chunk sizes of code.
pub fn declared(model: &str) -> Option<EmbedderChunking> {
    match model {
        "granite-278m" | "granite-107m" => Some(EmbedderChunking { window: 512, tokens_per_char: 0.45 }),
        "bge-m3" => Some(EmbedderChunking { window: 8192, tokens_per_char: 0.45 }),
        _ => None,
    }
}

/// **The bound of a chunk for these embedders**: the smallest of them, in bytes and in tokens. None when none is
/// known: the chunking keeps `max_size` alone.
pub fn bound_for<'a>(models: impl IntoIterator<Item = &'a str>, margin: f64) -> Option<(usize, TokenBound)> {
    models
        .into_iter()
        .filter_map(|m| declared(m).map(|d| (m, d)))
        .map(|(m, d)| {
            let octets = ((d.window as f64) * margin / d.tokens_per_char).floor() as usize;
            (octets.max(1), TokenBound { model: m.to_string(), window: d.window })
        })
        .min_by_key(|(octets, b)| (*octets, b.window))
}

/// **A counter of tokens**, special tokens included: what the embedder's own encoding of the text would count.
pub type TokenCounter = std::sync::Arc<dyn Fn(&str) -> usize + Send + Sync>;

/// **The counter of a model**, from the tokenizer its embedder names (`RAG3WEAVER_<MODEL>_TOKENIZER`, otherwise
/// `~/.cache/rag3weaver/<model>/tokenizer.json`). Loaded once per process. A refusal names the file and the variable.
#[cfg(feature = "token-count")]
pub fn counter(model: &str) -> Result<TokenCounter, String> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    static CHARGES: OnceLock<Mutex<HashMap<String, TokenCounter>>> = OnceLock::new();
    let charges = CHARGES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(c) = charges.lock().unwrap_or_else(|p| p.into_inner()).get(model) {
        return Ok(c.clone());
    }
    let prefixe = format!("RAG3WEAVER_{}", model.to_uppercase().replace('-', "_"));
    let chemin = crate::model_source::local_artifact(model, &prefixe, "TOKENIZER", "tokenizer.json")
        .map_err(|e| format!("le découpage pour {model} compte ses jetons avec son tokenizer : {e}"))?;
    let tokenizer = tokenizers::Tokenizer::from_file(&chemin).map_err(|e| format!("tokenizer de {model} ({}) : {e}", chemin.display()))?;
    let tokenizer = Arc::new(tokenizer);
    let c: TokenCounter = Arc::new(move |texte: &str| tokenizer.encode(texte, true).map(|e| e.len()).unwrap_or(usize::MAX));
    charges.lock().unwrap_or_else(|p| p.into_inner()).insert(model.to_string(), c.clone());
    Ok(c)
}

/// Without the `token-count` feature, no model's chunks can be bounded in tokens: the refusal says which build has it.
#[cfg(not(feature = "token-count"))]
pub fn counter(model: &str) -> Result<TokenCounter, String> {
    Err(format!(
        "le découpage pour {model} compte ses jetons, et ce bâti n'a pas le compteur : il faut la feature token-count \
         (comprise dans burn-embedder, candle-embedder et bge-m3)"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **No chunk passes granite's window, counted by its tokenizer, on the engine's C++ headers** — the worst case of
    /// the measure (`docs/everything-declarative/20-11-2026/01-la-borne-des-morceaux-mesuree.md`: dense tables up to
    /// 0,8 token per character, where the bound in bytes alone left 773 packets of 31 620 over 512). The tokenizer is
    /// a capability: without it the witness refuses, naming the file, it does not skip.
    #[cfg(feature = "token-count")]
    #[test]
    fn no_chunk_for_granite_passes_its_window_on_the_engine_headers() {
        use crate::chunker::{Chunker, ChunkerConfig};
        use crate::config::ChunkStrategy;
        let compte = counter("granite-278m").unwrap_or_else(|e| panic!("the witness counts with granite's tokenizer: {e}"));
        let (octets, borne) = bound_for(["granite-278m"], DEFAULT_MARGIN).expect("granite is declared");
        let config = ChunkerConfig {
            strategy: ChunkStrategy::Lines,
            max_lines: 30,
            max_size: 1500,
            overlap_lines: 5,
            overlap: 0,
            max_chunk_size: Some(octets),
            token_bound: Some(borne.clone()),
        };
        let decoupeur = Chunker::with_counter(config, compte.clone());
        let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/include");
        let mut fichiers = Vec::new();
        let mut a_voir = vec![racine.clone()];
        while let Some(d) = a_voir.pop() {
            for e in std::fs::read_dir(&d).unwrap_or_else(|e| panic!("{}: {e}", d.display())).flatten() {
                let p = e.path();
                if p.is_dir() {
                    a_voir.push(p);
                } else if p.extension().is_some_and(|x| x == "h") {
                    fichiers.push(p);
                }
            }
        }
        assert!(fichiers.len() > 500, "the engine's headers are there: {} under {}", fichiers.len(), racine.display());
        let (mut morceaux, mut trop) = (0usize, Vec::new());
        for f in &fichiers {
            let texte = std::fs::read_to_string(f).unwrap_or_default();
            for k in decoupeur.chunk(&texte) {
                morceaux += 1;
                let n = compte(&k.text);
                if n > borne.window {
                    trop.push((f.strip_prefix(&racine).unwrap_or(f).display().to_string(), k.start_line, n));
                }
            }
        }
        assert!(morceaux > 2_000, "the headers give thousands of chunks: {morceaux}");
        assert!(trop.is_empty(), "{} chunks of {morceaux} over {} tokens, first ones: {:?}", trop.len(), borne.window, &trop[..trop.len().min(5)]);
    }

    #[test]
    fn the_smallest_embedder_makes_the_bound() {
        let (octets, borne) = bound_for(["bge-m3", "granite-278m"], DEFAULT_MARGIN).expect("known models");
        assert_eq!(borne, TokenBound { model: "granite-278m".into(), window: 512 });
        assert_eq!(octets, 1024, "512 × 0,9 ÷ 0,45");
        let (octets, borne) = bound_for(["bge-m3"], DEFAULT_MARGIN).expect("bge-m3");
        assert_eq!(borne.window, 8192);
        assert!(octets > 1500 * 3, "bge-m3 alone constrains nothing at the sizes of code: {octets}");
        assert_eq!(bound_for(["un-modele-inconnu"], DEFAULT_MARGIN), None);
    }
}

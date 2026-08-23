//! Example: embed texts via TEI (Text Embeddings Inference) using reqwest.
//!
//! Requires a running TEI server, e.g.:
//!   docker run -p 8081:80 ghcr.io/huggingface/text-embeddings-inference:turing-1.7 \
//!     --model-id BAAI/bge-base-en-v1.5
//!
//! Run: cargo run --example tei_reqwest

use rag3weaver::{EmbedError, Embedder};
use serde::{Deserialize, Serialize};

struct TeiEmbedder {
    client: reqwest::blocking::Client,
    url: String,
    model: String,
    dim: usize,
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    input: &'a [String],
    model: &'a str,
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

#[derive(Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

impl Embedder for TeiEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request = EmbedRequest {
            input: texts,
            model: &self.model,
        };

        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .map_err(|e| EmbedError::ProviderError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(EmbedError::ProviderError(format!("{status}: {body}")));
        }

        let embed_response: EmbedResponse = response
            .json()
            .map_err(|e| EmbedError::ProviderError(e.to_string()))?;

        let vectors: Vec<Vec<f32>> = embed_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect();

        for v in &vectors {
            if v.len() != self.dim {
                return Err(EmbedError::DimensionMismatch {
                    expected: self.dim,
                    got: v.len(),
                });
            }
        }

        Ok(vectors)
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

fn main() {
    let embedder = TeiEmbedder {
        client: reqwest::blocking::Client::new(),
        url: "http://localhost:8081/v1/embeddings".into(),
        model: "BAAI/bge-base-en-v1.5".into(),
        dim: 768,
    };

    let texts = vec![
        "Rust is a systems programming language".into(),
        "Graph databases store relationships natively".into(),
        "Full-text search uses inverted indexes".into(),
    ];

    println!("Embedding {} texts via TEI (reqwest)...", texts.len());

    match embedder.embed(&texts) {
        Ok(vectors) => {
            println!(
                "Success! Got {} vectors of dim {}",
                vectors.len(),
                embedder.dim()
            );
            for (i, v) in vectors.iter().enumerate() {
                println!("  [{}] first 5: {:?}", i, &v[..5.min(v.len())]);
            }
            println!(
                "\nCosine similarity [0]·[1]: {:.4}",
                cosine_similarity(&vectors[0], &vectors[1])
            );
            println!(
                "Cosine similarity [0]·[2]: {:.4}",
                cosine_similarity(&vectors[0], &vectors[2])
            );
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

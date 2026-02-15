//! Example: embed texts via TEI using the async-openai SDK.
//!
//! TEI exposes an OpenAI-compatible `/v1/embeddings` endpoint, so we can
//! use async-openai pointed at localhost. Same pattern works for OpenAI,
//! Azure, Ollama, vLLM, or any compatible provider.
//!
//! Run: cargo run --example tei_openai

use async_openai::{
    config::OpenAIConfig,
    types::embeddings::{CreateEmbeddingRequestArgs, EmbeddingInput},
    Client,
};
use async_trait::async_trait;
use rag3weaver::{EmbedError, Embedder};

struct OpenAIEmbedder {
    client: Client<OpenAIConfig>,
    model: String,
    dim: usize,
}

impl OpenAIEmbedder {
    fn new(base_url: &str, model: &str, dim: usize) -> Self {
        let config = OpenAIConfig::new()
            .with_api_base(base_url)
            .with_api_key("unused"); // TEI doesn't require an API key
        Self {
            client: Client::with_config(config),
            model: model.into(),
            dim,
        }
    }
}

#[async_trait]
impl Embedder for OpenAIEmbedder {
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input(EmbeddingInput::StringArray(texts.to_vec()))
            .build()
            .map_err(|e| EmbedError::ProviderError(e.to_string()))?;

        let response = self
            .client
            .embeddings()
            .create(request)
            .await
            .map_err(|e| EmbedError::ProviderError(e.to_string()))?;

        let vectors: Vec<Vec<f32>> = response
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

#[tokio::main]
async fn main() {
    let embedder = OpenAIEmbedder::new(
        "http://localhost:8081/v1",
        "BAAI/bge-base-en-v1.5",
        768,
    );

    let texts = vec![
        "Rust is a systems programming language".into(),
        "Graph databases store relationships natively".into(),
        "Full-text search uses inverted indexes".into(),
    ];

    println!("Embedding {} texts via TEI (async-openai)...", texts.len());

    match embedder.embed(&texts).await {
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

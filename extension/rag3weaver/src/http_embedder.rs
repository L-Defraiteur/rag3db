//! Configurable dense embeddings over an OpenAI-compatible HTTP contract.
//! Independent of LLM selection, UI, MCP, and any domain.
use crate::{EmbedError, Embedder};
use serde_json::{json, Value};

pub struct HttpEmbedder {
    endpoint: String,
    model: String,
    dimensions: usize,
    key: Option<String>,
    client: ureq::Agent,
}
impl HttpEmbedder {
    /// `base_url` ends before `/embeddings`, usually `/v1`.
    pub fn new(
        base_url: &str,
        model: &str,
        dimensions: usize,
        key_env: Option<&str>,
    ) -> Result<Self, String> {
        if !(base_url.starts_with("http://") || base_url.starts_with("https://"))
            || model.trim().is_empty()
            || dimensions == 0
        {
            return Err(
                "embeddings require an HTTP(S) base URL, model and positive dimensions".into(),
            );
        }
        let key = key_env
            .map(|name| {
                std::env::var(name)
                    .ok()
                    .filter(|v| !v.trim().is_empty())
                    .ok_or_else(|| {
                        format!("embedding credential environment variable {name} is missing")
                    })
            })
            .transpose()?;
        Ok(Self {
            endpoint: format!("{}/embeddings", base_url.trim_end_matches('/')),
            model: model.into(),
            dimensions,
            key,
            client: ureq::Agent::new_with_config(
                ureq::Agent::config_builder()
                    .http_status_as_error(false)
                    .timeout_global(Some(std::time::Duration::from_secs(120)))
                    .build(),
            ),
        })
    }
    fn decode(&self, value: Value, expected: usize) -> Result<Vec<Vec<f32>>, String> {
        let data = value["data"]
            .as_array()
            .ok_or("embedding response missing data array")?;
        if data.len() != expected {
            return Err(format!(
                "embedding count mismatch: expected {expected}, got {}",
                data.len()
            ));
        }
        let mut out = vec![None; expected];
        for row in data {
            let index = row["index"]
                .as_u64()
                .and_then(|i| usize::try_from(i).ok())
                .filter(|i| *i < expected)
                .ok_or("invalid embedding index")?;
            if out[index].is_some() {
                return Err("duplicate embedding index".into());
            }
            let vector: Vec<f32> = serde_json::from_value(row["embedding"].clone())
                .map_err(|_| "embedding must be a numeric array")?;
            if vector.len() != self.dimensions || vector.iter().any(|v| !v.is_finite()) {
                return Err(format!(
                    "embedding requires {} finite components",
                    self.dimensions
                ));
            }
            out[index] = Some(vector);
        }
        out.into_iter()
            .map(|v| v.ok_or("missing embedding index".into()))
            .collect()
    }
}
impl Embedder for HttpEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        let request = json!({"model":self.model,"input":texts,"encoding_format":"float"});
        let mut req = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json");
        if let Some(key) = &self.key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        let mut response = req.send(request.to_string().as_bytes()).map_err(|_| {
            EmbedError::ProviderError(
                "embedding HTTP transport failed (check endpoint/connectivity)".into(),
            )
        })?;
        if !response.status().is_success() {
            return Err(EmbedError::ProviderError(format!(
                "embedding provider HTTP {}",
                response.status()
            )));
        }
        let body = response
            .body_mut()
            .with_config()
            .limit(64 * 1024 * 1024)
            .read_to_string()
            .map_err(|_| {
                EmbedError::ProviderError("invalid or oversized embedding response".into())
            })?;
        let value = serde_json::from_str(&body).map_err(|_| {
            EmbedError::ProviderError("embedding provider returned invalid JSON".into())
        })?;
        self.decode(value, texts.len())
            .map_err(EmbedError::ProviderError)
    }
    fn dim(&self) -> usize {
        self.dimensions
    }
    fn name(&self) -> &str {
        &self.model
    }
    fn distant(&self) -> bool {
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compatible_http_batch_preserves_order_and_checks_contract() {
        use std::io::Read;
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let address = format!("http://{}/v1", server.server_addr());
        let worker = std::thread::spawn(move || {
            let mut request = server
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap()
                .unwrap();
            assert_eq!(request.url(), "/v1/embeddings");
            assert!(request.headers().iter().any(
                |h| h.field.equiv("Authorization") && h.value.as_str() == "Bearer fixture-key"
            ));
            let mut body = String::new();
            Read::read_to_string(request.as_reader(), &mut body).unwrap();
            let body: Value = serde_json::from_str(&body).unwrap();
            assert_eq!(
                body,
                json!({"model":"fixture", "input":["one","two"], "encoding_format":"float"})
            );
            request
                .respond(tiny_http::Response::from_string(
                    json!({"data":[{"index":1,"embedding":[3,4]},{"index":0,"embedding":[1,2]}]})
                        .to_string(),
                ))
                .unwrap();
        });
        let mut embedder = HttpEmbedder::new(&address, "fixture", 2, None).unwrap();
        embedder.key = Some("fixture-key".into());
        let actual = embedder.embed(&["one".into(), "two".into()]).unwrap();
        worker.join().unwrap();
        assert_eq!(actual, vec![vec![1., 2.], vec![3., 4.]]);
    }
    #[test]
    fn indexed_response_is_reordered_and_invalid_batches_fail() {
        let e = HttpEmbedder::new("http://localhost/v1", "model", 2, None).unwrap();
        assert_eq!(
            e.decode(
                json!({"data":[{"index":1,"embedding":[3,4]},{"index":0,"embedding":[1,2]}]}),
                2
            )
            .unwrap(),
            vec![vec![1., 2.], vec![3., 4.]]
        );
        for bad in [
            json!({"data":[]}),
            json!({"data":[{"index":0,"embedding":[1]}]}),
            json!({"data":[{"index":4,"embedding":[1,2]}]}),
            json!({"data":[{"index":0,"embedding":[1e100,2]}]}),
        ] {
            assert!(e.decode(bad, 1).is_err());
        }
        assert!(e
            .decode(
                json!({"data":[{"index":0,"embedding":[1,2]},{"index":0,"embedding":[3,4]}]}),
                2
            )
            .is_err());
    }
}

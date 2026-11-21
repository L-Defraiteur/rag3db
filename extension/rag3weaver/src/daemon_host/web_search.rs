//! **web_search, the teammate's search** (the teammate's base, 20 November 2026, page 02 of rag3weaver-archi, § 7): a
//! query answered by Gemini with Google Search grounding, on Vertex, by a native `generateContent` request with
//! `tools: [{"googleSearch": {}}]` — not through the chat's client, which the step does not hold. The reply gives
//! the model's answer, the queries it ran, and the sources: title, address, the passages of the answer they support,
//! and their provenance (chunk index, number of supports, best confidence). The sources are bounded by
//! `teammate.web_search.max_sources`, and the cut is said. A model that is not Gemini is refused with the reason,
//! before anything is sent.

use crate::backend::WebSearchPolicy;
use serde_json::{json, Value};

/// The characters of one source's snippet at most.
const SNIPPET_CHARS: usize = 600;

/// The model the search runs on: the policy's, else the host's Vertex model.
fn model(policy: &WebSearchPolicy) -> String {
    policy
        .model
        .clone()
        .filter(|m| !m.trim().is_empty())
        .or_else(|| std::env::var("VERTEX_MODEL").ok().filter(|m| !m.trim().is_empty()))
        .unwrap_or_else(|| crate::regime::MODELE_VERTEX_PAR_DEFAUT.to_string())
}

/// The address of `generateContent` for this model, and the token to send (none for a declared endpoint without
/// Vertex credentials: a proxy or a test).
fn target(policy: &WebSearchPolicy, model: &str) -> Result<(String, Option<String>), String> {
    let short = model.rsplit('/').next().unwrap_or(model);
    let location = std::env::var("GOOGLE_CLOUD_LOCATION").unwrap_or_else(|_| "global".into());
    let project = std::env::var("GOOGLE_CLOUD_PROJECT").ok().filter(|p| !p.trim().is_empty());
    let host = match policy.endpoint.as_deref().filter(|e| !e.trim().is_empty()) {
        Some(e) => e.trim_end_matches('/').to_string(),
        None if location == "global" => "https://aiplatform.googleapis.com".to_string(),
        None => format!("https://{location}-aiplatform.googleapis.com"),
    };
    let declared = policy.endpoint.as_deref().is_some_and(|e| !e.trim().is_empty());
    let project = match (project, declared) {
        (Some(p), _) => p,
        (None, true) => "local".to_string(),
        (None, false) => return Err("web_search runs on Gemini on Vertex, and GOOGLE_CLOUD_PROJECT is not set on the host".into()),
    };
    let url = format!("{host}/v1/projects/{project}/locations/{location}/publishers/google/models/{short}:generateContent");
    Ok((url, token(declared)?))
}

#[cfg(feature = "openai-llm")]
fn token(declared: bool) -> Result<Option<String>, String> {
    match crate::gcp_auth::TokenSource::from_env().and_then(|s| s.token()) {
        Ok(t) => Ok(Some(t)),
        Err(_) if declared => Ok(None),
        Err(e) => Err(format!("web_search runs on Gemini on Vertex, and the host has no Vertex credentials: {e}")),
    }
}

#[cfg(not(feature = "openai-llm"))]
fn token(declared: bool) -> Result<Option<String>, String> {
    if declared {
        return Ok(None);
    }
    Err("web_search runs on Gemini on Vertex, and this build has no Vertex client (feature openai-llm)".into())
}

/// **web_search {query}**: Gemini's answer grounded by Google Search, its queries and its sources.
pub fn web_search(policy: &WebSearchPolicy, args: &Value) -> Result<Value, String> {
    let query = args["query"].as_str().map(str::trim).filter(|q| !q.is_empty()).ok_or("web_search: query is required")?;
    let model = model(policy);
    if !model.to_ascii_lowercase().contains("gemini") {
        return Err(format!(
            "web_search: the grounding by Google Search is Gemini's, on Vertex; « {model} » is not a Gemini model (teammate.web_search.model, or VERTEX_MODEL)"
        ));
    }
    let (url, token) = target(policy, &model)?;
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(std::time::Duration::from_secs(policy.timeout_s.max(1))))
            .build(),
    );
    let body = json!({ "contents": [{ "role": "user", "parts": [{ "text": query }] }], "tools": [{ "googleSearch": {} }] });
    let mut req = agent.post(&url).header("content-type", "application/json");
    if let Some(t) = &token {
        req = req.header("authorization", &format!("Bearer {t}"));
    }
    let mut resp = req.send(body.to_string().as_bytes()).map_err(|e| format!("web_search: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp.body_mut().with_config().limit(16 * 1024 * 1024).read_to_string().map_err(|e| format!("web_search: reading the reply: {e}"))?;
    if status >= 400 {
        let said: String = text.chars().take(600).collect();
        return Err(format!("web_search: Vertex answered HTTP {status} for {model}: {said}"));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("web_search: the reply is not JSON ({e})"))?;
    Ok(read_reply(&v, policy.max_sources.max(1), &model))
}

/// The reply of `generateContent`, read: the answer, the queries, the sources bounded at `max`.
fn read_reply(v: &Value, max: usize, model: &str) -> Value {
    let candidate = &v["candidates"][0];
    let answer: String = candidate["content"]["parts"].as_array().map(|p| p.iter().filter_map(|x| x["text"].as_str()).collect::<Vec<_>>().join("")).unwrap_or_default();
    let meta = &candidate["groundingMetadata"];
    let chunks = meta["groundingChunks"].as_array().cloned().unwrap_or_default();
    let supports = meta["groundingSupports"].as_array().cloned().unwrap_or_default();
    let mut sources = Vec::new();
    for (i, chunk) in chunks.iter().enumerate().take(max) {
        let web = &chunk["web"];
        let mut passages = Vec::new();
        let mut best: f64 = 0.0;
        for s in &supports {
            let Some(idx) = s["groundingChunkIndices"].as_array() else { continue };
            if let Some(pos) = idx.iter().position(|n| n.as_u64() == Some(i as u64)) {
                if let Some(t) = s["segment"]["text"].as_str() {
                    passages.push(t.to_string());
                }
                best = best.max(s["confidenceScores"][pos].as_f64().unwrap_or(0.0));
            }
        }
        let mut snippet = passages.join(" … ");
        if snippet.chars().count() > SNIPPET_CHARS {
            snippet = snippet.chars().take(SNIPPET_CHARS).collect::<String>() + "…";
        }
        sources.push(json!({
            "title": web["title"].as_str().unwrap_or(""),
            "url": web["uri"].as_str().unwrap_or(""),
            "snippet": snippet,
            "provenance": { "grounding_chunk": i, "supports": passages.len(), "best_confidence": best },
        }));
    }
    let total = chunks.len();
    let mut out = json!({
        "ok": true,
        "model": model,
        "answer": answer,
        "queries": meta["webSearchQueries"].clone(),
        "sources": sources,
        "total_sources": total,
        "cut_sources": total.saturating_sub(max),
    });
    if out["queries"].is_null() {
        out["queries"] = json!([]);
    }
    if total == 0 {
        out["note"] = json!("the model answered without grounding sources: its answer is not backed by a search");
    }
    out
}

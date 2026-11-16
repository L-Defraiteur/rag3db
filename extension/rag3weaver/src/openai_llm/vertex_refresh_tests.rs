//! Token refresh contracts on the real Vertex transport and declared connector.
//! Prepared only: no execution or compilation verdict yet.

use super::*;
use crate::llm::StringSink;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Default)]
struct FakeTokenSource {
    acquisitions: AtomicUsize,
    invalidations: AtomicUsize,
}

impl FakeTokenSource {
    fn token(&self) -> &'static str {
        self.acquisitions.fetch_add(1, Ordering::SeqCst);
        if self.invalidations.load(Ordering::SeqCst) == 0 {
            "fixture-token-1"
        } else {
            "fixture-token-2"
        }
    }

    fn invalidate(&self) {
        self.invalidations.fetch_add(1, Ordering::SeqCst);
    }
}

struct RefreshServer {
    server: Arc<tiny_http::Server>,
    stopped: Arc<AtomicBool>,
    authorizations: Arc<Mutex<Vec<String>>>,
    worker: Option<JoinHandle<()>>,
    base_url: String,
}

#[derive(Clone, Copy)]
enum ReplyMode {
    ExpireAfterFirstCall,
    AlwaysUnauthorized,
    UnauthorizedAfterStreamStarts,
}

impl RefreshServer {
    fn start() -> Self {
        Self::with_mode(ReplyMode::ExpireAfterFirstCall)
    }

    fn with_mode(mode: ReplyMode) -> Self {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").expect("bind fixture HTTP"));
        let base_url = format!("http://{}/v1", server.server_addr());
        let stopped = Arc::new(AtomicBool::new(false));
        let authorizations = Arc::new(Mutex::new(Vec::new()));
        let worker_server = server.clone();
        let worker_stopped = stopped.clone();
        let worker_authorizations = authorizations.clone();
        let worker = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(8);
            while !worker_stopped.load(Ordering::SeqCst) && Instant::now() < deadline {
                let request = match worker_server.recv_timeout(Duration::from_millis(50)) {
                    Ok(Some(request)) => request,
                    Ok(None) => continue,
                    Err(_) => break,
                };
                let authorization = request
                    .headers()
                    .iter()
                    .find(|header| header.field.equiv("Authorization"))
                    .map(|header| header.value.as_str().to_string())
                    .unwrap_or_default();
                let first = {
                    let mut observed = worker_authorizations.lock().unwrap();
                    let first = observed.is_empty();
                    observed.push(authorization.clone());
                    first
                };
                if matches!(mode, ReplyMode::UnauthorizedAfterStreamStarts) {
                    let body = concat!(
                        "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"},\"finish_reason\":null}]}\n\n",
                        "data: {\"error\":{\"code\":401,\"status\":\"UNAUTHENTICATED\",\"message\":\"fixture stream denied\"}}\n\n"
                    );
                    let response = tiny_http::Response::from_string(body).with_header(
                        tiny_http::Header::from_bytes("Content-Type", "text/event-stream").unwrap(),
                    );
                    let _ = request.respond(response);
                } else if matches!(mode, ReplyMode::ExpireAfterFirstCall)
                    && ((first && authorization == "Bearer fixture-token-1")
                        || authorization == "Bearer fixture-token-2")
                {
                    let body = concat!(
                        "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"},\"finish_reason\":null}]}\n\n",
                        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
                        "data: [DONE]\n\n"
                    );
                    let response = tiny_http::Response::from_string(body).with_header(
                        tiny_http::Header::from_bytes("Content-Type", "text/event-stream").unwrap(),
                    );
                    let _ = request.respond(response);
                } else {
                    let response = tiny_http::Response::from_string(
                        r#"{"error":{"reason":"ACCESS_TOKEN_EXPIRED"}}"#,
                    )
                    .with_status_code(401);
                    let _ = request.respond(response);
                }
            }
        });
        Self { server, stopped, authorizations, worker: Some(worker), base_url }
    }
}

impl Drop for RefreshServer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.server.unblock();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl crate::gcp_auth::AccessTokenSource for FakeTokenSource {
    fn token(&self) -> Result<String, LlmError> { Ok(FakeTokenSource::token(self).into()) }
    fn invalidate(&self) { FakeTokenSource::invalidate(self); }
}

fn current_declared_vertex(source: &Arc<FakeTokenSource>, base_url: &str) -> OpenAiLlm {
    let mut client = OpenAiLlm::vertex_with_source("fixture-project", "global", source.clone(), "fixture-model")
        .with_base_url(base_url)
        .without_retry();
    client.agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(3)))
            .build(),
    );
    client
}

#[test]
fn vertex_rejected_token_is_refreshed_once_before_opening_a_stream() {
    let source = Arc::new(FakeTokenSource::default());
    let server = RefreshServer::start();
    let client = current_declared_vertex(&source, &server.base_url);
    let turns = [Turn::user("fixture request")];
    let mut first = StringSink::default();
    client.generate(&turns, &GenOptions::default(), &mut first).expect("first fixture call");
    assert_eq!(first.text, "ok");

    let mut second = StringSink::default();
    let result = client.generate(&turns, &GenOptions::default(), &mut second);
    assert!(result.is_ok(), "Vertex must refresh one rejected token before retrying: {result:?}");
    assert_eq!(second.text, "ok", "the successful stream must be emitted once");
    assert_eq!(source.invalidations.load(Ordering::SeqCst), 1);
    assert_eq!(
        *server.authorizations.lock().unwrap(),
        ["Bearer fixture-token-1", "Bearer fixture-token-1", "Bearer fixture-token-2"]
    );
}

#[test]
fn fixture_token_source_changes_its_cached_token_only_after_invalidation() {
    let source = FakeTokenSource::default();
    assert_eq!(source.token(), "fixture-token-1");
    assert_eq!(source.token(), "fixture-token-1");
    source.invalidate();
    assert_eq!(source.token(), "fixture-token-2");
    assert_eq!(source.acquisitions.load(Ordering::SeqCst), 3);
    assert_eq!(source.invalidations.load(Ordering::SeqCst), 1);
}

#[test]
fn vertex_persistent_unauthorized_is_named_after_one_refresh_only() {
    let source = Arc::new(FakeTokenSource::default());
    let server = RefreshServer::with_mode(ReplyMode::AlwaysUnauthorized);
    let client = current_declared_vertex(&source, &server.base_url);
    let mut sink = StringSink::default();
    let error = client.generate(&[Turn::user("fixture request")], &GenOptions::default(), &mut sink)
        .expect_err("a persistent 401 must remain an error");
    assert_eq!(source.invalidations.load(Ordering::SeqCst), 1, "one invalidation only");
    assert_eq!(
        *server.authorizations.lock().unwrap(),
        ["Bearer fixture-token-1", "Bearer fixture-token-2"],
        "an authentication refresh must not become a retry loop"
    );
    let message = error.to_string();
    assert!(message.contains("Vertex authentication rejected after refresh"), "{message}");
    assert!(message.contains("401"), "{message}");
    assert!(!message.contains("fixture-token-"), "error must not disclose a token");
    assert!(sink.text.is_empty());
}

#[test]
fn a_fixed_api_bearer_is_not_refreshed_after_unauthorized() {
    let server = RefreshServer::with_mode(ReplyMode::AlwaysUnauthorized);
    let mut client = OpenAiLlm::new(&server.base_url, "fixture-model")
        .with_auth(Auth::Bearer("fixture-api-key".into()))
        .without_retry();
    client.agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(3)))
            .build(),
    );
    let mut sink = StringSink::default();
    let error = client.generate(&[Turn::user("fixture request")], &GenOptions::default(), &mut sink)
        .expect_err("a fixed API key rejection is terminal");
    assert!(error.to_string().contains("401"));
    assert_eq!(*server.authorizations.lock().unwrap(), ["Bearer fixture-api-key"]);
    assert!(sink.text.is_empty());
}

#[test]
fn vertex_never_refreshes_or_replays_after_a_stream_fragment() {
    let source = Arc::new(FakeTokenSource::default());
    let server = RefreshServer::with_mode(ReplyMode::UnauthorizedAfterStreamStarts);
    let client = current_declared_vertex(&source, &server.base_url);
    let mut sink = StringSink::default();
    let error = client.generate(&[Turn::user("fixture request")], &GenOptions::default(), &mut sink)
        .expect_err("a denied stream must return its error");
    assert!(error.to_string().contains("401"));
    assert_eq!(sink.text, "partial", "an opened stream must never be emitted twice");
    assert_eq!(source.invalidations.load(Ordering::SeqCst), 0);
    assert_eq!(*server.authorizations.lock().unwrap(), ["Bearer fixture-token-1"]);
}

#[test]
fn declared_vertex_connector_retains_injected_source_and_refreshes() {
    use crate::model_source::{connect_llm_with_vertex, ModelSource, Origin};
    let source = Arc::new(FakeTokenSource::default());
    let server = RefreshServer::start();
    let declaration: ModelSource = serde_json::from_value(json!({
        "provider":"compatible", "protocol":"vertex", "model":"fixture-model",
        "context_tokens":12345, "params":{"project":"fixture-project","location":"global"}
    })).unwrap();
    let mut constructed = 0;
    let (client, origin) = connect_llm_with_vertex(&declaration, |project, location, model| {
        constructed += 1;
        assert_eq!((project,location,model), ("fixture-project","global","fixture-model"));
        Ok(current_declared_vertex(&source, &server.base_url))
    }).expect("real declared provider branch with injected constructor, no from_env");
    assert_eq!(constructed,1);
    assert_eq!(origin,Origin::Compatible("vertex:fixture-project/global".into()));
    assert_eq!(client.context_len(),12345);
    assert_eq!(source.acquisitions.load(Ordering::SeqCst),0,"connection must retain capability, not fetch a snapshot");
    for _ in 0..2 {
        let mut sink=StringSink::default();
        client.generate(&[Turn::user("fixture")],&GenOptions::default(),&mut sink).unwrap();
        assert_eq!(sink.text,"ok");
    }
    assert_eq!(source.invalidations.load(Ordering::SeqCst),1);
    assert_eq!(*server.authorizations.lock().unwrap(),["Bearer fixture-token-1","Bearer fixture-token-1","Bearer fixture-token-2"]);
}

#[test]
fn dynamic_vertex_auth_debug_never_discloses_a_token() {
    let source = Arc::new(FakeTokenSource::default());
    let debug=format!("{:?}",OpenAiLlm::vertex_with_source("p","global",source,"m"));
    assert!(!debug.contains("fixture-token"));
    assert!(debug.contains("redacted"));
}

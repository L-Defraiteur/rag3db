//! Transport reproducer for the current declared Vertex token snapshot.
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

impl RefreshServer {
    fn start() -> Self {
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
                if (first && authorization == "Bearer fixture-token-1")
                    || authorization == "Bearer fixture-token-2"
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

// Exact current connect_llm boundary: snapshot token once, pass it to the
// REAL Vertex client constructor. After the observed red, wire this fixture
// to the production retained-source constructor without changing the scenario.
fn current_declared_vertex(source: &FakeTokenSource, base_url: &str) -> OpenAiLlm {
    let mut client = OpenAiLlm::vertex("fixture-project", "global", source.token(), "fixture-model")
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

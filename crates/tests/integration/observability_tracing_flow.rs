use std::{
    io,
    io::Write,
    sync::{Arc, Mutex},
};

use app_server::{
    config::AppConfig,
    router::build_router,
    state::{AppState, ProbeSnapshot},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use core_infra::surrealdb::InMemorySurrealDb;
use serde_json::{Value, json};
use tower::ServiceExt;
use tracing_subscriber::fmt::format::FmtSpan;

const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
const TRACESTATE: &str = "vendor=trace-state";

#[tokio::test]
async fn public_endpoints_propagate_trace_context_headers() {
    let logs = capture_tracing_output(async {
        let db = Arc::new(InMemorySurrealDb::new());
        let state =
            AppState::for_memory_ingest_test(AppConfig::for_test(), ProbeSnapshot::ready(), db);
        let app = build_router(state.clone());

        assert_tracing_headers(&app, request("GET", "/health", None), StatusCode::OK).await;
        assert_tracing_headers(&app, request("GET", "/ready", None), StatusCode::OK).await;

        let register = send(
            &app,
            request(
                "POST",
                "/sources/register",
                Some(json!({
                    "@context": ["https://www.w3.org/ns/credentials/v2"],
                    "type": ["VerifiableCredential", "OpenBadgeCredential"],
                    "id": "urn:badge:trace-flow-001",
                    "name": "Tracing Badge",
                    "issuer": {"id": "https://issuer.example.org"}
                })),
            ),
        )
        .await;
        assert_eq!(register.status(), StatusCode::CREATED);
        assert_common_headers(&register);
        let register_body = decode_json(register).await;
        let source_id = register_body["source_id"].as_str().expect("source id");
        let urn = register_body["memory_items"][0]["urn"]
            .as_str()
            .expect("urn");

        state
            .memory_ingest()
            .expect("memory ingest services configured")
            .index_memory_items_service()
            .process_next_job()
            .await
            .expect("worker run should succeed")
            .expect("job should exist");

        assert_tracing_headers(
            &app,
            request("GET", &format!("/sources/{source_id}"), None),
            StatusCode::OK,
        )
        .await;
        assert_tracing_headers(
            &app,
            request("GET", &format!("/memory-items/{urn}"), None),
            StatusCode::OK,
        )
        .await;
        assert_tracing_headers(
            &app,
            request(
                "GET",
                "/search/memory-items?q=Tracing&document-type=json&limit=10&offset=0",
                None,
            ),
            StatusCode::OK,
        )
        .await;
    })
    .await;

    assert!(logs.contains("http.request"));
    assert!(logs.contains("/sources/register"));
    assert!(logs.contains("/sources/"));
    assert!(logs.contains("/memory-items/"));
    assert!(logs.contains("/search/memory-items"));
    assert!(logs.contains("trace_id\":\"4bf92f3577b34da6a3ce929d0e0e4736"));
    assert!(logs.contains("request_id"));
}

async fn assert_tracing_headers(app: &axum::Router, request: Request<Body>, expected: StatusCode) {
    let response = send(app, request).await;
    assert_eq!(response.status(), expected);
    assert_common_headers(&response);
}

fn assert_common_headers(response: &axum::response::Response) {
    assert_eq!(
        response
            .headers()
            .get("traceparent")
            .and_then(|value| value.to_str().ok()),
        Some(TRACEPARENT)
    );
    assert_eq!(
        response
            .headers()
            .get("tracestate")
            .and_then(|value| value.to_str().ok()),
        Some(TRACESTATE)
    );
    assert!(response.headers().contains_key("x-request-id"));
}

fn request(method: &str, uri: &str, payload: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("traceparent", TRACEPARENT)
        .header("tracestate", TRACESTATE);
    if payload.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    builder
        .body(payload.map_or_else(Body::empty, |body| Body::from(body.to_string())))
        .expect("request must build")
}

async fn send(app: &axum::Router, request: Request<Body>) -> axum::response::Response {
    app.clone()
        .oneshot(request)
        .await
        .expect("request must succeed")
}

async fn decode_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must decode");
    serde_json::from_slice(&body).expect("response must be valid json")
}

async fn capture_tracing_output(future: impl std::future::Future<Output = ()>) -> String {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_target(false)
        .json()
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_writer(move || SharedWriter(writer.clone()))
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    future.await;

    let raw_bytes = {
        let guard = buffer.lock().expect("log buffer poisoned");
        guard.clone()
    };

    String::from_utf8(raw_bytes).expect("logs should be UTF-8")
}

#[derive(Clone)]
struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("log buffer poisoned")
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

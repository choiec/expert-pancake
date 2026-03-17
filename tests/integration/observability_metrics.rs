use std::sync::Arc;

use app_server::{
    config::AppConfig,
    router::build_router,
    state::{AppState, ProbeSnapshot},
};
use axum::{
    body::Body,
    http::{Request, header},
};
use core_infra::surrealdb::InMemorySurrealDb;
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
async fn histogram_metrics_cover_all_public_endpoints_with_bounded_labels() {
    let db = Arc::new(InMemorySurrealDb::new());
    let state = AppState::for_memory_ingest_test(AppConfig::for_test(), ProbeSnapshot::ready(), db);
    let app = build_router(state.clone());

    let canonical = post_json(
        &app,
        "/sources/register",
        json!({
            "title": "Metrics markdown",
            "external-id": "metrics-001",
            "document-type": "markdown",
            "content": "# Intro\n\nmetrics path"
        }),
    )
    .await;
    let canonical_source_id = canonical["source_id"]
        .as_str()
        .expect("source id")
        .to_owned();
    let canonical_urn = canonical["memory_items"][0]["urn"]
        .as_str()
        .expect("urn")
        .to_owned();

    post_json(
        &app,
        "/sources/register",
        json!({
            "@context": ["https://www.w3.org/ns/credentials/v2"],
            "type": ["VerifiableCredential", "OpenBadgeCredential"],
            "id": "urn:badge:metrics-002",
            "name": "Metrics Badge",
            "issuer": {"id": "https://issuer.example.org"}
        }),
    )
    .await;

    let worker = state
        .memory_ingest()
        .expect("memory ingest services configured")
        .index_memory_items_service();
    worker
        .process_next_job()
        .await
        .expect("first worker run should succeed")
        .expect("first job should exist");
    worker
        .process_next_job()
        .await
        .expect("second worker run should succeed")
        .expect("second job should exist");

    get(&app, "/health").await;
    get(&app, "/ready").await;
    get(&app, &format!("/sources/{canonical_source_id}")).await;
    get(&app, &format!("/memory-items/{canonical_urn}")).await;
    get(
        &app,
        "/search/memory-items?q=Metrics&document-type=json&limit=10&offset=0",
    )
    .await;

    let metrics = state.request_metrics().render_prometheus();
    assert!(metrics.contains("route=\"/health\""));
    assert!(metrics.contains("route=\"/ready\""));
    assert!(metrics.contains("route=\"/sources/register\""));
    assert!(metrics.contains("route=\"/sources/{source-id}\""));
    assert!(metrics.contains("route=\"/memory-items/{urn}\""));
    assert!(metrics.contains("route=\"/search/memory-items\""));
    assert!(metrics.contains("document_type=\"markdown\""));
    assert!(metrics.contains("document_type=\"json\""));
    assert!(metrics.contains("ingest_kind=\"canonical\""));
    assert!(metrics.contains("ingest_kind=\"direct_standard\""));
    assert!(metrics.contains("http_request_latency_ms_bucket"));
    assert!(metrics.contains("http_request_latency_ms_sum"));
    assert!(metrics.contains("http_request_latency_ms_count"));
}

async fn post_json(app: &axum::Router, uri: &str, payload: Value) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    assert!(response.status().is_success());
    decode_json(response).await
}

async fn get(app: &axum::Router, uri: &str) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    assert!(response.status().is_success());
}

async fn decode_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must decode");
    serde_json::from_slice(&body).expect("response must be valid json")
}

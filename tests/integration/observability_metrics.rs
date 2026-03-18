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
            "external-id": "https://api.cherry-pick.net/cc/v1p3/example.edu:metrics-001",
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
    assert!(metrics.contains("http_request_latency_ms_bucket"));
    assert!(metrics.contains("http_request_latency_ms_sum"));
    assert!(metrics.contains("http_request_latency_ms_count"));

    assert_eq!(
        metric_count(
            &metrics,
            "/sources/register",
            201,
            Some("markdown"),
            Some("canonical"),
            Some("steady_state"),
            Some("MANUAL_CANONICAL_ACCEPTED")
        ),
        1
    );
    assert_eq!(
        metric_count(
            &metrics,
            "/sources/register",
            201,
            Some("json"),
            Some("direct_standard"),
            Some("steady_state"),
            Some("DIRECT_STANDARD_CANONICALIZED")
        ),
        1
    );
    assert_eq!(metric_count(&metrics, "/health", 200, None, None, None, None), 1);
    assert_eq!(metric_count(&metrics, "/ready", 200, None, None, None, None), 1);
    assert_eq!(
        metric_count(
            &metrics,
            "/sources/{source-id}",
            200,
            Some("markdown"),
            None,
            Some("steady_state"),
            Some("LOOKUP_RESOLVED_CANONICAL")
        ),
        1
    );
    assert_eq!(
        metric_count(&metrics, "/memory-items/{urn}", 200, Some("markdown"), None, None, None),
        1
    );
    assert_eq!(
        metric_count(&metrics, "/search/memory-items", 200, Some("json"), None, None, None),
        1
    );

    for bucket in [50, 100, 200, 500, 1_000, 5_000] {
        assert!(metrics.contains(&format!("le=\"{bucket}\"")));
    }
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

fn metric_count(
    metrics: &str,
    route: &str,
    status_code: u16,
    document_type: Option<&str>,
    ingest_kind: Option<&str>,
    migration_phase: Option<&str>,
    decision_reason: Option<&str>,
) -> u64 {
    let document_type = document_type.unwrap_or("unknown");
    let ingest_kind = ingest_kind.unwrap_or("unknown");
    let migration_phase = migration_phase.unwrap_or("unknown");
    let decision_reason = decision_reason.unwrap_or("unknown");

    metrics
        .lines()
        .find(|line| {
            line.starts_with("http_request_latency_ms_count{")
                && line.contains(&format!("route=\"{route}\""))
                && line.contains(&format!("status_code=\"{status_code}\""))
                && line.contains(&format!("document_type=\"{document_type}\""))
                && line.contains(&format!("ingest_kind=\"{ingest_kind}\""))
                && line.contains(&format!("migration_phase=\"{migration_phase}\""))
                && line.contains(&format!("decision_reason=\"{decision_reason}\""))
        })
        .and_then(|line| line.rsplit_once(' '))
        .and_then(|(_, value)| value.parse::<u64>().ok())
        .unwrap_or_default()
}

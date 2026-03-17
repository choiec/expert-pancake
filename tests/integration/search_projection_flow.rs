use std::sync::Arc;

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

#[tokio::test]
async fn healthy_indexing_makes_registered_content_searchable() {
    let db = Arc::new(InMemorySurrealDb::new());
    let state = AppState::for_memory_ingest_test(AppConfig::for_test(), ProbeSnapshot::ready(), db);
    let app = build_router(state.clone());

    let registered = post_json(
        &app,
        "/sources/register",
        json!({
            "title": "Flow markdown",
            "summary": "healthy search",
            "external-id": "flow-search-001",
            "document-type": "markdown",
            "content": "# Intro\n\nhello integration world"
        }),
    )
    .await;
    assert_eq!(registered.0, StatusCode::CREATED);
    assert_eq!(registered.1["indexing_status"], "queued");
    let source_id = registered.1["source_id"].as_str().expect("source id");

    state
        .memory_ingest()
        .expect("memory ingest services configured")
        .index_memory_items_service()
        .process_next_job()
        .await
        .expect("worker run should succeed")
        .expect("job should exist");

    let search = get_json(&app, "/search/memory-items?q=integration&limit=10&offset=0").await;
    assert_eq!(search.0, StatusCode::OK);
    assert_eq!(search.1["total"], 1);
    assert_eq!(search.1["items"][0]["document_type"], "markdown");

    let source = get_json(&app, &format!("/sources/{source_id}")).await;
    assert_eq!(source.0, StatusCode::OK);
    assert_eq!(source.1["indexing_status"], "indexed");
}

#[tokio::test]
async fn direct_standard_projection_is_searchable_as_json() {
    let db = Arc::new(InMemorySurrealDb::new());
    let state = AppState::for_memory_ingest_test(AppConfig::for_test(), ProbeSnapshot::ready(), db);
    let app = build_router(state.clone());

    let registered = post_json(
        &app,
        "/sources/register",
        json!({
            "@context": ["https://www.w3.org/ns/credentials/v2"],
            "type": ["VerifiableCredential", "OpenBadgeCredential"],
            "id": "urn:badge:flow-001",
            "name": "Rust Badge Flow",
            "issuer": {"id": "https://issuer.example.org"}
        }),
    )
    .await;
    assert_eq!(registered.0, StatusCode::CREATED);

    state
        .memory_ingest()
        .expect("memory ingest services configured")
        .index_memory_items_service()
        .process_next_job()
        .await
        .expect("worker run should succeed")
        .expect("job should exist");

    let search = get_json(
        &app,
        "/search/memory-items?q=Rust&document-type=json&limit=10&offset=0",
    )
    .await;
    assert_eq!(search.0, StatusCode::OK);
    assert_eq!(search.1["total"], 1);
    assert_eq!(search.1["items"][0]["document_type"], "json");
    assert!(
        search.1["items"][0]["content_preview"]
            .as_str()
            .expect("preview string")
            .contains("Rust Badge Flow")
    );
}

#[tokio::test]
async fn registration_still_succeeds_when_search_is_degraded() {
    let db = Arc::new(InMemorySurrealDb::new());
    db.set_search_available(false);
    let state = AppState::for_memory_ingest_test_with_projection(
        AppConfig::for_test(),
        ProbeSnapshot::new(
            app_server::state::ProbeStatus::Ready,
            app_server::state::ProbeStatus::Ready,
            app_server::state::ProbeStatus::Degraded,
        ),
        db,
        false,
    );
    let app = build_router(state);

    let registered = post_json(
        &app,
        "/sources/register",
        json!({
            "title": "Degraded search",
            "external-id": "flow-search-degraded-001",
            "document-type": "text",
            "content": "search degradation should not block registration"
        }),
    )
    .await;
    assert_eq!(registered.0, StatusCode::CREATED);
    assert_eq!(registered.1["indexing_status"], "deferred");

    let search = get_json(&app, "/search/memory-items?q=search").await;
    assert_eq!(search.0, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(search.1["error_code"], "SEARCH_UNAVAILABLE");
}

async fn post_json(app: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
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
    let status = response.status();
    (status, decode_json(response).await)
}

async fn get_json(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
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
    let status = response.status();
    (status, decode_json(response).await)
}

async fn decode_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must decode");
    serde_json::from_slice(&body).expect("response must be valid json")
}

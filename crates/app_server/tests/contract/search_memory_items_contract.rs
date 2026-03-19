use std::{fs, path::Path};
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
async fn search_contract_matches_openapi_and_returns_projection_hits() {
    let contract = load_contract();
    assert!(contract.contains("/search/memory-items:"));
    assert!(contract.contains("name: source-id"));
    assert!(contract.contains("name: document-type"));
    assert!(contract.contains("name: limit"));
    assert!(contract.contains("name: offset"));
    assert!(contract.contains("$ref: '#/components/schemas/SearchResponse'"));

    let db = Arc::new(InMemorySurrealDb::new());
    let state = AppState::for_memory_ingest_test(AppConfig::for_test(), ProbeSnapshot::ready(), db);
    let app = build_router(state.clone());

    register_source(
        &app,
        json!({
            "title": "Searchable markdown",
            "summary": "contract",
            "external-id": "https://api.cherry-pick.net/cc/v1p3/example.edu:search-contract-001",
            "document-type": "markdown",
            "content": "# Intro\n\nhello projection world",
            "metadata": {"topic": "contract"}
        }),
    )
    .await;

    state
        .memory_ingest()
        .expect("memory ingest services configured")
        .index_memory_items_service()
        .process_next_job()
        .await
        .expect("worker run should succeed")
        .expect("job should exist");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/search/memory-items?q=hello&limit=10&offset=0")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = decode_json(response).await;
    assert_eq!(body["limit"], 10);
    assert_eq!(body["offset"], 0);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["document_type"], "markdown");
    assert!(
        body["items"][0]["content_preview"]
            .as_str()
            .expect("preview string")
            .contains("hello projection world")
    );
    assert!(body["items"][0].get("content").is_none());
}

#[tokio::test]
async fn search_contract_supports_document_type_json_filter() {
    let db = Arc::new(InMemorySurrealDb::new());
    let state = AppState::for_memory_ingest_test(AppConfig::for_test(), ProbeSnapshot::ready(), db);
    let app = build_router(state.clone());

    register_source(
        &app,
        json!({
            "@context": ["https://www.w3.org/ns/credentials/v2"],
            "type": ["VerifiableCredential", "OpenBadgeCredential"],
            "id": "urn:badge:search-contract-001",
            "name": "Rust Badge Search",
            "issuer": {"id": "https://issuer.example.org"}
        }),
    )
    .await;

    state
        .memory_ingest()
        .expect("memory ingest services configured")
        .index_memory_items_service()
        .process_next_job()
        .await
        .expect("worker run should succeed")
        .expect("job should exist");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/search/memory-items?q=Rust&document-type=json&limit=5&offset=0")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = decode_json(response).await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["document_type"], "json");
    assert!(
        body["items"][0]["content_preview"]
            .as_str()
            .expect("preview string")
            .contains("Rust Badge Search")
    );
}

#[tokio::test]
async fn search_contract_returns_structured_503_when_projection_is_unavailable() {
    let contract = load_contract();
    assert!(contract.contains("'503':\n          description: Search unavailable."));

    let db = Arc::new(InMemorySurrealDb::new());
    let state = AppState::for_memory_ingest_test_with_projection(
        AppConfig::for_test(),
        ProbeSnapshot::ready(),
        db,
        false,
    );
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/search/memory-items?q=hello")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = decode_json(response).await;
    assert_eq!(body["error_code"], "SEARCH_UNAVAILABLE");
    assert!(
        body["message"]
            .as_str()
            .expect("message string")
            .contains("unavailable")
    );
}

fn load_contract() -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("expected workspace root")
        .to_path_buf();
    let path = workspace_root.join("specs/002-canonical-source-external-id/contracts/memory-ingest.openapi.yaml");

    fs::read_to_string(path).expect("contract file must exist")
}

async fn register_source(app: &axum::Router, payload: Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/sources/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");

    assert!(matches!(
        response.status(),
        StatusCode::CREATED | StatusCode::OK
    ));
}

async fn decode_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must decode");
    serde_json::from_slice(&body).expect("response must be valid json")
}

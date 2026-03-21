use std::sync::Arc;

use app_server::{
    config::AppConfig,
    router::build_router,
    state::{AppState, ProbeSnapshot},
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use core_infra::InMemorySurrealDb;
use tower::util::ServiceExt;

fn app() -> Router {
    let db = Arc::new(InMemorySurrealDb::new());
    build_router(AppState::for_memory_ingest_test(
        AppConfig::for_test(),
        ProbeSnapshot::ready(),
        db,
    ))
}

#[tokio::test]
async fn register_and_retrieve_standard_json() {
    let app = app();
    let body = r#"{"@context":"https://purl.imsglobal.org/spec/ob/v3p0/context.json","type":"AchievementCredential","id":"https://example.com/credential/1","name":"Rust Badge"}"#;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sources/register")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let source_id = payload["source_id"].as_str().unwrap();
    let urn = payload["memory_items"][0]["urn"].as_str().unwrap();

    let source_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/sources/{source_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source_response.status(), StatusCode::OK);

    let item_response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/memory-items/{urn}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(item_response.status(), StatusCode::OK);
}

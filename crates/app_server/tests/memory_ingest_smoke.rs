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
    assert_eq!(
        payload["external_id"],
        "https://api.cherry-pick.net/openbadges/v3p0/example.com:https%3A%2F%2Fexample.com%2Fcredential%2F1"
    );
    assert_eq!(payload["document_type"], "json");
    assert_eq!(payload["memory_items"][0]["unit_type"], "json_document");
    assert_eq!(
        payload["source_metadata"]["system"]["canonical_id_version"],
        "v1"
    );
    assert_eq!(
        payload["source_metadata"]["system"]["ingest_kind"],
        "direct_standard"
    );
    assert_eq!(
        payload["source_metadata"]["system"]["original_standard_id"],
        "https://example.com/credential/1"
    );

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
    let source_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(source_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        source_payload["source_metadata"]["system"]["semantic_payload_hash"],
        payload["source_metadata"]["system"]["semantic_payload_hash"]
    );

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
    let item_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(item_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(item_payload["content"], body);
}

#[tokio::test]
async fn canonical_manual_replay_and_conflict_follow_spec() {
    let app = app();
    let canonical_body = r##"{
        "title":"Axum Plan",
        "summary":"Planning notes",
        "external-id":"https://api.cherry-pick.net/qti/v3p0/kice.re.kr:20240621",
        "document-type":"markdown",
        "content":"# Intro\n\nHello world",
        "metadata":{"topic":"planning"}
    }"##;

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sources/register")
                .header("content-type", "application/json")
                .body(Body::from(canonical_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(created.into_body(), usize::MAX).await.unwrap()).unwrap();

    let replay = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sources/register")
                .header("content-type", "application/json")
                .body(Body::from(canonical_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(replay.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(created_payload["source_id"], replay_payload["source_id"]);

    let conflict_body = r##"{
        "title":"Axum Plan",
        "summary":"Planning notes",
        "external-id":"https://api.cherry-pick.net/qti/v3p0/kice.re.kr:20240621",
        "document-type":"markdown",
        "content":"# Intro\n\nChanged body",
        "metadata":{"topic":"planning"}
    }"##;
    let conflict = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sources/register")
                .header("content-type", "application/json")
                .body(Body::from(conflict_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn rejects_non_canonical_manual_external_id() {
    let app = app();
    let body = r#"{
        "title":"Invalid Canonical ID",
        "external-id":"https://example.com/not-canonical",
        "document-type":"text",
        "content":"hello"
    }"#;

    let response = app
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

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn standard_replay_uses_semantic_hash_and_conflict_blocks_mutation() {
    let app = app();
    let compact = r#"{"@context":"https://purl.imsglobal.org/spec/ob/v3p0/context.json","type":"AchievementCredential","id":"https://example.com/credential/2","name":"Rust Badge"}"#;
    let pretty = r#"{
      "name":"Rust Badge",
      "id":"https://example.com/credential/2",
      "type":"AchievementCredential",
      "@context":"https://purl.imsglobal.org/spec/ob/v3p0/context.json"
    }"#;
    let conflict = r#"{"@context":"https://purl.imsglobal.org/spec/ob/v3p0/context.json","type":"AchievementCredential","id":"https://example.com/credential/2","name":"Changed Badge"}"#;

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sources/register")
                .header("content-type", "application/json")
                .body(Body::from(compact))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(created.into_body(), usize::MAX).await.unwrap()).unwrap();

    let replay = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sources/register")
                .header("content-type", "application/json")
                .body(Body::from(pretty))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(replay.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(created_payload["source_id"], replay_payload["source_id"]);

    let conflict_response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sources/register")
                .header("content-type", "application/json")
                .body(Body::from(conflict))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(conflict_response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn missing_memory_item_returns_not_found() {
    let response = app()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/memory-items/urn:memory-item:missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

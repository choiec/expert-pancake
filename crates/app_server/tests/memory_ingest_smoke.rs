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

fn open_badges_payload(id: &str, name: &str) -> serde_json::Value {
    serde_json::json!({
        "@context": [
            "https://www.w3.org/ns/credentials/v2",
            "https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json"
        ],
        "id": id,
        "type": ["VerifiableCredential", "AchievementCredential"],
        "name": name,
        "issuer": "https://issuer.example.com/issuers/1",
        "validFrom": "2025-01-01T00:00:00Z",
        "credentialSubject": {
            "type": "AchievementSubject",
            "achievement": {
                "id": "https://example.com/achievements/rust-badge",
                "type": "Achievement",
                "name": name,
                "description": "Awarded for Rust basics",
                "criteria": {}
            }
        }
    })
}

fn clr_payload(id: &str, name: &str) -> serde_json::Value {
    serde_json::json!({
        "@context": [
            "https://www.w3.org/ns/credentials/v2",
            "https://purl.imsglobal.org/spec/clr/v2p0/context-2.0.1.json",
            "https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json"
        ],
        "type": ["VerifiableCredential", "ClrCredential"],
        "id": id,
        "name": name,
        "issuer": "https://issuer.example.com/issuers/1",
        "validFrom": "2025-01-01T00:00:00Z",
        "credentialSubject": {
            "type": "ClrSubject",
            "verifiableCredential": [
                {
                    "@context": [
                        "https://www.w3.org/ns/credentials/v2",
                        "https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json"
                    ],
                    "id": "https://example.com/credential/embedded-rust-badge",
                    "type": ["VerifiableCredential", "AchievementCredential"],
                    "name": "Embedded Rust Badge",
                    "issuer": "https://issuer.example.com/issuers/1",
                    "validFrom": "2025-01-01T00:00:00Z",
                    "credentialSubject": {
                        "type": "AchievementSubject",
                        "achievement": {
                            "id": "https://example.com/achievements/embedded-rust-badge",
                            "type": "Achievement",
                            "name": "Embedded Rust Badge",
                            "description": "Awarded for Rust basics",
                            "criteria": {}
                        }
                    }
                }
            ]
        }
    })
}

#[tokio::test]
async fn register_and_retrieve_open_badges_json() {
    let app = app();
    let body = open_badges_payload("https://example.com/credential/1", "Rust Badge").to_string();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sources/register")
                .header("content-type", "application/json")
                .body(Body::from(body.clone()))
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
        "https://api.cherry-pick.net/openbadges/v3p0/issuer.example.com:https%3A%2F%2Fexample.com%2Fcredential%2F1"
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
async fn register_and_retrieve_clr_json() {
    let app = app();
    let body = clr_payload("https://example.com/clr/1", "Rust Learner Record").to_string();
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
    assert_eq!(
        payload["external_id"],
        "https://api.cherry-pick.net/clr/v2p0/issuer.example.com:https%3A%2F%2Fexample.com%2Fclr%2F1"
    );
    assert_eq!(payload["document_type"], "json");
    assert_eq!(
        payload["source_metadata"]["system"]["original_standard_id"],
        "https://example.com/clr/1"
    );
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
    let compact = open_badges_payload("https://example.com/credential/2", "Rust Badge").to_string();
    let pretty = serde_json::to_string_pretty(&open_badges_payload(
        "https://example.com/credential/2",
        "Rust Badge",
    ))
    .unwrap();
    let conflict =
        open_badges_payload("https://example.com/credential/2", "Changed Badge").to_string();

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
async fn invalid_standard_envelope_returns_bad_request() {
    let response = app()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sources/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"@context":"https://purl.imsglobal.org/spec/ob/v3p0/context.json","type":"AchievementCredential","id":"https://example.com/credential/invalid","name":"Incomplete Badge"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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

use std::{fs, path::Path};

use app_server::{
    config::AppConfig,
    router::build_router,
    state::{AppState, ProbeSnapshot, ProbeStatus},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn health_matches_openapi_and_local_only_shape() {
    let contract = load_contract();
    assert!(contract.contains("/health:"));
    assert!(contract.contains("Local-only liveness probe."));
    assert!(contract.contains("$ref: '#/components/schemas/HealthResponse'"));

    let app = build_router(AppState::for_test(
        AppConfig::for_test(),
        ProbeSnapshot::ready(),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));

    let body = decode_json(response).await;
    assert_eq!(body["status"], "ready");
    assert_eq!(body["components"]["service"], "ready");
    assert!(body["components"].get("database").is_none());
    assert!(body["components"].get("search").is_none());
}

#[tokio::test]
async fn health_does_not_change_when_dependencies_are_down() {
    let app = build_router(AppState::for_test(
        AppConfig::for_test(),
        ProbeSnapshot::new(ProbeStatus::Down, ProbeStatus::Down, ProbeStatus::Degraded),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = decode_json(response).await;
    assert_eq!(body["status"], "ready");
    assert_eq!(body["components"]["service"], "ready");
    assert!(body["components"].get("database").is_none());
    assert!(body["components"].get("search").is_none());
}

#[tokio::test]
async fn ready_matches_openapi_when_dependencies_are_ready() {
    let contract = load_contract();
    assert!(contract.contains("/ready:"));
    assert!(contract.contains("Dependency-aware readiness probe for the write path."));
    assert!(contract.contains("$ref: '#/components/schemas/ReadinessResponse'"));
    assert!(
        contract
            .contains("'503':\n          description: Authoritative write path is unavailable.")
    );

    let app = build_router(AppState::for_test(
        AppConfig::for_test(),
        ProbeSnapshot::ready(),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = decode_json(response).await;
    assert_eq!(body["status"], "ready");
    assert_eq!(body["components"]["service"], "ready");
    assert_eq!(body["components"]["database"], "ready");
    assert_eq!(body["components"]["search"], "ready");
}

#[tokio::test]
async fn ready_returns_503_when_database_write_path_is_down() {
    let app = build_router(AppState::for_test(
        AppConfig::for_test(),
        ProbeSnapshot::new(ProbeStatus::Ready, ProbeStatus::Down, ProbeStatus::Degraded),
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = decode_json(response).await;
    assert_eq!(body["status"], "down");
    assert_eq!(body["components"]["service"], "ready");
    assert_eq!(body["components"]["database"], "down");
    assert_eq!(body["components"]["search"], "degraded");
}

#[tokio::test]
async fn ready_returns_200_and_degraded_when_search_is_unavailable() {
    let app = build_router(AppState::for_test(
        AppConfig::for_test(),
        ProbeSnapshot::new(
            ProbeStatus::Ready,
            ProbeStatus::Ready,
            ProbeStatus::Degraded,
        ),
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = decode_json(response).await;
    assert_eq!(body["status"], "degraded");
    assert_eq!(body["components"]["service"], "ready");
    assert_eq!(body["components"]["database"], "ready");
    assert_eq!(body["components"]["search"], "degraded");
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

async fn decode_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must decode");

    serde_json::from_slice(&body).expect("response must be valid json")
}

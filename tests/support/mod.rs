#![allow(dead_code)]

use std::{fs, sync::Arc};

use app_server::{
    config::AppConfig,
    router::build_router,
    state::{AppState, ProbeSnapshot},
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
    response::Response,
};
use core_infra::surrealdb::InMemorySurrealDb;
use serde_json::Value;
use tower::ServiceExt;

pub fn load_contract() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path =
        format!("{manifest_dir}/specs/001-memory-ingest/contracts/memory-ingest.openapi.yaml");

    fs::read_to_string(path).expect("contract file must exist")
}

pub fn load_fixture(relative_path: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = format!("{manifest_dir}/tests/fixtures/{relative_path}");

    fs::read_to_string(path).expect("fixture file must exist")
}

pub fn build_memory_ingest_app(db: Arc<InMemorySurrealDb>) -> Router {
    build_router(AppState::for_memory_ingest_test(
        AppConfig::for_test(),
        ProbeSnapshot::ready(),
        db,
    ))
}

pub async fn send_json(app: Router, method: Method, uri: &str, body: &str) -> Response {
    app.oneshot(
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .expect("request must build"),
    )
    .await
    .expect("request must succeed")
}

pub async fn send_empty(app: Router, method: Method, uri: &str) -> Response {
    app.oneshot(
        Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .expect("request must build"),
    )
    .await
    .expect("request must succeed")
}

pub async fn decode_json(response: Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must decode");

    serde_json::from_slice(&body).expect("response must be valid json")
}

pub async fn assert_status_json(response: Response, expected: StatusCode) -> Value {
    assert_eq!(response.status(), expected);
    decode_json(response).await
}

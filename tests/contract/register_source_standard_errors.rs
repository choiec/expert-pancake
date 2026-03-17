#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;

use support::{assert_status_json, build_memory_ingest_app, load_fixture, send_json};

#[tokio::test]
async fn schema_invalid_standard_payloads_return_invalid_input_without_state() {
    let db = Arc::new(InMemorySurrealDb::new());
    let response = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/validation_matrix/open_badges_schema_invalid.json"),
    )
    .await;
    let payload = assert_status_json(response, StatusCode::BAD_REQUEST).await;

    assert_eq!(payload["error_code"], "INVALID_INPUT");
    assert!(
        db.lookup_source_by_external_id("urn:example:badge:matrix-invalid")
            .is_none()
    );
}

#[tokio::test]
async fn unmappable_standard_payloads_return_invalid_standard_payload_without_state() {
    let db = Arc::new(InMemorySurrealDb::new());
    let response = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/validation_matrix/clr_unmappable.json"),
    )
    .await;
    let payload = assert_status_json(response, StatusCode::BAD_REQUEST).await;

    assert_eq!(payload["error_code"], "INVALID_STANDARD_PAYLOAD");
    assert!(
        db.lookup_source_by_external_id("https://clr.example/credentials/matrix-unmappable")
            .is_none()
    );
}

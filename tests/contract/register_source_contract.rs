#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;

use support::{
    assert_status_json, build_memory_ingest_app, load_contract, load_fixture, send_json,
};

#[tokio::test]
async fn register_source_contract_pins_status_matrix() {
    let contract = load_contract();

    assert!(contract.contains("/sources/register:"));
    for status in [
        "'200'", "'201'", "'400'", "'408'", "'409'", "'413'", "'503'",
    ] {
        assert!(contract.contains(status), "missing status {status}");
    }
    assert!(contract.contains("OpenBadgesRegisterRequest"));
    assert!(contract.contains("ClrRegisterRequest"));
    assert!(contract.contains("enum: [queued, indexed, deferred]"));
}

#[tokio::test]
async fn canonical_registration_returns_created_shape() {
    let db = Arc::new(InMemorySurrealDb::new());
    let app = build_memory_ingest_app(db);
    let body = load_fixture("register_source/canonical_success.json");

    let response = send_json(app, Method::POST, "/sources/register", &body).await;
    let payload = assert_status_json(response, StatusCode::CREATED).await;

    assert_eq!(payload["external_id"], "canonical-source-001");
    assert_eq!(payload["document_type"], "markdown");
    assert_eq!(payload["indexing_status"], "queued");
    assert_eq!(payload["memory_items"][0]["unit_type"], "section");
}

#[tokio::test]
async fn canonical_replay_returns_200_with_same_identifiers() {
    let db = Arc::new(InMemorySurrealDb::new());
    let body = load_fixture("register_source/canonical_success.json");

    let created = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &body,
    )
    .await;
    let created_payload = assert_status_json(created, StatusCode::CREATED).await;

    let replay = send_json(
        build_memory_ingest_app(db),
        Method::POST,
        "/sources/register",
        &body,
    )
    .await;
    let replay_payload = assert_status_json(replay, StatusCode::OK).await;

    assert_eq!(replay_payload["source_id"], created_payload["source_id"]);
    assert_eq!(
        replay_payload["memory_items"],
        created_payload["memory_items"]
    );
}

#[tokio::test]
async fn canonical_conflict_returns_structured_409() {
    let db = Arc::new(InMemorySurrealDb::new());

    let created = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/canonical_success.json"),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);

    let conflict = send_json(
        build_memory_ingest_app(db),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/canonical_conflict.json"),
    )
    .await;
    let payload = assert_status_json(conflict, StatusCode::CONFLICT).await;

    assert_eq!(payload["error_code"], "EXTERNAL_ID_CONFLICT");
}

#[tokio::test]
async fn direct_standard_registration_returns_json_document_summary() {
    let db = Arc::new(InMemorySurrealDb::new());
    let open_badges = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/standards/open_badges_valid.json"),
    )
    .await;
    let open_badges_payload = assert_status_json(open_badges, StatusCode::CREATED).await;

    assert_eq!(open_badges_payload["document_type"], "json");
    assert_eq!(
        open_badges_payload["memory_items"][0]["unit_type"],
        "json_document"
    );

    let clr = send_json(
        build_memory_ingest_app(db),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/standards/clr_valid.json"),
    )
    .await;
    let clr_payload = assert_status_json(clr, StatusCode::CREATED).await;

    assert_eq!(clr_payload["document_type"], "json");
    assert_eq!(clr_payload["memory_items"][0]["unit_type"], "json_document");
}

#[tokio::test]
async fn oversized_payload_returns_413() {
    let db = Arc::new(InMemorySurrealDb::new());
    let large_body = format!(
        "{{\"title\":\"Large\",\"external-id\":\"too-large\",\"document-type\":\"text\",\"content\":\"{}\"}}",
        "a".repeat(10 * 1024 * 1024 + 1)
    );

    let response = send_json(
        build_memory_ingest_app(db),
        Method::POST,
        "/sources/register",
        &large_body,
    )
    .await;
    let payload = assert_status_json(response, StatusCode::PAYLOAD_TOO_LARGE).await;

    assert_eq!(payload["error_code"], "PAYLOAD_TOO_LARGE");
}

#[tokio::test]
async fn storage_outage_returns_503() {
    let db = Arc::new(InMemorySurrealDb::new());
    db.set_write_available(false);

    let response = send_json(
        build_memory_ingest_app(db),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/canonical_success.json"),
    )
    .await;
    let payload = assert_status_json(response, StatusCode::SERVICE_UNAVAILABLE).await;

    assert_eq!(payload["error_code"], "STORAGE_UNAVAILABLE");
}

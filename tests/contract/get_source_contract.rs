#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;
use uuid::Uuid;

use support::{
    assert_status_json, build_memory_ingest_app, load_contract, load_fixture, send_empty, send_json,
};

#[tokio::test]
async fn get_source_contract_pins_surface() {
    let contract = load_contract();

    assert!(contract.contains("/sources/{source-id}:"));
    assert!(contract.contains("Source found."));
    assert!(contract.contains("Source not found."));
    assert!(contract.contains("enum: [queued, indexed, deferred]"));
}

#[tokio::test]
async fn get_source_returns_authoritative_ordered_shape() {
    let db = Arc::new(InMemorySurrealDb::new());
    let created = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/canonical_success.json"),
    )
    .await;
    let created_payload = assert_status_json(created, StatusCode::CREATED).await;
    let source_id = created_payload["source_id"].as_str().unwrap();

    let response = send_empty(
        build_memory_ingest_app(db),
        Method::GET,
        &format!("/sources/{source_id}"),
    )
    .await;
    let payload = assert_status_json(response, StatusCode::OK).await;

    assert_eq!(payload["source_id"], source_id);
    assert_eq!(payload["indexing_status"], "queued");
    assert_eq!(payload["source_metadata"]["system"]["canonical_id_version"], "v1");
    assert_eq!(payload["memory_items"][0]["sequence"], 0);
    assert_eq!(payload["memory_items"][1]["sequence"], 1);
}

#[tokio::test]
async fn get_source_returns_structured_404() {
    let response = send_empty(
        build_memory_ingest_app(Arc::new(InMemorySurrealDb::new())),
        Method::GET,
        &format!("/sources/{}", Uuid::new_v4()),
    )
    .await;
    let payload = assert_status_json(response, StatusCode::NOT_FOUND).await;

    assert_eq!(payload["error_code"], "NOT_FOUND");
}

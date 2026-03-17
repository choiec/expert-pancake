#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;

use support::{
    assert_status_json, build_memory_ingest_app, load_contract, load_fixture, send_empty, send_json,
};

#[tokio::test]
async fn get_memory_item_contract_pins_surface() {
    let contract = load_contract();

    assert!(contract.contains("/memory-items/{urn}:"));
    assert!(contract.contains("Memory item found."));
    assert!(contract.contains("Memory item not found."));
    assert!(contract.contains("Authoritative storage unavailable."));
}

#[tokio::test]
async fn get_memory_item_returns_authoritative_shape() {
    let db = Arc::new(InMemorySurrealDb::new());
    let created = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/standards/open_badges_valid.json"),
    )
    .await;
    let created_payload = assert_status_json(created, StatusCode::CREATED).await;
    let urn = created_payload["memory_items"][0]["urn"].as_str().unwrap();

    let response = send_empty(
        build_memory_ingest_app(db),
        Method::GET,
        &format!("/memory-items/{urn}"),
    )
    .await;
    let payload = assert_status_json(response, StatusCode::OK).await;

    assert_eq!(payload["urn"], urn);
    assert_eq!(payload["item_metadata"]["unit_type"], "json_document");
    assert!(payload.get("created_at").is_some());
}

#[tokio::test]
async fn get_memory_item_returns_structured_404() {
    let response = send_empty(
        build_memory_ingest_app(Arc::new(InMemorySurrealDb::new())),
        Method::GET,
        "/memory-items/urn:mem:missing",
    )
    .await;
    let payload = assert_status_json(response, StatusCode::NOT_FOUND).await;

    assert_eq!(payload["error_code"], "NOT_FOUND");
}

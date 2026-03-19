#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;

use support::{assert_status_json, build_memory_ingest_app, load_fixture, send_empty, send_json};

#[tokio::test]
async fn memory_item_retrieval_returns_byte_accurate_authoritative_content() {
    let db = Arc::new(InMemorySurrealDb::new());
    let body = load_fixture("register_source/standards/open_badges_valid.json");

    let created = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &body,
    )
    .await;
    let created_payload = assert_status_json(created, StatusCode::CREATED).await;
    let urn = created_payload["memory_items"][0]["urn"]
        .as_str()
        .unwrap()
        .to_owned();

    let response = send_empty(
        build_memory_ingest_app(db),
        Method::GET,
        &format!("/memory-items/{urn}"),
    )
    .await;
    let payload = assert_status_json(response, StatusCode::OK).await;

    assert_eq!(payload["content"], body);
}

#[tokio::test]
async fn missing_memory_item_returns_404() {
    let response = send_empty(
        build_memory_ingest_app(Arc::new(InMemorySurrealDb::new())),
        Method::GET,
        "/memory-items/urn:missing:item",
    )
    .await;
    let payload = assert_status_json(response, StatusCode::NOT_FOUND).await;

    assert_eq!(payload["error_code"], "NOT_FOUND");
}

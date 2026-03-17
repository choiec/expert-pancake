#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;

use support::{assert_status_json, build_memory_ingest_app, load_fixture, send_empty, send_json};

#[tokio::test]
async fn direct_standard_authoritative_flow_registers_and_retrieves_consistently() {
    let db = Arc::new(InMemorySurrealDb::new());
    let standard_body = load_fixture("register_source/standards/open_badges_valid.json");

    let created = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &standard_body,
    )
    .await;
    let created_payload = assert_status_json(created, StatusCode::CREATED).await;
    let source_id = created_payload["source_id"].as_str().unwrap().to_owned();
    let urn = created_payload["memory_items"][0]["urn"]
        .as_str()
        .unwrap()
        .to_owned();

    let source_response = send_empty(
        build_memory_ingest_app(db.clone()),
        Method::GET,
        &format!("/sources/{source_id}"),
    )
    .await;
    let source_payload = assert_status_json(source_response, StatusCode::OK).await;

    let memory_response = send_empty(
        build_memory_ingest_app(db),
        Method::GET,
        &format!("/memory-items/{urn}"),
    )
    .await;
    let memory_payload = assert_status_json(memory_response, StatusCode::OK).await;

    assert_eq!(source_payload["document_type"], "json");
    assert_eq!(
        source_payload["memory_items"][0]["item_metadata"]["unit_type"],
        "json_document"
    );
    assert_eq!(memory_payload["content"], standard_body);
    assert_eq!(
        memory_payload["item_metadata"]["unit_type"],
        "json_document"
    );
}

#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;

use support::{assert_status_json, build_memory_ingest_app, load_fixture, send_empty, send_json};

#[tokio::test]
async fn source_retrieval_returns_ordered_memory_items() {
    let db = Arc::new(InMemorySurrealDb::new());

    let created = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/canonical_success.json"),
    )
    .await;
    let created_payload = assert_status_json(created, StatusCode::CREATED).await;
    let source_id = created_payload["source_id"].as_str().unwrap().to_owned();

    let response = send_empty(
        build_memory_ingest_app(db),
        Method::GET,
        &format!("/sources/{source_id}"),
    )
    .await;
    let payload = assert_status_json(response, StatusCode::OK).await;

    assert_eq!(payload["memory_items"][0]["sequence"], 0);
    assert_eq!(payload["memory_items"][1]["sequence"], 1);
    assert_eq!(payload["memory_items"][0]["source_id"], source_id);
}

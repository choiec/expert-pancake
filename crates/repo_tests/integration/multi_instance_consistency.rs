#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;

use support::{assert_status_json, build_memory_ingest_app, load_fixture, send_empty, send_json};

#[tokio::test]
async fn registration_is_visible_across_app_instances_sharing_authoritative_store() {
    let db = Arc::new(InMemorySurrealDb::new());
    let app_a = build_memory_ingest_app(db.clone());
    let app_b = build_memory_ingest_app(db.clone());

    let created = send_json(
        app_a,
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/canonical_success.json"),
    )
    .await;
    let created_payload = assert_status_json(created, StatusCode::CREATED).await;

    let source_response = send_empty(
        app_b,
        Method::GET,
        &format!(
            "/sources/{}",
            created_payload["source_id"].as_str().unwrap()
        ),
    )
    .await;
    let source_payload = assert_status_json(source_response, StatusCode::OK).await;

    assert_eq!(source_payload["source_id"], created_payload["source_id"]);
}

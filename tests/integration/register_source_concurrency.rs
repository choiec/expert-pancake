#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;

use support::{assert_status_json, build_memory_ingest_app, load_fixture, send_json};

#[tokio::test]
async fn duplicate_registration_race_returns_created_and_replay_without_duplicate_state() {
    let db = Arc::new(InMemorySurrealDb::new());
    let body = load_fixture("register_source/canonical_success.json");

    let left = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &body,
    );
    let right = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &body,
    );

    let (left, right) = tokio::join!(left, right);
    let left_status = left.status();
    let right_status = right.status();

    let left_payload = assert_status_json(left, left_status).await;
    let right_payload = assert_status_json(right, right_status).await;

    let statuses = [left_status, right_status];
    assert!(statuses.contains(&StatusCode::CREATED));
    assert!(statuses.contains(&StatusCode::OK));
    assert_eq!(left_payload["source_id"], right_payload["source_id"]);
    assert!(
        db.lookup_source_by_external_id("canonical-source-001")
            .is_some()
    );
}

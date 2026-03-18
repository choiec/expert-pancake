#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;

use support::{assert_status_json, build_memory_ingest_app, load_fixture, send_json};

#[tokio::test]
async fn canonical_success_replay_and_conflict_preserve_single_authoritative_source() {
    let db = Arc::new(InMemorySurrealDb::new());

    let created = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/canonical_success.json"),
    )
    .await;
    let created_payload = assert_status_json(created, StatusCode::CREATED).await;

    let replay = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/canonical_success.json"),
    )
    .await;
    let replay_payload = assert_status_json(replay, StatusCode::OK).await;

    let conflict = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/canonical_conflict.json"),
    )
    .await;
    let conflict_payload = assert_status_json(conflict, StatusCode::CONFLICT).await;

    let stored = db
        .lookup_source_by_external_id("https://api.cherry-pick.net/cc/v1p3/nebooks.co.kr:eng3-ch01")
        .expect("authoritative source should exist");

    assert_eq!(replay_payload["source_id"], created_payload["source_id"]);
    assert_eq!(conflict_payload["error_code"], "EXTERNAL_ID_CONFLICT");
    assert_eq!(stored.memory_items.len(), 2);
}

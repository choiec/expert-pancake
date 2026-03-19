#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;

use support::{assert_status_json, build_memory_ingest_app, load_fixture, send_empty, send_json};

#[tokio::test]
async fn open_badges_success_replay_and_conflict_preserve_first_authoritative_body() {
    let db = Arc::new(InMemorySurrealDb::new());
    let valid = load_fixture("register_source/standards/open_badges_valid.json");
    let replay = load_fixture("register_source/standards/open_badges_replay.json");
    let conflict = load_fixture("register_source/standards/open_badges_conflict.json");

    let created = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &valid,
    )
    .await;
    let created_payload = assert_status_json(created, StatusCode::CREATED).await;
    let urn = created_payload["memory_items"][0]["urn"]
        .as_str()
        .unwrap()
        .to_owned();

    let replay_response = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &replay,
    )
    .await;
    let replay_payload = assert_status_json(replay_response, StatusCode::OK).await;

    let conflict_response = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &conflict,
    )
    .await;
    let conflict_payload = assert_status_json(conflict_response, StatusCode::CONFLICT).await;

    let memory_item = send_empty(
        build_memory_ingest_app(db),
        Method::GET,
        &format!("/memory-items/{urn}"),
    )
    .await;
    let memory_item_payload = assert_status_json(memory_item, StatusCode::OK).await;

    assert_eq!(replay_payload["source_id"], created_payload["source_id"]);
    assert_eq!(conflict_payload["error_code"], "EXTERNAL_ID_CONFLICT");
    assert_eq!(memory_item_payload["content"], valid);
}

#[tokio::test]
async fn clr_success_replay_and_conflict_preserve_first_authoritative_body() {
    let db = Arc::new(InMemorySurrealDb::new());
    let valid = load_fixture("register_source/standards/clr_valid.json");
    let replay = load_fixture("register_source/standards/clr_replay.json");
    let conflict = load_fixture("register_source/standards/clr_conflict.json");

    let created = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &valid,
    )
    .await;
    let created_payload = assert_status_json(created, StatusCode::CREATED).await;
    let urn = created_payload["memory_items"][0]["urn"]
        .as_str()
        .unwrap()
        .to_owned();

    let replay_response = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &replay,
    )
    .await;
    let replay_payload = assert_status_json(replay_response, StatusCode::OK).await;

    let conflict_response = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &conflict,
    )
    .await;
    let conflict_payload = assert_status_json(conflict_response, StatusCode::CONFLICT).await;

    let memory_item = send_empty(
        build_memory_ingest_app(db),
        Method::GET,
        &format!("/memory-items/{urn}"),
    )
    .await;
    let memory_item_payload = assert_status_json(memory_item, StatusCode::OK).await;

    assert_eq!(replay_payload["source_id"], created_payload["source_id"]);
    assert_eq!(conflict_payload["error_code"], "EXTERNAL_ID_CONFLICT");
    assert_eq!(memory_item_payload["content"], valid);
}

#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;
use uuid::Uuid;

use support::{build_memory_ingest_app, load_contract, load_fixture, send_empty, send_json};

#[tokio::test]
async fn openapi_smoke_matches_implemented_routes() {
    let contract = load_contract();

    for route in [
        "/sources/register",
        "/sources/{source-id}",
        "/memory-items/{urn}",
    ] {
        assert!(contract.contains(route), "missing route {route}");
    }

    let db = Arc::new(InMemorySurrealDb::new());
    let register = send_json(
        build_memory_ingest_app(db.clone()),
        Method::POST,
        "/sources/register",
        &load_fixture("register_source/canonical_success.json"),
    )
    .await;
    assert_eq!(register.status(), StatusCode::CREATED);

    let get_source = send_empty(
        build_memory_ingest_app(db.clone()),
        Method::GET,
        &format!("/sources/{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(get_source.status(), StatusCode::NOT_FOUND);

    let get_memory = send_empty(
        build_memory_ingest_app(db),
        Method::GET,
        "/memory-items/urn:memory:missing",
    )
    .await;
    assert_eq!(get_memory.status(), StatusCode::NOT_FOUND);
}

mod support;

use core_shared::encode_credential_id;
use http::Method;
use http::StatusCode;
use serde_json::Value;

use support::{fixture, request_empty, request_json, response_json, test_app};

#[tokio::test]
async fn end_to_end_schema_native_flows_work_from_registration_to_search() {
    let (app, bundle) = test_app();

    let open_badges = fixture("register_credential/open_badges.json");
    let open_badges_id = open_badges["id"].as_str().unwrap().to_string();

    let created = request_json(
        &app,
        Method::POST,
        "/credentials/register",
        open_badges.clone(),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = response_json(created).await;
    assert_eq!(created_body["id"], Value::String(open_badges_id.clone()));
    assert!(created_body.get("source_id").is_none());

    let replay_payload = serde_json::from_str::<Value>(
        r#"{
            "validFrom":"2026-01-01T00:00:00Z",
            "proof":{"proofValue":"z3v8example","proofPurpose":"assertionMethod","verificationMethod":"https://issuer.example.org/keys/1","created":"2026-01-01T00:00:00Z","cryptosuite":"eddsa-rdfc-2022","type":"DataIntegrityProof"},
            "credentialSubject":{"achievement":{"name":"Rust Badge","type":"Achievement","id":"https://issuer.example.org/achievements/rust-badge"},"type":"AchievementSubject","id":"did:example:learner:001"},
            "issuer":{"name":"Rust Academy","type":"Profile","id":"https://issuer.example.org"},
            "name":"Rust Badge",
            "id":"urn:example:badge:001",
            "type":["VerifiableCredential","OpenBadgeCredential"],
            "@context":["https://www.w3.org/ns/credentials/v2","https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json"]
        }"#,
    )
    .unwrap();

    let replay = request_json(&app, Method::POST, "/credentials/register", replay_payload).await;
    assert_eq!(replay.status(), StatusCode::OK);

    let mut conflict_payload = open_badges.clone();
    conflict_payload["name"] = Value::String("Rust Badge Updated".to_string());
    let conflict = request_json(
        &app,
        Method::POST,
        "/credentials/register",
        conflict_payload,
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    let clr = request_json(
        &app,
        Method::POST,
        "/credentials/register",
        fixture("register_credential/clr.json"),
    )
    .await;
    assert_eq!(clr.status(), StatusCode::CREATED);

    let get_registered = request_empty(
        &app,
        Method::GET,
        &format!("/credentials/{}", encode_credential_id(&open_badges_id)),
    )
    .await;
    assert_eq!(get_registered.status(), StatusCode::OK);
    let get_registered_body = response_json(get_registered).await;
    assert_eq!(
        get_registered_body["id"],
        Value::String(open_badges_id.clone())
    );

    let get_missing = request_empty(
        &app,
        Method::GET,
        &format!(
            "/credentials/{}",
            encode_credential_id("urn:example:missing")
        ),
    )
    .await;
    assert_eq!(get_missing.status(), StatusCode::NOT_FOUND);

    let search = request_empty(
        &app,
        Method::GET,
        "/credentials/search?q=rust&family=open_badges_v3&limit=10",
    )
    .await;
    assert_eq!(search.status(), StatusCode::OK);
    let search_body = response_json(search).await;
    assert_eq!(search_body["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        search_body["items"][0]["credential"]["id"],
        Value::String(open_badges_id.clone())
    );

    bundle.handles.search_store.set_ready(false).await;

    let degraded_search = request_empty(&app, Method::GET, "/credentials/search?q=rust").await;
    assert_eq!(degraded_search.status(), StatusCode::SERVICE_UNAVAILABLE);

    let register_while_search_down = request_json(
        &app,
        Method::POST,
        "/credentials/register",
        fixture("register_credential/clr.json"),
    )
    .await;
    assert_eq!(register_while_search_down.status(), StatusCode::OK);

    let get_while_search_down = request_empty(
        &app,
        Method::GET,
        &format!("/credentials/{}", encode_credential_id(&open_badges_id)),
    )
    .await;
    assert_eq!(get_while_search_down.status(), StatusCode::OK);
}

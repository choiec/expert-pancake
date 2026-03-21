mod support;

use http::Method;
use http::StatusCode;

use support::{assert_contract_schema, fixture, request_json, response_json, test_app};

#[tokio::test]
async fn register_returns_authoritative_credential_contract_for_open_badges() {
    let (app, _) = test_app();
    let response = request_json(
        &app,
        Method::POST,
        "/credentials/register",
        fixture("register_credential/open_badges.json"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    assert_contract_schema("AuthoritativeCredential", &body);
    assert!(body.get("source_id").is_none());
    assert!(body.get("memory_items").is_none());
}

#[tokio::test]
async fn register_rejects_unsupported_top_level_fields_with_error_contract() {
    let (app, _) = test_app();
    let response = request_json(
        &app,
        Method::POST,
        "/credentials/register",
        fixture("register_credential/unsupported_top_level.json"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_contract_schema("ErrorResponse", &body);
}

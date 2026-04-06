mod support;

use core_shared::encode_credential_id;
use http::Method;
use http::StatusCode;

use support::{
    assert_contract_schema, fixture, request_empty, request_json, response_json, test_app,
};

#[tokio::test]
async fn get_returns_authoritative_credential_contract() {
    let (app, _) = test_app();
    let credential = fixture("register_credential/open_badges.json");
    let credential_id = credential["id"].as_str().unwrap().to_string();

    let response = request_json(&app, Method::POST, "/credentials/register", credential).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = request_empty(
        &app,
        Method::GET,
        &format!("/credentials/{}", encode_credential_id(&credential_id)),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_contract_schema("AuthoritativeCredential", &body);
}

#[tokio::test]
async fn get_returns_error_contract_for_missing_credential() {
    let (app, _) = test_app();
    let response = request_empty(
        &app,
        Method::GET,
        &format!(
            "/credentials/{}",
            encode_credential_id("urn:example:missing")
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert_contract_schema("ErrorResponse", &body);
}

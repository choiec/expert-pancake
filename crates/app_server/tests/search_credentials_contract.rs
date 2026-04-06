mod support;

use http::Method;
use http::StatusCode;

use support::{
    assert_contract_schema, fixture, request_empty, request_json, response_json, test_app,
};

#[tokio::test]
async fn search_returns_projection_contract() {
    let (app, _) = test_app();

    let response = request_json(
        &app,
        Method::POST,
        "/credentials/register",
        fixture("register_credential/open_badges.json"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = request_empty(&app, Method::GET, "/credentials/search?q=rust&limit=10").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_contract_schema("CredentialSearchResponse", &body);
}

#[tokio::test]
async fn search_returns_error_contract_when_projection_is_unavailable() {
    let (app, bundle) = test_app();
    bundle.handles.search_store.set_ready(false).await;

    let response = request_empty(&app, Method::GET, "/credentials/search?q=rust").await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = response_json(response).await;
    assert_contract_schema("ErrorResponse", &body);
}

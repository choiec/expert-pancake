mod support;

use http::Method;
use http::StatusCode;

use support::{assert_contract_schema, request_empty, response_json, test_app};

#[tokio::test]
async fn health_and_ready_follow_contract_shapes() {
    let (app, bundle) = test_app();

    let health = request_empty(&app, Method::GET, "/health").await;
    assert_eq!(health.status(), StatusCode::OK);
    let health_body = response_json(health).await;
    assert_contract_schema("HealthResponse", &health_body);

    let ready = request_empty(&app, Method::GET, "/ready").await;
    assert_eq!(ready.status(), StatusCode::OK);
    let ready_body = response_json(ready).await;
    assert_contract_schema("ReadinessResponse", &ready_body);

    bundle.handles.search_store.set_ready(false).await;
    let degraded = request_empty(&app, Method::GET, "/ready").await;
    assert_eq!(degraded.status(), StatusCode::OK);
    let degraded_body = response_json(degraded).await;
    assert_contract_schema("ReadinessResponse", &degraded_body);

    bundle.handles.authoritative_store.set_ready(false).await;
    let unavailable = request_empty(&app, Method::GET, "/ready").await;
    assert_eq!(unavailable.status(), StatusCode::SERVICE_UNAVAILABLE);
    let unavailable_body = response_json(unavailable).await;
    assert_contract_schema("ReadinessResponse", &unavailable_body);
}

#![allow(dead_code)]

use axum::Router;
use axum::body::Body;
use core_infra::{InfraBundle, build_infra_bundle};
use http::{Method, Request, Response};
use http_body_util::BodyExt;
use jsonschema::validator_for;
use serde_json::Value;
use tower::util::ServiceExt;

use app_server::{AppState, build_router};

pub fn test_app() -> (Router, InfraBundle) {
    let bundle = build_infra_bundle();
    let app = build_router(AppState::new(bundle.module.clone()));
    (app, bundle)
}

pub async fn request_json(app: &Router, method: Method, uri: &str, body: Value) -> Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request builds"),
        )
        .await
        .expect("request succeeds")
}

pub async fn request_empty(app: &Router, method: Method, uri: &str) -> Response<Body> {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("request succeeds")
}

pub async fn response_json(response: Response<Body>) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body bytes")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("response is valid JSON")
}

pub fn fixture(path: &str) -> Value {
    let full_path = format!("{}/tests/fixtures/{path}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(full_path).expect("fixture file exists");
    serde_json::from_slice(&bytes).expect("fixture is valid JSON")
}

pub fn assert_contract_schema(schema_name: &str, body: &Value) {
    let spec = load_openapi();
    if schema_name == "AuthoritativeCredential" {
        let candidates = ["OpenBadgesCredential", "ClrCredential"]
            .into_iter()
            .map(|name| resolve_component_schema(&spec, name))
            .collect::<Vec<_>>();

        let accepted = candidates.iter().any(|schema| {
            validator_for(schema)
                .expect("schema compiles")
                .iter_errors(body)
                .next()
                .is_none()
        });

        assert!(
            accepted,
            "schema `{schema_name}` rejected response body={}",
            serde_json::to_string_pretty(body).unwrap_or_default()
        );
        return;
    }

    let schema = resolve_component_schema(&spec, schema_name);
    let validator = validator_for(&schema).expect("schema compiles");
    let errors = validator
        .iter_errors(body)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();

    assert!(
        errors.is_empty(),
        "schema `{schema_name}` rejected response: {errors:#?}\nbody={}",
        serde_json::to_string_pretty(body).unwrap_or_default()
    );
}

fn load_openapi() -> Value {
    serde_yaml::from_str(include_str!(
        "../../../../specs/001-memory-ingest/contracts/memory-ingest.openapi.yaml"
    ))
    .expect("OpenAPI spec parses")
}

fn resolve_component_schema(spec: &Value, schema_name: &str) -> Value {
    let pointer = format!("/components/schemas/{schema_name}");
    let schema = spec
        .pointer(&pointer)
        .unwrap_or_else(|| panic!("schema `{schema_name}` exists"))
        .clone();

    resolve_refs(spec, &schema)
}

fn resolve_refs(spec: &Value, value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                let pointer = reference.strip_prefix("#").expect("local ref");
                let target = spec.pointer(pointer).expect("ref target exists");
                return resolve_refs(spec, target);
            }

            let mut resolved = serde_json::Map::new();
            for (key, value) in object {
                resolved.insert(key.clone(), resolve_refs(spec, value));
            }
            Value::Object(resolved)
        }
        Value::Array(items) => {
            Value::Array(items.iter().map(|item| resolve_refs(spec, item)).collect())
        }
        _ => value.clone(),
    }
}

#[path = "../support/mod.rs"]
mod support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use core_infra::surrealdb::InMemorySurrealDb;

use support::{assert_status_json, build_memory_ingest_app, load_fixture, send_json};

#[tokio::test]
async fn standard_validation_matrix_covers_accept_reject_paths() {
    struct Case<'a> {
        fixture: &'a str,
        external_id: &'a str,
        expected_status: StatusCode,
        expected_error_code: Option<&'a str>,
    }

    let cases = [
        Case {
            fixture: "register_source/validation_matrix/open_badges_accepted.json",
            external_id: "https://api.cherry-pick.net/ob/v2p0/issuer.example.org:urn%3Aexample%3Abadge%3Amatrix-accepted",
            expected_status: StatusCode::CREATED,
            expected_error_code: None,
        },
        Case {
            fixture: "register_source/validation_matrix/open_badges_schema_invalid.json",
            external_id: "urn:example:badge:matrix-invalid",
            expected_status: StatusCode::BAD_REQUEST,
            expected_error_code: Some("DIRECT_STANDARD_REJECTED_UNTRUSTED_DOMAIN"),
        },
        Case {
            fixture: "register_source/validation_matrix/open_badges_unmappable.json",
            external_id: "urn:example:badge:matrix-unmappable",
            expected_status: StatusCode::BAD_REQUEST,
            expected_error_code: Some("INVALID_STANDARD_PAYLOAD"),
        },
        Case {
            fixture: "register_source/validation_matrix/clr_accepted.json",
            external_id: "https://api.cherry-pick.net/clr/v2p0/issuer.example.org:https%3A%2F%2Fclr.example%2Fcredentials%2Fmatrix-accepted",
            expected_status: StatusCode::CREATED,
            expected_error_code: None,
        },
        Case {
            fixture: "register_source/validation_matrix/clr_schema_invalid.json",
            external_id: "https://clr.example/credentials/matrix-invalid",
            expected_status: StatusCode::BAD_REQUEST,
            expected_error_code: Some("DIRECT_STANDARD_REJECTED_UNTRUSTED_DOMAIN"),
        },
        Case {
            fixture: "register_source/validation_matrix/clr_unmappable.json",
            external_id: "https://clr.example/credentials/matrix-unmappable",
            expected_status: StatusCode::BAD_REQUEST,
            expected_error_code: Some("INVALID_STANDARD_PAYLOAD"),
        },
    ];

    for case in cases {
        let db = Arc::new(InMemorySurrealDb::new());
        let response = send_json(
            build_memory_ingest_app(db.clone()),
            Method::POST,
            "/sources/register",
            &load_fixture(case.fixture),
        )
        .await;
        let payload = assert_status_json(response, case.expected_status).await;

        if let Some(expected_error_code) = case.expected_error_code {
            assert_eq!(payload["error_code"], expected_error_code);
            assert!(db.lookup_source_by_external_id(case.external_id).is_none());
        } else {
            assert_eq!(payload["document_type"], "json");
            assert_eq!(payload["memory_items"][0]["unit_type"], "json_document");
            assert!(db.lookup_source_by_external_id(case.external_id).is_some());
        }
    }
}

use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use time::format_description::well_known::Rfc3339;

use crate::{middleware::error_response, state::AppState};

pub async fn register_source(State(state): State<AppState>, body: Bytes) -> Response {
    if body.len() > state.max_request_body_bytes() {
        return error_response(core_shared::AppError::payload_too_large(
            "request body exceeds 10 MB limit",
        ));
    }

    let raw_body = match String::from_utf8(body.to_vec()) {
        Ok(value) => value,
        Err(_) => {
            return error_response(core_shared::AppError::validation(
                "request body must be valid utf-8",
            ));
        }
    };

    match state.memory_ingest().register_source(&raw_body) {
        Ok(outcome) => {
            let source = outcome.source;
            let status = if outcome.created {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            };
            let created_at = source
                .created_at
                .format(&Rfc3339)
                .unwrap_or_else(|_| source.created_at.unix_timestamp().to_string());
            (
                status,
                axum::Json(json!({
                    "source_id": source.source_id,
                    "external_id": source.external_id,
                    "title": source.title,
                    "summary": source.summary,
                    "document_type": source.document_type,
                    "created_at": created_at,
                    "indexing_status": source.indexing_status,
                    "source_metadata": source.source_metadata,
                    "memory_items": source.memory_items.iter().map(|item| {
                        json!({
                            "urn": item.urn,
                            "sequence": item.sequence,
                            "unit_type": item.item_metadata["unit_type"],
                        })
                    }).collect::<Vec<_>>(),
                })),
            )
                .into_response()
        }
        Err(error) => error_response(error),
    }
}

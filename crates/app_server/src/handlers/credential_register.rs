use axum::Json;
use axum::extract::{State, rejection::JsonRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use core_shared::{ApiError, error_body};
use mod_memory::domain::credential::RegistrationStatus;

use crate::middleware::RequestId;
use crate::state::AppState;

pub async fn credential_register(
    State(state): State<AppState>,
    request_id: Option<axum::extract::Extension<RequestId>>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Response {
    let request_id = request_id
        .as_ref()
        .map(|id| id.0.0.clone())
        .unwrap_or_else(|| "unknown-request".to_string());

    let payload = match payload {
        Ok(Json(payload)) => payload,
        Err(JsonRejection::BytesRejection(_)) => {
            return api_error_response(
                &ApiError::payload_too_large(
                    "Credential payload exceeds the configured size limit",
                ),
                &request_id,
            );
        }
        Err(error) => {
            return api_error_response(
                &ApiError::invalid_input(
                    "Credential payload must be valid JSON",
                    Some(Value::String(error.to_string())),
                ),
                &request_id,
            );
        }
    };

    match state.module.register_service.register(payload).await {
        Ok(outcome) => {
            let status = match outcome.status {
                RegistrationStatus::Created => StatusCode::CREATED,
                RegistrationStatus::Replayed => StatusCode::OK,
            };

            (status, Json(outcome.credential.response_document())).into_response()
        }
        Err(error) => api_error_response(&error, &request_id),
    }
}

fn api_error_response(error: &ApiError, request_id: &str) -> Response {
    (error.status(), Json(error_body(error, request_id))).into_response()
}

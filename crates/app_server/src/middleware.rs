use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use core_shared::{AppError, ErrorKind};
use serde_json::json;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

pub fn error_response(error: AppError) -> Response {
    let status = match error.kind() {
        ErrorKind::Validation => StatusCode::BAD_REQUEST,
        ErrorKind::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        ErrorKind::Conflict => StatusCode::CONFLICT,
        ErrorKind::Timeout => StatusCode::REQUEST_TIMEOUT,
        ErrorKind::NotFound => StatusCode::NOT_FOUND,
        ErrorKind::StorageUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        ErrorKind::SearchUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        ErrorKind::Startup | ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    };

    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| OffsetDateTime::now_utc().unix_timestamp().to_string());

    (
        status,
        Json(json!({
            "error_code": error.error_code(),
            "message": error.message(),
            "details": error.details().cloned(),
            "timestamp": timestamp,
            "request_id": Uuid::new_v4().to_string(),
        })),
    )
        .into_response()
}

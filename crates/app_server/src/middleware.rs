use axum::{
    Json,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use core_shared::{AppError, ErrorKind};
use serde_json::json;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub traceparent: Option<String>,
}

pub async fn request_context(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let traceparent = request
        .headers()
        .get("traceparent")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    request.extensions_mut().insert(RequestContext {
        request_id: request_id.clone(),
        traceparent: traceparent.clone(),
    });

    let mut response = next.run(request).await;
    if let Ok(value) = axum::http::HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    if let Some(traceparent) = traceparent
        && let Ok(value) = axum::http::HeaderValue::from_str(&traceparent)
    {
        response.headers_mut().insert("traceparent", value);
    }
    response
}

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

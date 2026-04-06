use http::StatusCode;
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Clone)]
pub struct ApiError {
    status: StatusCode,
    error_code: &'static str,
    message: String,
    details: Option<Value>,
}

impl ApiError {
    pub fn new(
        status: StatusCode,
        error_code: &'static str,
        message: impl Into<String>,
        details: Option<Value>,
    ) -> Self {
        Self {
            status,
            error_code,
            message: message.into(),
            details,
        }
    }

    pub fn invalid_input(message: impl Into<String>, details: Option<Value>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "INVALID_INPUT", message, details)
    }

    pub fn conflict(message: impl Into<String>, details: Option<Value>) -> Self {
        Self::new(StatusCode::CONFLICT, "CONFLICT", message, details)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", message, None)
    }

    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "PAYLOAD_TOO_LARGE",
            message,
            None,
        )
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::REQUEST_TIMEOUT,
            "VALIDATION_TIMEOUT",
            message,
            None,
        )
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "SERVICE_UNAVAILABLE",
            message,
            None,
        )
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn error_code(&self) -> &'static str {
        self.error_code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn details(&self) -> Option<&Value> {
        self.details.as_ref()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub error_code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    pub timestamp: String,
    pub request_id: String,
}

pub fn error_body(error: &ApiError, request_id: &str) -> ErrorBody {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());

    ErrorBody {
        error_code: error.error_code().to_string(),
        message: error.message().to_string(),
        details: error.details().cloned(),
        timestamp,
        request_id: request_id.to_string(),
    }
}

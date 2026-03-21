use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorKind {
    Validation,
    PayloadTooLarge,
    Conflict,
    Timeout,
    NotFound,
    StorageUnavailable,
    SearchUnavailable,
    Startup,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppError {
    kind: ErrorKind,
    message: String,
    details: Option<Value>,
    error_code_override: Option<String>,
}

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: None,
            error_code_override: None,
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Validation, message)
    }

    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::PayloadTooLarge, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Conflict, message)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Timeout, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }

    pub fn storage_unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::StorageUnavailable, message)
    }

    pub fn search_unavailable(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::SearchUnavailable, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_error_code(mut self, error_code: impl Into<String>) -> Self {
        self.error_code_override = Some(error_code.into());
        self
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn details(&self) -> Option<&Value> {
        self.details.as_ref()
    }

    pub fn error_code(&self) -> &str {
        if let Some(error_code) = self.error_code_override.as_deref() {
            return error_code;
        }

        match self.kind {
            ErrorKind::Validation => "INVALID_INPUT",
            ErrorKind::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
            ErrorKind::Conflict => "EXTERNAL_ID_CONFLICT",
            ErrorKind::Timeout => "NORMALIZATION_TIMEOUT",
            ErrorKind::NotFound => "NOT_FOUND",
            ErrorKind::StorageUnavailable => "STORAGE_UNAVAILABLE",
            ErrorKind::SearchUnavailable => "SEARCH_UNAVAILABLE",
            ErrorKind::Startup => "STARTUP_FAILURE",
            ErrorKind::Internal => "INTERNAL_ERROR",
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartupError {
    #[error("missing required environment variable {key}")]
    MissingEnv { key: String },
    #[error("invalid environment variable {key}: {reason}")]
    InvalidEnv {
        key: String,
        value: String,
        reason: String,
    },
}

impl From<StartupError> for AppError {
    fn from(value: StartupError) -> Self {
        AppError::new(ErrorKind::Startup, value.to_string())
    }
}

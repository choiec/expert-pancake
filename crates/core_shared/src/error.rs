use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub type CoreResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorKind {
    Validation,
    Conflict,
    Timeout,
    NotFound,
    StorageUnavailable,
    SearchDegraded,
    Startup,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppError {
    kind: ErrorKind,
    message: String,
    details: Option<Value>,
}

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Validation, message)
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

    pub fn search_degraded(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::SearchDegraded, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    pub fn startup(error: StartupError) -> Self {
        Self::new(ErrorKind::Startup, error.to_string()).with_details(error.details())
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

    pub fn error_code(&self) -> &'static str {
        match self.kind {
            ErrorKind::Validation => "INVALID_INPUT",
            ErrorKind::Conflict => "EXTERNAL_ID_CONFLICT",
            ErrorKind::Timeout => "NORMALIZATION_TIMEOUT",
            ErrorKind::NotFound => "NOT_FOUND",
            ErrorKind::StorageUnavailable => "STORAGE_UNAVAILABLE",
            ErrorKind::SearchDegraded => "SEARCH_UNAVAILABLE",
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
    #[error("failed to bootstrap {component}: {reason}")]
    InfraBootstrap { component: String, reason: String },
    #[error("failed to bind HTTP listener on {address}: {reason}")]
    ServerBind { address: String, reason: String },
    #[error("failed to start HTTP server: {reason}")]
    ServerStart { reason: String },
}

impl StartupError {
    pub fn details(&self) -> Value {
        match self {
            StartupError::MissingEnv { key } => {
                serde_json::json!({ "key": key })
            }
            StartupError::InvalidEnv { key, value, reason } => serde_json::json!({
                "key": key,
                "value": value,
                "reason": reason,
            }),
            StartupError::InfraBootstrap { component, reason } => serde_json::json!({
                "component": component,
                "reason": reason,
            }),
            StartupError::ServerBind { address, reason } => serde_json::json!({
                "address": address,
                "reason": reason,
            }),
            StartupError::ServerStart { reason } => serde_json::json!({
                "reason": reason,
            }),
        }
    }
}

impl From<StartupError> for AppError {
    fn from(value: StartupError) -> Self {
        Self::startup(value)
    }
}

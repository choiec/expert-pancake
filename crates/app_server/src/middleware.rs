use std::{borrow::Cow, time::Instant};

use axum::{
    Json,
    extract::{MatchedPath, Request, State},
    http::{HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use core_shared::{AppError, ErrorKind};
use serde::Serialize;
use tracing::Instrument;
use uuid::Uuid;

use crate::state::{AppState, MetricKey, MetricsLabels};

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");
const TRACEPARENT_HEADER: HeaderName = HeaderName::from_static("traceparent");
const TRACESTATE_HEADER: HeaderName = HeaderName::from_static("tracestate");

#[derive(Debug, Clone)]
pub struct RequestContext {
    request_id: String,
    traceparent: Option<String>,
    tracestate: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    error_code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
    timestamp: String,
    request_id: String,
}

pub async fn request_context(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let traceparent = request
        .headers()
        .get(&TRACEPARENT_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let tracestate = request
        .headers()
        .get(&TRACESTATE_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);

    let request_path = request.uri().path().to_string();
    let method = request.method().clone();

    let context = RequestContext {
        request_id: request_id.clone(),
        traceparent: traceparent.clone(),
        tracestate: tracestate.clone(),
    };
    request.extensions_mut().insert(context.clone());

    let trace_id = traceparent
        .as_deref()
        .map(extract_trace_id)
        .unwrap_or_else(|| Cow::Borrowed(""));

    let span = tracing::info_span!(
        "http.request",
        request_id = %request_id,
        method = %method,
        path = %request_path,
        trace_id = %trace_id,
    );

    let mut response = next.run(request).instrument(span).await;
    response.headers_mut().insert(
        REQUEST_ID_HEADER.clone(),
        HeaderValue::from_str(&request_id).expect("generated request id is valid"),
    );

    if let Some(traceparent) = traceparent {
        if let Ok(value) = HeaderValue::from_str(&traceparent) {
            response
                .headers_mut()
                .insert(TRACEPARENT_HEADER.clone(), value);
        }
    }

    if let Some(tracestate) = tracestate {
        if let Ok(value) = HeaderValue::from_str(&tracestate) {
            response
                .headers_mut()
                .insert(TRACESTATE_HEADER.clone(), value);
        }
    }

    response
}

pub async fn latency_metrics(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| matched.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());
    let method = request.method().to_string();
    let started_at = Instant::now();

    let response = next.run(request).await;

    // Future handlers can enrich this with bounded labels by writing a
    // `MetricsLabels` value into the response extensions before returning.
    let labels = response
        .extensions()
        .get::<MetricsLabels>()
        .cloned()
        .unwrap_or_default();

    state.request_metrics().record(
        MetricKey {
            method,
            route,
            status_code: response.status().as_u16(),
            document_type: labels.document_type,
            ingest_kind: labels.ingest_kind,
            decision_reason: labels.decision_reason,
        },
        started_at.elapsed(),
    );

    response
}

pub fn map_app_error(error: AppError, context: &RequestContext) -> Response {
    let status = match error.kind() {
        ErrorKind::Validation => StatusCode::BAD_REQUEST,
        ErrorKind::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        ErrorKind::Conflict => StatusCode::CONFLICT,
        ErrorKind::Timeout => StatusCode::REQUEST_TIMEOUT,
        ErrorKind::NotFound => StatusCode::NOT_FOUND,
        ErrorKind::StorageUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        ErrorKind::SearchDegraded => StatusCode::SERVICE_UNAVAILABLE,
        ErrorKind::Startup | ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    };

    let payload = ErrorPayload {
        error_code: error.error_code().to_owned(),
        message: error.message().to_string(),
        details: error.details().cloned(),
        timestamp: chrono_like_timestamp(),
        request_id: context.request_id().to_string(),
    };

    (status, Json(payload)).into_response()
}

impl RequestContext {
    pub fn fallback() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            traceparent: None,
            tracestate: None,
        }
    }

    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    pub fn traceparent(&self) -> Option<&str> {
        self.traceparent.as_deref()
    }

    pub fn tracestate(&self) -> Option<&str> {
        self.tracestate.as_deref()
    }
}

fn extract_trace_id(traceparent: &str) -> Cow<'_, str> {
    let mut parts = traceparent.split('-');
    let _version = parts.next();
    parts
        .next()
        .map(|value| Cow::Owned(value.to_string()))
        .unwrap_or_else(|| Cow::Borrowed(""))
}

fn chrono_like_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after unix epoch");

    format!("{}.{:03}Z", now.as_secs(), now.subsec_millis())
}

use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use core_shared::AppError;
use mod_memory::{
    application::register_source::RegisterSourceCommand,
    domain::{
        normalization::{normalized_json_hash_from_str, raw_body_hash_from_str},
        source::{DocumentType, IngestKind},
        source_external_id::{CanonicalSourceExternalId, canonicalize_direct_standard_payload},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::{
    middleware::{RequestContext, map_app_error},
    state::{AppState, MetricsLabels},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/sources/register", post(register_source))
}

#[derive(Debug, Deserialize)]
struct CanonicalRegisterRequest {
    title: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(rename = "external-id")]
    external_id: String,
    #[serde(rename = "document-type")]
    document_type: CanonicalDocumentType,
    content: String,
    #[serde(default)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CanonicalDocumentType {
    Text,
    Markdown,
}

#[derive(Debug, Serialize)]
struct RegisterSourceResponse {
    source_id: String,
    external_id: String,
    document_type: String,
    indexing_status: String,
    source_metadata: Value,
    memory_items: Vec<MemoryItemSummary>,
}

#[derive(Debug, Serialize)]
struct MemoryItemSummary {
    urn: String,
    sequence: u32,
    unit_type: String,
}

async fn register_source(State(state): State<AppState>, request: Request) -> Response {
    let context = request
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(RequestContext::fallback);

    match register_source_inner(state, request, context.clone()).await {
        Ok(response) => response,
        Err(error) => map_app_error(error, &context),
    }
}

async fn register_source_inner(
    state: AppState,
    request: Request,
    context: RequestContext,
) -> Result<Response, AppError> {
    ensure_json_content_type(request.headers())?;

    let body = to_bytes(request.into_body(), state.max_request_body_bytes())
        .await
        .map_err(map_body_error)?;
    let raw_body = String::from_utf8(body.to_vec())
        .map_err(|_| AppError::validation("request body must be valid UTF-8 JSON"))?;
    let value: Value = serde_json::from_str(&raw_body)
        .map_err(|error| AppError::validation(format!("invalid JSON payload: {error}")))?;

    let (command, ingest_kind) = canonicalize_request(&value, &raw_body)?;
    let result = state
        .memory_ingest()
        .ok_or_else(|| AppError::storage_unavailable("memory ingest services are not configured"))?
        .register_source()
        .execute(command)
        .await?;

    let payload = RegisterSourceResponse {
        source_id: result.source_id.to_string(),
        external_id: result.external_id,
        document_type: result.document_type.as_str().to_owned(),
        indexing_status: result.indexing_status.as_str().to_owned(),
        source_metadata: result.source_metadata,
        memory_items: result
            .memory_items
            .into_iter()
            .map(|item| MemoryItemSummary {
                urn: item.urn,
                sequence: item.sequence,
                unit_type: item.unit_type,
            })
            .collect(),
    };
    let status = if result.replayed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };

    tracing::info!(
        request_id = %context.request_id(),
        trace_id = %extract_trace_id(context.traceparent().unwrap_or_default()),
        handler = "source_register",
        route = "/sources/register",
        method = "POST",
        source_id = %payload.source_id,
        canonical_external_id = %payload.external_id,
        original_standard_id = ?payload.source_metadata.pointer("/system/original_standard_id").and_then(|value| value.as_str()),
        canonical_id_version = ?payload.source_metadata.pointer("/system/canonical_id_version").and_then(|value| value.as_str()),
        semantic_payload_hash = ?payload.source_metadata.pointer("/system/semantic_payload_hash").and_then(|value| value.as_str()),
        raw_body_hash_present = false,
        decision_reason = %result.decision_reason,
        ingest_kind = %ingest_kind.as_str(),
        "register_source completed"
    );

    let mut response = (status, Json(payload)).into_response();
    MetricsLabels::new()
        .with_document_type(result.document_type.as_str())
        .with_ingest_kind(match ingest_kind {
            IngestKind::Canonical => "canonical",
            IngestKind::DirectStandard => "direct_standard",
        })
        .with_decision_reason(&result.decision_reason)
        .insert_response_extension(&mut response);
    Ok(response)
}

fn canonicalize_request(
    value: &Value,
    raw_body: &str,
) -> Result<(RegisterSourceCommand, IngestKind), AppError> {
    if is_canonical_shape(value) {
        let request: CanonicalRegisterRequest = serde_json::from_value(value.clone())
            .map_err(|error| AppError::validation(format!("invalid canonical request: {error}")))?;

        let title = request.title.trim().to_owned();
        let external_id = CanonicalSourceExternalId::parse_canonical_uri(request.external_id.trim())?
            .canonical_uri();
        if title.is_empty() {
            return Err(AppError::validation("title is required"));
        }

        return Ok((
            RegisterSourceCommand {
                external_id,
                title,
                summary: request.summary.and_then(trimmed_option),
                document_type: match request.document_type {
                    CanonicalDocumentType::Text => DocumentType::Text,
                    CanonicalDocumentType::Markdown => DocumentType::Markdown,
                },
                authoritative_content: request.content,
                source_metadata: request.metadata.unwrap_or_else(empty_object),
                semantic_payload_hash: normalized_json_hash_from_str(raw_body)?,
                original_standard_id: None,
                raw_body_hash: None,
                ingest_kind: IngestKind::Canonical,
            },
            IngestKind::Canonical,
        ));
    }

    let standard = canonicalize_direct_standard_payload(value)?;
    Ok((
        RegisterSourceCommand {
            external_id: standard.external_id.canonical_uri(),
            title: standard.title,
            summary: None,
            document_type: DocumentType::Json,
            authoritative_content: raw_body.to_owned(),
            source_metadata: json!({}),
            semantic_payload_hash: normalized_json_hash_from_str(raw_body)?,
            original_standard_id: Some(standard.original_standard_id),
            raw_body_hash: Some(raw_body_hash_from_str(raw_body)),
            ingest_kind: IngestKind::DirectStandard,
        },
        IngestKind::DirectStandard,
    ))
}

fn trimmed_option(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

fn is_canonical_shape(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };

    object.contains_key("document-type")
        || object.contains_key("external-id")
        || object.contains_key("content")
}

fn extract_trace_id(traceparent: &str) -> String {
    traceparent.split('-').nth(1).unwrap_or_default().to_owned()
}

fn ensure_json_content_type(headers: &HeaderMap) -> Result<(), AppError> {
    let Some(value) = headers.get(header::CONTENT_TYPE) else {
        return Ok(());
    };
    let content_type = value
        .to_str()
        .map_err(|_| AppError::validation("content-type header must be valid UTF-8"))?;

    if content_type
        .split(';')
        .next()
        .map(str::trim)
        .is_some_and(|mime| mime.eq_ignore_ascii_case("application/json"))
    {
        Ok(())
    } else {
        Err(AppError::validation(
            "content-type must be application/json",
        ))
    }
}

fn map_body_error(error: axum::Error) -> AppError {
    let message = error.to_string();
    if message.to_ascii_lowercase().contains("length limit") {
        AppError::payload_too_large("request payload exceeds the 10 MB ingest limit")
    } else {
        AppError::validation(format!("failed to read request body: {message}"))
    }
}

#[cfg(test)]
mod tests {
    use mod_memory::domain::source_external_id::{
        DirectStandardProfile, canonicalize_direct_standard_payload,
    };
    use serde_json::json;

    #[test]
    fn classifies_open_badges_payloads() {
        let payload = json!({
            "@context": ["https://www.w3.org/ns/credentials/v2"],
            "type": ["VerifiableCredential", "OpenBadgeCredential"],
            "id": " urn:badge:1 ",
            "name": " Rust Badge "
        });

        let standard = canonicalize_direct_standard_payload(&payload)
            .expect("open badges payload should canonicalize");

        assert_eq!(
            standard.profile,
            DirectStandardProfile::OpenBadgesAchievementCredential
        );
        assert_eq!(
            standard.external_id.canonical_uri(),
            "https://api.cherry-pick.net/ob/v2p0/issuer.example.org:urn%3Abadge%3A1"
        );
        assert_eq!(standard.original_standard_id, "urn:badge:1");
        assert_eq!(standard.title, "Rust Badge");
    }

    #[test]
    fn rejects_shape_valid_but_unmappable_standard_payloads() {
        let payload = json!({
            "@context": ["https://www.w3.org/ns/credentials/v2"],
            "type": ["VerifiableCredential"],
            "id": "urn:example:1",
            "name": "Example"
        });

        let error = canonicalize_direct_standard_payload(&payload)
            .expect_err("unsupported family must fail");

        assert_eq!(error.error_code(), "INVALID_STANDARD_PAYLOAD");
    }
}

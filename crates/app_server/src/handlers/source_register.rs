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
        normalization::normalized_json_hash_from_str,
        source::{DocumentType, IngestKind},
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
    memory_items: Vec<MemoryItemSummary>,
}

#[derive(Debug, Serialize)]
struct MemoryItemSummary {
    urn: String,
    sequence: u32,
    unit_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StandardFamily {
    OpenBadges,
    Clr,
}

async fn register_source(State(state): State<AppState>, request: Request) -> Response {
    let context = request
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(RequestContext::fallback);

    match register_source_inner(state, request).await {
        Ok(response) => response,
        Err(error) => map_app_error(error, &context),
    }
}

async fn register_source_inner(state: AppState, request: Request) -> Result<Response, AppError> {
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

    let mut response = (status, Json(payload)).into_response();
    MetricsLabels::new()
        .with_document_type(result.document_type.as_str())
        .with_ingest_kind(match ingest_kind {
            IngestKind::Canonical => "canonical",
            IngestKind::DirectStandard => "direct_standard",
        })
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
        let external_id = request.external_id.trim().to_owned();
        if title.is_empty() {
            return Err(AppError::validation("title is required"));
        }
        if external_id.is_empty() {
            return Err(AppError::validation("external-id is required"));
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
                canonical_payload_hash: normalized_json_hash_from_str(raw_body)?,
                ingest_kind: IngestKind::Canonical,
            },
            IngestKind::Canonical,
        ));
    }

    let standard = canonicalize_standard_payload(value, raw_body)?;
    let _family = standard.family;
    Ok((
        RegisterSourceCommand {
            external_id: standard.external_id,
            title: standard.title,
            summary: None,
            document_type: DocumentType::Json,
            authoritative_content: raw_body.to_owned(),
            source_metadata: json!({}),
            canonical_payload_hash: normalized_json_hash_from_str(raw_body)?,
            ingest_kind: IngestKind::DirectStandard,
        },
        IngestKind::DirectStandard,
    ))
}

#[derive(Debug)]
struct CanonicalizedStandard {
    family: StandardFamily,
    external_id: String,
    title: String,
}

fn canonicalize_standard_payload(
    value: &Value,
    _raw_body: &str,
) -> Result<CanonicalizedStandard, AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::validation("request body must be a JSON object"))?;

    let _contexts = string_or_string_array(object, "@context")?;
    let types = string_or_string_array(object, "type")?;
    let external_id = required_trimmed_string(object, "id")?;
    let title = required_trimmed_string(object, "name")?;
    let family = classify_standard_family(object, &types)?;

    Ok(CanonicalizedStandard {
        family,
        external_id,
        title,
    })
}

fn classify_standard_family(
    object: &Map<String, Value>,
    types: &[String],
) -> Result<StandardFamily, AppError> {
    let has_badge_marker = types.iter().any(|value| {
        matches!(
            value.as_str(),
            "OpenBadgeCredential" | "AchievementCredential"
        )
    });
    let has_clr_marker = object.get("credentialSubject").is_some()
        || types
            .iter()
            .any(|value| matches!(value.as_str(), "ClrCredential" | "CLRCredential"));

    if has_clr_marker {
        return Ok(StandardFamily::Clr);
    }
    if has_badge_marker {
        return Ok(StandardFamily::OpenBadges);
    }

    Err(AppError::validation(
        "supported-standard payload could not be mapped to a supported family",
    )
    .with_error_code("INVALID_STANDARD_PAYLOAD"))
}

fn string_or_string_array(
    object: &Map<String, Value>,
    field: &str,
) -> Result<Vec<String>, AppError> {
    let value = object
        .get(field)
        .ok_or_else(|| AppError::validation(format!("{field} is required")))?;

    match value {
        Value::String(single) => Ok(vec![single.clone()]),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::String(item) => Ok(item.clone()),
                _ => Err(AppError::validation(format!(
                    "{field} entries must be strings"
                ))),
            })
            .collect(),
        _ => Err(AppError::validation(format!(
            "{field} must be a string or array of strings"
        ))),
    }
}

fn required_trimmed_string(object: &Map<String, Value>, field: &str) -> Result<String, AppError> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::validation(format!("{field} must be a string")))?;
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(AppError::validation(format!("{field} must not be empty"))
            .with_error_code("INVALID_STANDARD_PAYLOAD"));
    }
    Ok(trimmed)
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
    use super::{StandardFamily, canonicalize_standard_payload};
    use serde_json::json;

    #[test]
    fn classifies_open_badges_payloads() {
        let payload = json!({
            "@context": ["https://www.w3.org/ns/credentials/v2"],
            "type": ["VerifiableCredential", "OpenBadgeCredential"],
            "id": " urn:badge:1 ",
            "name": " Rust Badge "
        });

        let standard = canonicalize_standard_payload(&payload, "{}")
            .expect("open badges payload should canonicalize");

        assert_eq!(standard.family, StandardFamily::OpenBadges);
        assert_eq!(standard.external_id, "urn:badge:1");
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

        let error = canonicalize_standard_payload(&payload, "{}")
            .expect_err("unsupported family must fail");

        assert_eq!(error.error_code(), "INVALID_STANDARD_PAYLOAD");
    }
}

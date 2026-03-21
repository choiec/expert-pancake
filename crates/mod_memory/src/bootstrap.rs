use std::{sync::Arc, time::Duration};

use core_infra::surrealdb::{
    CommitRegistrationOutcome, InMemorySurrealDb, PersistedMemoryItemRecord, PersistedSourceRecord,
    SearchProjectionRecord,
};
use core_shared::{AppError, AppResult, DefaultIdGenerator, IdGenerator};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::{
    memory_item::MemoryItem,
    normalization::{NormalizationInput, normalize_source, normalized_json_hash, sha256_hex},
    source::{
        CANONICAL_ID_VERSION, CanonicalSource, DocumentType, StandardFamily,
        canonical_external_id_for_standard, derive_source_id, is_canonical_external_id,
    },
};

#[derive(Clone)]
pub struct MemoryModule {
    db: Arc<InMemorySurrealDb>,
    id_generator: Arc<dyn IdGenerator>,
    normalization_timeout: Duration,
}

impl std::fmt::Debug for MemoryModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryModule").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSourcePayload {
    pub title: String,
    pub summary: Option<String>,
    pub external_id: String,
    pub document_type: String,
    pub content: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSourceOutcome {
    pub created: bool,
    pub source: SourceView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceView {
    pub source_id: Uuid,
    pub external_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub document_type: String,
    pub created_at: OffsetDateTime,
    pub indexing_status: String,
    pub source_metadata: Value,
    pub memory_items: Vec<MemoryItemView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItemView {
    pub urn: String,
    pub source_id: Uuid,
    pub sequence: u32,
    pub content: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub item_metadata: Value,
    pub source_metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub source_id: Option<Uuid>,
    pub document_type: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHitView {
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
    pub items: Vec<SearchProjectionRecord>,
}

impl MemoryModule {
    pub fn fixture(db: Arc<InMemorySurrealDb>, normalization_timeout: Duration) -> Self {
        Self {
            db,
            id_generator: Arc::new(DefaultIdGenerator),
            normalization_timeout,
        }
    }

    pub fn register_source(&self, raw_body: &str) -> AppResult<RegisterSourceOutcome> {
        let started_at = OffsetDateTime::now_utc();
        let source = parse_register_payload(raw_body)?;
        let source_id = derive_source_id(&source.external_id);
        let normalized = normalize_source(
            &NormalizationInput {
                source_id,
                external_id: &source.external_id,
                title: &source.title,
                summary: source.summary.as_deref(),
                document_type: source.document_type,
                authoritative_content: &source.content,
                source_metadata: &source.metadata,
                created_at: started_at,
            },
            self.id_generator.as_ref(),
        )?;

        let timeout = self.normalization_timeout;
        if timeout.as_secs() == 0 {
            return Err(AppError::timeout("normalization timed out"));
        }

        let persisted_source = PersistedSourceRecord {
            source_id,
            external_id: source.external_id.clone(),
            title: source.title,
            summary: source.summary,
            document_type: source.document_type.as_str().to_owned(),
            source_metadata: source.metadata,
            canonical_hash: source.canonical_hash,
            created_at: started_at,
            updated_at: started_at,
        };
        let persisted_items = normalized
            .into_iter()
            .map(into_persisted_memory_item)
            .collect();

        let committed = self
            .db
            .commit_registration(persisted_source, persisted_items)?;
        match committed {
            CommitRegistrationOutcome::Created(bundle) => Ok(RegisterSourceOutcome {
                created: true,
                source: bundle_to_source_view(bundle),
            }),
            CommitRegistrationOutcome::Replay(bundle) => Ok(RegisterSourceOutcome {
                created: false,
                source: bundle_to_source_view(bundle),
            }),
        }
    }

    pub fn get_source(&self, source_id: Uuid) -> AppResult<SourceView> {
        let bundle = self
            .db
            .get_source_bundle(source_id)
            .ok_or_else(|| AppError::not_found("source was not found"))?;
        Ok(bundle_to_source_view(bundle))
    }

    pub fn get_memory_item(&self, urn: &str) -> AppResult<MemoryItemView> {
        let (item, source, _) = self
            .db
            .get_memory_item(urn)
            .ok_or_else(|| AppError::not_found("memory item was not found"))?;
        Ok(memory_item_view(item, &source.source_metadata))
    }

    pub fn search_memory_items(&self, query: SearchQuery) -> AppResult<SearchHitView> {
        let (total, items) = self.db.search(
            query.q.as_deref(),
            query.source_id,
            query.document_type.as_deref(),
            query.limit,
            query.offset,
        )?;
        Ok(SearchHitView {
            total,
            limit: query.limit,
            offset: query.offset,
            items,
        })
    }
}

fn parse_register_payload(raw_body: &str) -> AppResult<CanonicalSource> {
    let value: Value = serde_json::from_str(raw_body)
        .map_err(|_| AppError::validation("request body must be valid json"))?;

    if value.get("external-id").is_some()
        || value.get("document-type").is_some()
        || value.get("content").is_some()
    {
        let title = required_string(&value, "title")?;
        let external_id = required_string(&value, "external-id")?;
        let document_type = match required_string(&value, "document-type")?.as_str() {
            "text" => DocumentType::Text,
            "markdown" => DocumentType::Markdown,
            other => {
                return Err(AppError::validation(format!(
                    "unsupported document-type '{other}'"
                )));
            }
        };
        let content = required_string(&value, "content")?;
        let summary = optional_string(&value, "summary");
        if !is_canonical_external_id(&external_id) {
            return Err(AppError::validation(
                "canonical/manual ingest requires a canonical project-owned external-id",
            ));
        }
        let user_metadata = metadata_object(&value)?;
        let canonical_hash = sha256_hex(&canonical_json_shape(&json!({
            "title": &title,
            "summary": &summary,
            "external-id": &external_id,
            "document-type": document_type.as_str(),
            "content": &content,
            "metadata": &user_metadata,
        })));
        return Ok(CanonicalSource {
            title,
            summary,
            external_id,
            document_type,
            content,
            metadata: attach_system_metadata(user_metadata, "canonical", &canonical_hash, None),
            canonical_hash,
        });
    }

    let title = required_string(&value, "name")?;
    let original_standard_id = required_string(&value, "id")?;
    let family = classify_standard_payload(&value)?;
    let source_domain = trusted_source_domain(&value, &original_standard_id)?;
    let external_id =
        canonical_external_id_for_standard(family, &source_domain, &original_standard_id);
    let canonical_hash = normalized_json_hash(raw_body)?;

    Ok(CanonicalSource {
        title,
        summary: None,
        external_id,
        document_type: DocumentType::Json,
        content: raw_body.to_owned(),
        metadata: attach_system_metadata(
            json!({}),
            "direct_standard",
            &canonical_hash,
            Some(original_standard_id),
        ),
        canonical_hash,
    })
}

fn classify_standard_payload(value: &Value) -> AppResult<StandardFamily> {
    let context = value
        .get("@context")
        .ok_or_else(|| AppError::validation("supported-standard payload must include @context"))?;
    let kind = value
        .get("type")
        .ok_or_else(|| AppError::validation("supported-standard payload must include type"))?;

    let context_text = context.to_string().to_lowercase();
    let kind_text = kind.to_string().to_lowercase();
    if context_text.contains("openbadges")
        || kind_text.contains("achievementcredential")
        || kind_text.contains("openbadge")
    {
        Ok(StandardFamily::OpenBadges)
    } else if context_text.contains("clr")
        || kind_text.contains("clr")
        || kind_text.contains("clrcredential")
    {
        Ok(StandardFamily::Clr)
    } else if context_text.contains("openbadges")
        || kind_text.contains("openbadge")
        || kind_text.contains("achievementcredential")
    {
        Ok(StandardFamily::OpenBadges)
    } else if value.get("issuer").is_some() {
        Ok(StandardFamily::Clr)
    } else {
        Err(
            AppError::validation("supported-standard payload is shape-valid but unmappable")
                .with_error_code("INVALID_STANDARD_PAYLOAD"),
        )
    }
}

fn trusted_source_domain(value: &Value, original_standard_id: &str) -> AppResult<String> {
    if let Some(issuer_id) = value
        .get("issuer")
        .and_then(|issuer| issuer.get("id").or(Some(issuer)))
        .and_then(Value::as_str)
        .and_then(extract_host)
    {
        return Ok(issuer_id);
    }

    extract_host(original_standard_id).ok_or_else(|| {
        AppError::validation("supported-standard payload is shape-valid but unmappable")
            .with_error_code("INVALID_STANDARD_PAYLOAD")
    })
}

fn required_string(value: &Value, key: &str) -> AppResult<String> {
    let field = value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::validation(format!("'{key}' is required")))?;
    Ok(field.to_owned())
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn metadata_object(value: &Value) -> AppResult<Value> {
    match value.get("metadata") {
        Some(Value::Object(_)) => Ok(value.get("metadata").cloned().unwrap_or_else(|| json!({}))),
        Some(_) => Err(AppError::validation("'metadata' must be an object")),
        None => Ok(json!({})),
    }
}

fn attach_system_metadata(
    metadata: Value,
    ingest_kind: &str,
    semantic_payload_hash: &str,
    original_standard_id: Option<String>,
) -> Value {
    let mut metadata = match metadata {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };

    metadata.insert(
        "system".to_owned(),
        json!({
            "canonical_id_version": CANONICAL_ID_VERSION,
            "ingest_kind": ingest_kind,
            "semantic_payload_hash": semantic_payload_hash,
            "original_standard_id": original_standard_id,
        }),
    );

    Value::Object(metadata)
}

fn extract_host(value: &str) -> Option<String> {
    value.parse::<http::Uri>().ok()?.host().map(str::to_owned)
}

fn canonical_json_shape(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value).unwrap_or_default(),
        Value::Array(values) => {
            let inner = values
                .iter()
                .map(canonical_json_shape)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{inner}]")
        }
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            let inner = entries
                .into_iter()
                .map(|(key, value)| {
                    let key = serde_json::to_string(key).unwrap_or_default();
                    format!("{key}:{}", canonical_json_shape(value))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{inner}}}")
        }
    }
}

fn into_persisted_memory_item(item: MemoryItem) -> PersistedMemoryItemRecord {
    PersistedMemoryItemRecord {
        urn: item.urn,
        source_id: item.source_id,
        sequence: item.sequence,
        unit_type: item.unit_type.as_str().to_owned(),
        start_offset: item.start_offset,
        end_offset: item.end_offset,
        version: item.version,
        content: item.content,
        content_hash: item.content_hash,
        item_metadata: item.item_metadata,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

fn bundle_to_source_view(bundle: core_infra::surrealdb::PersistedSourceBundle) -> SourceView {
    SourceView {
        source_id: bundle.source.source_id,
        external_id: bundle.source.external_id.clone(),
        title: bundle.source.title.clone(),
        summary: bundle.source.summary.clone(),
        document_type: bundle.source.document_type.clone(),
        created_at: bundle.source.created_at,
        indexing_status: bundle.indexing_status,
        source_metadata: bundle.source.source_metadata.clone(),
        memory_items: bundle
            .memory_items
            .into_iter()
            .map(|item| memory_item_view(item, &bundle.source.source_metadata))
            .collect(),
    }
}

fn memory_item_view(item: PersistedMemoryItemRecord, source_metadata: &Value) -> MemoryItemView {
    MemoryItemView {
        urn: item.urn,
        source_id: item.source_id,
        sequence: item.sequence,
        content: item.content,
        created_at: item.created_at,
        updated_at: item.updated_at,
        item_metadata: item.item_metadata,
        source_metadata: source_metadata.clone(),
    }
}

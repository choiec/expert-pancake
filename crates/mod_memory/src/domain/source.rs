use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use core_shared::{AppError, AppResult};

pub const CANONICAL_ID_VERSION: &str = "v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Text,
    Markdown,
    Json,
}

impl DocumentType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Markdown => "markdown",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestKind {
    Canonical,
    DirectStandard,
}

impl IngestKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::DirectStandard => "direct_standard",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSystemMetadata {
    pub canonical_id_version: String,
    pub ingest_kind: IngestKind,
    pub semantic_payload_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_standard_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_body_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub source_id: Uuid,
    pub external_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub document_type: DocumentType,
    pub source_metadata: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSource {
    pub source_id: Uuid,
    pub external_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub document_type: DocumentType,
    pub source_metadata: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl NewSource {
    pub fn new(
        source_id: Uuid,
        external_id: String,
        title: String,
        summary: Option<String>,
        document_type: DocumentType,
        source_metadata: Value,
        system: SourceSystemMetadata,
        created_at: OffsetDateTime,
    ) -> AppResult<Self> {
        let mut metadata_object = match source_metadata {
            Value::Null => Map::new(),
            Value::Object(map) => map,
            _ => {
                return Err(AppError::validation(
                    "source_metadata must be a JSON object when provided",
                ));
            }
        };
        metadata_object.remove("system");
        metadata_object.insert("system".to_owned(), json!(system));

        Ok(Self {
            source_id,
            external_id,
            title,
            summary,
            document_type,
            source_metadata: Value::Object(metadata_object),
            created_at,
            updated_at: created_at,
        })
    }

    pub fn semantic_payload_hash(&self) -> &str {
        self.source_metadata
            .pointer("/system/semantic_payload_hash")
            .and_then(Value::as_str)
            .unwrap_or_default()
    }
}

impl Source {
    pub fn public_source_metadata(&self) -> Value {
        public_source_metadata(&self.source_metadata)
    }

    pub fn semantic_payload_hash(&self) -> &str {
        self.source_metadata
            .pointer("/system/semantic_payload_hash")
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    pub fn canonical_id_version(&self) -> &str {
        self.source_metadata
            .pointer("/system/canonical_id_version")
            .and_then(Value::as_str)
            .unwrap_or(CANONICAL_ID_VERSION)
    }

    pub fn original_standard_id(&self) -> Option<&str> {
        self.source_metadata
            .pointer("/system/original_standard_id")
            .and_then(Value::as_str)
    }
}

pub fn public_source_metadata(source_metadata: &Value) -> Value {
    let mut metadata_object = match source_metadata {
        Value::Null => Map::new(),
        Value::Object(map) => map.clone(),
        _ => return Value::Object(Map::new()),
    };

    if let Some(Value::Object(system)) = metadata_object.get_mut("system") {
        system.remove("raw_body_hash");
    }

    Value::Object(metadata_object)
}

impl SourceSystemMetadata {
    pub fn new(
        ingest_kind: IngestKind,
        semantic_payload_hash: String,
        original_standard_id: Option<String>,
        raw_body_hash: Option<String>,
    ) -> AppResult<Self> {
        if semantic_payload_hash.trim().is_empty() {
            return Err(AppError::validation("semantic_payload_hash is required"));
        }

        Ok(Self {
            canonical_id_version: CANONICAL_ID_VERSION.to_owned(),
            ingest_kind,
            semantic_payload_hash,
            original_standard_id,
            raw_body_hash,
        })
    }
}

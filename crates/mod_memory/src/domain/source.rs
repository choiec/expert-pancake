use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use core_shared::{AppError, AppResult};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSystemMetadata {
    pub canonical_payload_hash: String,
    pub ingest_kind: IngestKind,
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

    pub fn canonical_payload_hash(&self) -> &str {
        self.source_metadata
            .pointer("/system/canonical_payload_hash")
            .and_then(Value::as_str)
            .unwrap_or_default()
    }
}

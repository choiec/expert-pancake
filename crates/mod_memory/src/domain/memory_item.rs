use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitType {
    Paragraph,
    Section,
    JsonDocument,
    MetadataPlaceholder,
}

impl UnitType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Paragraph => "paragraph",
            Self::Section => "section",
            Self::JsonDocument => "json_document",
            Self::MetadataPlaceholder => "metadata_placeholder",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryItem {
    pub urn: String,
    pub source_id: Uuid,
    pub sequence: u32,
    pub unit_type: UnitType,
    pub start_offset: u32,
    pub end_offset: u32,
    pub version: String,
    pub content: String,
    pub content_hash: String,
    pub item_metadata: serde_json::Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

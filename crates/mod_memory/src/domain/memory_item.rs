use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use core_shared::MemoryItemUrn;

pub const MEMORY_ITEM_VERSION: &str = "v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryUnitType {
    Paragraph,
    Section,
    JsonDocument,
    MetadataPlaceholder,
}

impl MemoryUnitType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Paragraph => "paragraph",
            Self::Section => "section",
            Self::JsonDocument => "json_document",
            Self::MetadataPlaceholder => "metadata_placeholder",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryItem {
    pub urn: MemoryItemUrn,
    pub source_id: Uuid,
    pub sequence: u32,
    pub unit_type: MemoryUnitType,
    pub start_offset: u32,
    pub end_offset: u32,
    pub version: String,
    pub content: String,
    pub content_hash: String,
    pub item_metadata: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

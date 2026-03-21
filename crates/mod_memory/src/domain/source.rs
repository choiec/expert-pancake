use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentType {
    Text,
    Markdown,
    Json,
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Markdown => "markdown",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalSource {
    pub title: String,
    pub summary: Option<String>,
    pub external_id: String,
    pub document_type: DocumentType,
    pub content: String,
    pub metadata: serde_json::Value,
    pub canonical_hash: String,
}

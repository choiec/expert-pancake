use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::memory_item::MemoryItem;
use crate::domain::source::{DocumentType, Source};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphMemoryItemProjection {
    pub urn: String,
    pub sequence: u32,
    pub unit_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRegisteredEvent {
    pub source_id: Uuid,
    pub document_type: DocumentType,
    pub memory_items: Vec<GraphMemoryItemProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphProjectionEvent {
    SourceRegistered(SourceRegisteredEvent),
}

impl GraphProjectionEvent {
    pub fn source_registered(source: &Source, memory_items: &[MemoryItem]) -> Self {
        Self::SourceRegistered(SourceRegisteredEvent {
            source_id: source.source_id,
            document_type: source.document_type,
            memory_items: memory_items
                .iter()
                .map(|item| GraphMemoryItemProjection {
                    urn: item.urn.to_string(),
                    sequence: item.sequence,
                    unit_type: item.unit_type.as_str().to_owned(),
                })
                .collect(),
        })
    }
}

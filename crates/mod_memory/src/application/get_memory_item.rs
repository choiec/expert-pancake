use std::sync::Arc;

use time::OffsetDateTime;

use core_shared::{AppResult, MemoryItemUrn};

use crate::domain::source::DocumentType;
use crate::infra::repo::MemoryQueryRepository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetMemoryItemResult {
    pub urn: String,
    pub source_id: uuid::Uuid,
    pub sequence: u32,
    pub document_type: DocumentType,
    pub content: String,
    pub unit_type: String,
    pub start_offset: u32,
    pub end_offset: u32,
    pub version: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub source_external_id: String,
    pub source_title: String,
}

pub struct GetMemoryItemService {
    repo: Arc<dyn MemoryQueryRepository>,
}

impl GetMemoryItemService {
    pub fn new(repo: Arc<dyn MemoryQueryRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, urn: &MemoryItemUrn) -> AppResult<GetMemoryItemResult> {
        let record = self.repo.get_memory_item(urn).await?;
        Ok(GetMemoryItemResult {
            urn: record.memory_item.urn.to_string(),
            source_id: record.memory_item.source_id,
            sequence: record.memory_item.sequence,
            document_type: record.source.document_type,
            content: record.memory_item.content,
            unit_type: record.memory_item.unit_type.as_str().to_owned(),
            start_offset: record.memory_item.start_offset,
            end_offset: record.memory_item.end_offset,
            version: record.memory_item.version,
            created_at: record.memory_item.created_at,
            updated_at: record.memory_item.updated_at,
            source_external_id: record.source.external_id,
            source_title: record.source.title,
        })
    }
}

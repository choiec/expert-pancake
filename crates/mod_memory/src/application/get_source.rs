use std::sync::Arc;

use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use core_shared::AppResult;

use crate::domain::source::DocumentType;
use crate::infra::indexer::PublicIndexingStatus;
use crate::infra::repo::SourceQueryRepository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMemoryItemResult {
    pub urn: String,
    pub source_id: Uuid,
    pub sequence: u32,
    pub unit_type: String,
    pub start_offset: u32,
    pub end_offset: u32,
    pub version: String,
    pub content: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetSourceResult {
    pub source_id: Uuid,
    pub external_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub document_type: DocumentType,
    pub source_metadata: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub indexing_status: PublicIndexingStatus,
    pub decision_reason: String,
    pub memory_items: Vec<SourceMemoryItemResult>,
}

pub struct GetSourceService {
    repo: Arc<dyn SourceQueryRepository>,
}

impl GetSourceService {
    pub fn new(repo: Arc<dyn SourceQueryRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, source_id: Uuid) -> AppResult<GetSourceResult> {
        let bundle = self.repo.get_source(source_id).await?;
        let source = bundle.source;
        let source_metadata = source.public_source_metadata();
        Ok(GetSourceResult {
            source_id: source.source_id,
            external_id: source.external_id,
            title: source.title,
            summary: source.summary,
            document_type: source.document_type,
            source_metadata,
            created_at: source.created_at,
            updated_at: source.updated_at,
            indexing_status: bundle.indexing_status,
            decision_reason: "LOOKUP_RESOLVED_CANONICAL".to_owned(),
            memory_items: bundle
                .memory_items
                .into_iter()
                .map(|item| SourceMemoryItemResult {
                    urn: item.urn.to_string(),
                    source_id: item.source_id,
                    sequence: item.sequence,
                    unit_type: item.unit_type.as_str().to_owned(),
                    start_offset: item.start_offset,
                    end_offset: item.end_offset,
                    version: item.version,
                    content: item.content,
                    created_at: item.created_at,
                    updated_at: item.updated_at,
                })
                .collect(),
        })
    }
}

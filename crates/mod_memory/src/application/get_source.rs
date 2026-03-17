use std::sync::Arc;

use time::OffsetDateTime;
use uuid::Uuid;

use core_shared::AppResult;

use crate::domain::source::DocumentType;
use crate::infra::indexer::PublicIndexingStatus;
use crate::infra::repo::SourceQueryRepository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMemoryItemResult {
    pub urn: String,
    pub sequence: u32,
    pub unit_type: String,
    pub start_offset: u32,
    pub end_offset: u32,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetSourceResult {
    pub source_id: Uuid,
    pub external_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub document_type: DocumentType,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub indexing_status: PublicIndexingStatus,
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
        Ok(GetSourceResult {
            source_id: bundle.source.source_id,
            external_id: bundle.source.external_id,
            title: bundle.source.title,
            summary: bundle.source.summary,
            document_type: bundle.source.document_type,
            created_at: bundle.source.created_at,
            updated_at: bundle.source.updated_at,
            indexing_status: bundle.indexing_status,
            memory_items: bundle
                .memory_items
                .into_iter()
                .map(|item| SourceMemoryItemResult {
                    urn: item.urn.to_string(),
                    sequence: item.sequence,
                    unit_type: item.unit_type.as_str().to_owned(),
                    start_offset: item.start_offset,
                    end_offset: item.end_offset,
                    content: item.content,
                })
                .collect(),
        })
    }
}

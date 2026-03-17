use std::sync::Arc;

use uuid::Uuid;

use core_shared::{AppError, AppResult};

use crate::domain::source::DocumentType;
use crate::infra::indexer::{ProjectionIndexPort, ProjectionSearchQuery};

const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMemoryItemsQuery {
    pub query: Option<String>,
    pub source_id: Option<Uuid>,
    pub document_type: Option<DocumentType>,
    pub limit: usize,
    pub offset: usize,
}

impl Default for SearchMemoryItemsQuery {
    fn default() -> Self {
        Self {
            query: None,
            source_id: None,
            document_type: None,
            limit: DEFAULT_LIMIT,
            offset: 0,
        }
    }
}

impl SearchMemoryItemsQuery {
    pub fn validate(&self) -> AppResult<()> {
        if self.limit == 0 || self.limit > MAX_LIMIT {
            return Err(AppError::validation(format!(
                "limit must be between 1 and {MAX_LIMIT}",
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchMemoryItemsHit {
    pub urn: String,
    pub source_id: Uuid,
    pub sequence: u32,
    pub document_type: DocumentType,
    pub content_preview: String,
    pub score: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchMemoryItemsResult {
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
    pub items: Vec<SearchMemoryItemsHit>,
}

pub struct SearchMemoryItemsService {
    projection_index: Arc<dyn ProjectionIndexPort>,
}

impl SearchMemoryItemsService {
    pub fn new(projection_index: Arc<dyn ProjectionIndexPort>) -> Self {
        Self { projection_index }
    }

    pub async fn execute(
        &self,
        query: SearchMemoryItemsQuery,
    ) -> AppResult<SearchMemoryItemsResult> {
        query.validate()?;
        let projection_result = self
            .projection_index
            .search(&ProjectionSearchQuery {
                query: query.query,
                source_id: query.source_id,
                document_type: query.document_type.map(|value| value.as_str().to_owned()),
                limit: query.limit,
                offset: query.offset,
            })
            .await?;

        Ok(SearchMemoryItemsResult {
            total: projection_result.total,
            limit: projection_result.limit,
            offset: projection_result.offset,
            items: projection_result
                .items
                .into_iter()
                .map(|item| SearchMemoryItemsHit {
                    urn: item.urn,
                    source_id: item.source_id,
                    sequence: item.sequence,
                    document_type: parse_document_type(&item.document_type),
                    content_preview: item.content_preview,
                    score: item.score,
                })
                .collect(),
        })
    }
}

fn parse_document_type(value: &str) -> DocumentType {
    match value {
        "text" => DocumentType::Text,
        "markdown" => DocumentType::Markdown,
        _ => DocumentType::Json,
    }
}

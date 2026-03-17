use async_trait::async_trait;
use uuid::Uuid;

use core_shared::{AppResult, MemoryItemUrn};

use crate::domain::memory_item::MemoryItem;
use crate::domain::source::{NewSource, Source};
use crate::infra::indexer::{IndexingJob, PublicIndexingStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBundle {
    pub source: Source,
    pub memory_items: Vec<MemoryItem>,
    pub indexing_status: PublicIndexingStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryItemWithSource {
    pub source: Source,
    pub memory_item: MemoryItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceCreateOrReplay {
    Create(NewSource),
    Replay(SourceBundle),
}

#[async_trait]
pub trait SourceRepository: Send + Sync {
    async fn prepare_create_or_replay(&self, source: NewSource) -> AppResult<SourceCreateOrReplay>;
}

#[async_trait]
pub trait MemoryRepository: Send + Sync {
    async fn commit_registration(
        &self,
        source: NewSource,
        memory_items: Vec<MemoryItem>,
        job: IndexingJob,
    ) -> AppResult<SourceBundle>;
}

#[async_trait]
pub trait MemoryQueryRepository: Send + Sync {
    async fn get_memory_item(&self, urn: &MemoryItemUrn) -> AppResult<MemoryItemWithSource>;
}

#[async_trait]
pub trait SourceQueryRepository: Send + Sync {
    async fn get_source(&self, source_id: Uuid) -> AppResult<SourceBundle>;
}

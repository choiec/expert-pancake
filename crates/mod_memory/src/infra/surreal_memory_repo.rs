use std::sync::Arc;

use async_trait::async_trait;

use core_infra::surrealdb::{CommitRegistrationOutcome, InMemorySurrealDb};
use core_shared::AppResult;

use crate::domain::memory_item::MemoryItem;
use crate::domain::source::NewSource;
use crate::infra::repo::{MemoryRepository, SourceBundle};
use crate::infra::surreal_source_repo::{
    bundle_from_records, job_to_record, memory_item_to_record, source_to_record,
};

#[derive(Debug, Clone)]
pub struct SurrealMemoryRepository {
    db: Arc<InMemorySurrealDb>,
}

impl SurrealMemoryRepository {
    pub fn new(db: Arc<InMemorySurrealDb>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MemoryRepository for SurrealMemoryRepository {
    async fn commit_registration(
        &self,
        source: NewSource,
        memory_items: Vec<MemoryItem>,
        job: crate::infra::indexer::IndexingJob,
    ) -> AppResult<SourceBundle> {
        let outcome = self.db.commit_registration(
            source_to_record(&source),
            memory_items.iter().map(memory_item_to_record).collect(),
            job_to_record(&job),
        )?;
        match outcome {
            CommitRegistrationOutcome::Created(bundle)
            | CommitRegistrationOutcome::Replay(bundle) => {
                bundle_from_records(bundle, self.db.search_available())
            }
        }
    }
}

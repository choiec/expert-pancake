use std::sync::Arc;

use async_trait::async_trait;

use core_infra::surrealdb::{CommitRegistrationOutcome, InMemorySurrealDb, SurrealDbService};
use core_shared::{AppError, AppResult};

use crate::domain::memory_item::MemoryItem;
use crate::domain::source::NewSource;
use crate::infra::repo::{MemoryRepository, SourceBundle};
use crate::infra::surreal_source_repo::{
    bundle_from_records, fetch_source_bundle_by_external_id, job_to_record, memory_item_to_record,
    source_to_record,
};

type SearchAvailabilityProbe = Arc<dyn Fn() -> bool + Send + Sync>;

#[derive(Debug, Clone)]
/// Fixture-backed repository used by storage contract tests.
pub struct SurrealMemoryRepository {
    db: Arc<InMemorySurrealDb>,
}

impl SurrealMemoryRepository {
    pub fn new(db: Arc<InMemorySurrealDb>) -> Self {
        Self { db }
    }
}

#[derive(Clone)]
/// Production/runtime repository backed by the real SurrealDB client.
pub struct RuntimeSurrealMemoryRepository {
    db: Arc<SurrealDbService>,
    search_available: SearchAvailabilityProbe,
}

impl RuntimeSurrealMemoryRepository {
    pub fn new(db: Arc<SurrealDbService>, search_available: SearchAvailabilityProbe) -> Self {
        Self {
            db,
            search_available,
        }
    }

    fn search_available(&self) -> bool {
        (self.search_available)()
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

#[async_trait]
impl MemoryRepository for RuntimeSurrealMemoryRepository {
    async fn commit_registration(
        &self,
        source: NewSource,
        memory_items: Vec<MemoryItem>,
        job: crate::infra::indexer::IndexingJob,
    ) -> AppResult<SourceBundle> {
        let persisted_source = source_to_record(&source);
        let persisted_items = memory_items.iter().map(memory_item_to_record).collect();
        let persisted_job = job_to_record(&job);

        match self
            .db
            .commit_authoritative_registration(persisted_source, persisted_items, persisted_job)
            .await
        {
            Ok(()) => {
                let bundle =
                    fetch_source_bundle_by_external_id(self.db.as_ref(), &source.external_id)
                        .await?
                        .ok_or_else(|| {
                            AppError::internal(format!(
                                "source '{}' missing after commit",
                                source.external_id
                            ))
                        })?;
                bundle_from_records(bundle, self.search_available())
            }
            Err(error) if error.kind() == core_shared::ErrorKind::Conflict => {
                let Some(existing) =
                    fetch_source_bundle_by_external_id(self.db.as_ref(), &source.external_id)
                        .await?
                else {
                    return Err(error);
                };
                let existing_hash = existing
                    .source
                    .source_metadata
                    .pointer("/system/semantic_payload_hash")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                if existing_hash == source.semantic_payload_hash() {
                    bundle_from_records(existing, self.search_available())
                } else {
                    Err(AppError::conflict(format!(
                        "external_id '{}' is already registered with a different semantic payload",
                        source.external_id
                    )))
                }
            }
            Err(error) => Err(error),
        }
    }
}

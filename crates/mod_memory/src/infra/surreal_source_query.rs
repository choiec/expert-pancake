use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use core_infra::surrealdb::{InMemorySurrealDb, SurrealDbService};
use core_shared::{AppError, AppResult};

use crate::infra::repo::{SourceBundle, SourceQueryRepository};
use crate::infra::surreal_source_repo::{bundle_from_records, fetch_source_bundle_by_source_id};

type SearchAvailabilityProbe = Arc<dyn Fn() -> bool + Send + Sync>;

#[derive(Debug, Clone)]
/// Fixture-backed repository used by storage contract tests.
pub struct SurrealSourceQueryRepository {
    db: Arc<InMemorySurrealDb>,
}

impl SurrealSourceQueryRepository {
    pub fn new(db: Arc<InMemorySurrealDb>) -> Self {
        Self { db }
    }
}

#[derive(Clone)]
/// Production/runtime repository backed by the real SurrealDB client.
pub struct RuntimeSurrealSourceQueryRepository {
    db: Arc<SurrealDbService>,
    search_available: SearchAvailabilityProbe,
}

impl RuntimeSurrealSourceQueryRepository {
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
impl SourceQueryRepository for SurrealSourceQueryRepository {
    async fn get_source(&self, source_id: Uuid) -> AppResult<SourceBundle> {
        let bundle = self
            .db
            .get_source_bundle(source_id)
            .ok_or_else(|| AppError::not_found(format!("source '{}' was not found", source_id)))?;
        bundle_from_records(bundle, self.db.search_available())
    }
}

#[async_trait]
impl SourceQueryRepository for RuntimeSurrealSourceQueryRepository {
    async fn get_source(&self, source_id: Uuid) -> AppResult<SourceBundle> {
        let bundle = fetch_source_bundle_by_source_id(self.db.as_ref(), source_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("source '{}' was not found", source_id)))?;
        bundle_from_records(bundle, self.search_available())
    }
}

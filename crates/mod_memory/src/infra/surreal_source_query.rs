use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use core_infra::surrealdb::InMemorySurrealDb;
use core_shared::{AppError, AppResult};

use crate::infra::repo::{SourceBundle, SourceQueryRepository};
use crate::infra::surreal_source_repo::bundle_from_records;

#[derive(Debug, Clone)]
pub struct SurrealSourceQueryRepository {
    db: Arc<InMemorySurrealDb>,
}

impl SurrealSourceQueryRepository {
    pub fn new(db: Arc<InMemorySurrealDb>) -> Self {
        Self { db }
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

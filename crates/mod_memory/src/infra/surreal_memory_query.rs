use std::sync::Arc;

use async_trait::async_trait;

use core_infra::surrealdb::{InMemorySurrealDb, SurrealDbService};
use core_shared::{AppError, AppResult, MemoryItemUrn};

use crate::infra::repo::{MemoryItemWithSource, MemoryQueryRepository};
use crate::infra::surreal_source_repo::{
    fetch_memory_item_with_source, memory_item_from_record, source_from_record,
};

#[derive(Debug, Clone)]
/// Fixture-backed repository used by storage contract tests.
pub struct SurrealMemoryQueryRepository {
    db: Arc<InMemorySurrealDb>,
}

impl SurrealMemoryQueryRepository {
    pub fn new(db: Arc<InMemorySurrealDb>) -> Self {
        Self { db }
    }
}

#[derive(Clone)]
/// Production/runtime repository backed by the real SurrealDB client.
pub struct RuntimeSurrealMemoryQueryRepository {
    db: Arc<SurrealDbService>,
}

impl RuntimeSurrealMemoryQueryRepository {
    pub fn new(db: Arc<SurrealDbService>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MemoryQueryRepository for SurrealMemoryQueryRepository {
    async fn get_memory_item(&self, urn: &MemoryItemUrn) -> AppResult<MemoryItemWithSource> {
        let (memory_item, source) = self
            .db
            .get_memory_item(urn.as_str())
            .ok_or_else(|| AppError::not_found(format!("memory item '{}' was not found", urn)))?;
        Ok(MemoryItemWithSource {
            source: source_from_record(source)?,
            memory_item: memory_item_from_record(memory_item)?,
        })
    }
}

#[async_trait]
impl MemoryQueryRepository for RuntimeSurrealMemoryQueryRepository {
    async fn get_memory_item(&self, urn: &MemoryItemUrn) -> AppResult<MemoryItemWithSource> {
        let (memory_item, source) = fetch_memory_item_with_source(self.db.as_ref(), urn.as_str())
            .await?
            .ok_or_else(|| AppError::not_found(format!("memory item '{}' was not found", urn)))?;
        Ok(MemoryItemWithSource {
            source: source_from_record(source)?,
            memory_item: memory_item_from_record(memory_item)?,
        })
    }
}

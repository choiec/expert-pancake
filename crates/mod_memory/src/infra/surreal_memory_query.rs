use std::sync::Arc;

use async_trait::async_trait;

use core_infra::surrealdb::InMemorySurrealDb;
use core_shared::{AppError, AppResult, MemoryItemUrn};

use crate::infra::repo::{MemoryItemWithSource, MemoryQueryRepository};
use crate::infra::surreal_source_repo::{memory_item_from_record, source_from_record};

#[derive(Debug, Clone)]
pub struct SurrealMemoryQueryRepository {
    db: Arc<InMemorySurrealDb>,
}

impl SurrealMemoryQueryRepository {
    pub fn new(db: Arc<InMemorySurrealDb>) -> Self {
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

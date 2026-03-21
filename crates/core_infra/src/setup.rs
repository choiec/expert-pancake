use std::sync::Arc;

use crate::{InMemorySurrealDb, MeilisearchClient, surrealdb::SurrealDbClient};

#[derive(Debug, Clone)]
pub struct InfrastructureSetup {
    pub db: Arc<InMemorySurrealDb>,
    pub surrealdb: SurrealDbClient,
    pub meilisearch: MeilisearchClient,
}

impl InfrastructureSetup {
    pub fn bootstrap_in_memory(surrealdb: SurrealDbClient, meilisearch: MeilisearchClient) -> Self {
        let db = Arc::new(InMemorySurrealDb::new());
        db.set_search_available(meilisearch.is_available());
        Self {
            db,
            surrealdb,
            meilisearch,
        }
    }
}

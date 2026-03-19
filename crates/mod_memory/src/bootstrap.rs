use std::sync::Arc;
use std::time::Duration;

use core_infra::surrealdb::InMemorySurrealDb;
use core_infra::{MeilisearchService, NoopGraphProjectionAdapter, SurrealDbService};
use core_shared::{DefaultIdGenerator, IdGenerator};

use crate::application::{
    get_memory_item::GetMemoryItemService,
    get_source::GetSourceService,
    index_memory_items::{IndexMemoryItemsService, RetryPolicy},
    register_source::{RegisterSourceService, SystemClock},
    search_memory_items::SearchMemoryItemsService,
};
use crate::infra::{
    indexer::OutboxOnlyIndexer,
    meili_indexer::{
        InMemoryIndexingOutboxRepository, InMemoryMeiliProjectionIndex,
        RuntimeIndexingOutboxRepository, RuntimeMeiliProjectionIndex,
    },
    surreal_memory_query::{RuntimeSurrealMemoryQueryRepository, SurrealMemoryQueryRepository},
    surreal_memory_repo::{RuntimeSurrealMemoryRepository, SurrealMemoryRepository},
    surreal_source_query::{RuntimeSurrealSourceQueryRepository, SurrealSourceQueryRepository},
    surreal_source_repo::{RuntimeSurrealSourceRepository, SurrealSourceRepository},
};

#[derive(Clone)]
pub struct MemoryModule {
    register_source: Arc<RegisterSourceService>,
    get_memory_item: Arc<GetMemoryItemService>,
    get_source: Arc<GetSourceService>,
    search_memory_items: Arc<SearchMemoryItemsService>,
    index_memory_items: Arc<IndexMemoryItemsService>,
}

impl MemoryModule {
    pub fn runtime(
        surrealdb: Arc<SurrealDbService>,
        meilisearch: Arc<MeilisearchService>,
        search_enabled: bool,
        normalization_timeout: Duration,
    ) -> Self {
        let id_generator = Arc::new(DefaultIdGenerator) as Arc<dyn IdGenerator>;
        let search_available = Arc::new(move || search_enabled);
        let register_source = Arc::new(RegisterSourceService::new(
            Arc::new(RuntimeSurrealSourceRepository::new(
                surrealdb.clone(),
                search_available.clone(),
            )),
            Arc::new(RuntimeSurrealMemoryRepository::new(
                surrealdb.clone(),
                search_available.clone(),
            )),
            Arc::new(OutboxOnlyIndexer::new(search_enabled)),
            Arc::new(NoopGraphProjectionAdapter),
            Arc::new(SystemClock),
            id_generator.clone(),
            normalization_timeout,
        ));
        let get_memory_item = Arc::new(GetMemoryItemService::new(Arc::new(
            RuntimeSurrealMemoryQueryRepository::new(surrealdb.clone()),
        )));
        let get_source = Arc::new(GetSourceService::new(Arc::new(
            RuntimeSurrealSourceQueryRepository::new(surrealdb.clone(), search_available),
        )));
        let projection_index = Arc::new(RuntimeMeiliProjectionIndex::new(meilisearch));
        let search_memory_items = Arc::new(SearchMemoryItemsService::new(projection_index.clone()));
        let index_memory_items = Arc::new(IndexMemoryItemsService::new(
            Arc::new(RuntimeIndexingOutboxRepository::new(surrealdb)),
            projection_index,
            Arc::new(SystemClock),
            RetryPolicy::default(),
        ));

        Self {
            register_source,
            get_memory_item,
            get_source,
            search_memory_items,
            index_memory_items,
        }
    }

    pub fn fixture(
        db: Arc<InMemorySurrealDb>,
        projection_available: bool,
        normalization_timeout: Duration,
    ) -> Self {
        let register_source = Arc::new(RegisterSourceService::new(
            Arc::new(SurrealSourceRepository::new(db.clone())),
            Arc::new(SurrealMemoryRepository::new(db.clone())),
            Arc::new(OutboxOnlyIndexer::new(db.search_available())),
            Arc::new(NoopGraphProjectionAdapter),
            Arc::new(SystemClock),
            Arc::new(DefaultIdGenerator),
            normalization_timeout,
        ));
        let get_memory_item = Arc::new(GetMemoryItemService::new(Arc::new(
            SurrealMemoryQueryRepository::new(db.clone()),
        )));
        let get_source = Arc::new(GetSourceService::new(Arc::new(
            SurrealSourceQueryRepository::new(db.clone()),
        )));
        let projection_index = Arc::new(InMemoryMeiliProjectionIndex::new());
        projection_index.set_available(projection_available);
        let search_memory_items = Arc::new(SearchMemoryItemsService::new(projection_index.clone()));
        let index_memory_items = Arc::new(IndexMemoryItemsService::new(
            Arc::new(InMemoryIndexingOutboxRepository::new(db)),
            projection_index,
            Arc::new(SystemClock),
            RetryPolicy::default(),
        ));

        Self {
            register_source,
            get_memory_item,
            get_source,
            search_memory_items,
            index_memory_items,
        }
    }

    pub fn register_source(&self) -> Arc<RegisterSourceService> {
        self.register_source.clone()
    }

    pub fn get_memory_item(&self) -> Arc<GetMemoryItemService> {
        self.get_memory_item.clone()
    }

    pub fn get_source(&self) -> Arc<GetSourceService> {
        self.get_source.clone()
    }

    pub fn search_memory_items(&self) -> Arc<SearchMemoryItemsService> {
        self.search_memory_items.clone()
    }

    pub fn index_memory_items_service(&self) -> Arc<IndexMemoryItemsService> {
        self.index_memory_items.clone()
    }

    pub async fn index_memory_items(&self) {
        self.index_memory_items.clone().run_forever().await;
    }
}

impl std::fmt::Debug for MemoryModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryModule").finish_non_exhaustive()
    }
}

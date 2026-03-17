use std::sync::{Arc, Mutex};
use std::time::Duration;

use core_infra::NoopGraphProjectionAdapter;
use core_infra::surrealdb::InMemorySurrealDb;
use core_shared::DefaultIdGenerator;
use mod_memory::application::get_source::GetSourceService;
use mod_memory::application::index_memory_items::{IndexMemoryItemsService, RetryPolicy};
use mod_memory::application::register_source::{
    ClockPort, RegisterSourceCommand, RegisterSourceService,
};
use mod_memory::domain::source::{DocumentType, IngestKind};
use mod_memory::infra::indexer::OutboxOnlyIndexer;
use mod_memory::infra::meili_indexer::{
    InMemoryIndexingOutboxRepository, InMemoryMeiliProjectionIndex,
};
use mod_memory::infra::surreal_memory_repo::SurrealMemoryRepository;
use mod_memory::infra::surreal_source_query::SurrealSourceQueryRepository;
use mod_memory::infra::surreal_source_repo::SurrealSourceRepository;
use time::OffsetDateTime;

#[tokio::test]
async fn public_indexing_status_maps_from_queued_to_indexed() {
    let db = Arc::new(InMemorySurrealDb::new());
    let register = RegisterSourceService::new(
        Arc::new(SurrealSourceRepository::new(db.clone())),
        Arc::new(SurrealMemoryRepository::new(db.clone())),
        Arc::new(OutboxOnlyIndexer::new(true)),
        Arc::new(NoopGraphProjectionAdapter),
        Arc::new(AdvancingClock::new(OffsetDateTime::now_utc(), 1)),
        Arc::new(DefaultIdGenerator),
        Duration::from_secs(30),
    );
    let query = GetSourceService::new(Arc::new(SurrealSourceQueryRepository::new(db.clone())));
    let worker = IndexMemoryItemsService::new(
        Arc::new(InMemoryIndexingOutboxRepository::new(db)),
        Arc::new(InMemoryMeiliProjectionIndex::new()),
        Arc::new(AdvancingClock::new(OffsetDateTime::now_utc(), 1)),
        RetryPolicy::default(),
    );

    let result = register
        .execute(RegisterSourceCommand {
            external_id: "status-map-001".to_owned(),
            title: "Status map".to_owned(),
            summary: None,
            document_type: DocumentType::Text,
            authoritative_content: "hello status".to_owned(),
            source_metadata: serde_json::json!({}),
            canonical_payload_hash: "status-hash-001".to_owned(),
            ingest_kind: IngestKind::Canonical,
        })
        .await
        .expect("registration should succeed");
    assert_eq!(result.indexing_status.as_str(), "queued");

    let queued = query
        .execute(result.source_id)
        .await
        .expect("source should load");
    assert_eq!(queued.indexing_status.as_str(), "queued");

    worker
        .process_next_job()
        .await
        .expect("worker run should succeed")
        .expect("job should exist");

    let indexed = query
        .execute(result.source_id)
        .await
        .expect("source should load");
    assert_eq!(indexed.indexing_status.as_str(), "indexed");
}

#[tokio::test]
async fn dead_letter_maps_to_deferred_without_exposing_internal_state() {
    let db = Arc::new(InMemorySurrealDb::new());
    let register = RegisterSourceService::new(
        Arc::new(SurrealSourceRepository::new(db.clone())),
        Arc::new(SurrealMemoryRepository::new(db.clone())),
        Arc::new(OutboxOnlyIndexer::new(true)),
        Arc::new(NoopGraphProjectionAdapter),
        Arc::new(AdvancingClock::new(OffsetDateTime::now_utc(), 60)),
        Arc::new(DefaultIdGenerator),
        Duration::from_secs(30),
    );
    let query = GetSourceService::new(Arc::new(SurrealSourceQueryRepository::new(db.clone())));
    let projection = InMemoryMeiliProjectionIndex::new();
    projection.set_available(false);
    let worker = IndexMemoryItemsService::new(
        Arc::new(InMemoryIndexingOutboxRepository::new(db)),
        Arc::new(projection),
        Arc::new(AdvancingClock::new(OffsetDateTime::now_utc(), 60)),
        RetryPolicy {
            max_retries: 1,
            retry_delay: Duration::from_secs(0),
            poll_interval: Duration::from_millis(1),
        },
    );

    let result = register
        .execute(RegisterSourceCommand {
            external_id: "status-map-dead-letter-001".to_owned(),
            title: "Dead letter".to_owned(),
            summary: None,
            document_type: DocumentType::Text,
            authoritative_content: "dead letter content".to_owned(),
            source_metadata: serde_json::json!({}),
            canonical_payload_hash: "status-hash-dead-letter-001".to_owned(),
            ingest_kind: IngestKind::Canonical,
        })
        .await
        .expect("registration should succeed");
    assert_eq!(result.indexing_status.as_str(), "queued");

    worker
        .process_next_job()
        .await
        .expect("first worker run should succeed")
        .expect("first job should exist");
    worker
        .process_next_job()
        .await
        .expect("second worker run should succeed")
        .expect("second job should exist");

    let source = query
        .execute(result.source_id)
        .await
        .expect("source should load");
    assert_eq!(source.indexing_status.as_str(), "deferred");
}

#[derive(Debug)]
struct AdvancingClock {
    current: Mutex<OffsetDateTime>,
    seconds_per_tick: i64,
}

impl AdvancingClock {
    fn new(start: OffsetDateTime, seconds_per_tick: i64) -> Self {
        Self {
            current: Mutex::new(start),
            seconds_per_tick,
        }
    }
}

impl ClockPort for AdvancingClock {
    fn now(&self) -> OffsetDateTime {
        let mut guard = self.current.lock().expect("clock mutex poisoned");
        let now = *guard;
        *guard += time::Duration::seconds(self.seconds_per_tick);
        now
    }
}

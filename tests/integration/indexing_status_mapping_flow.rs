use std::sync::{Arc, Mutex};
use std::time::Duration;

use app_server::{
    config::AppConfig,
    router::build_router,
    state::{AppState, ProbeSnapshot},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
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
use serde_json::{Value, json};
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn processing_internal_status_still_surfaces_as_public_queued() {
    let db = Arc::new(InMemorySurrealDb::new());
    let app = build_router(AppState::for_memory_ingest_test(
        AppConfig::for_test(),
        ProbeSnapshot::ready(),
        db.clone(),
    ));

    let registered = post_json(
        &app,
        "/sources/register",
        json!({
            "title": "Processing status",
            "external-id": "https://api.cherry-pick.net/cc/v1p3/example.edu:status-map-processing-001",
            "document-type": "text",
            "content": "processing status content"
        }),
    )
    .await;
    assert_eq!(registered.0, StatusCode::CREATED);

    let source_id = registered.1["source_id"]
        .as_str()
        .expect("source id present")
        .parse::<Uuid>()
        .expect("source id should parse");

    let claimed = db
        .claim_next_index_job(OffsetDateTime::now_utc())
        .expect("pending job should be claimed");
    assert_eq!(claimed.status, "processing");

    let source = get_json(&app, &format!("/sources/{source_id}")).await;
    assert_eq!(source.0, StatusCode::OK);
    assert_eq!(source.1["indexing_status"], "queued");
    assert_ne!(source.1["indexing_status"], "processing");
}

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
        Arc::new(InMemoryIndexingOutboxRepository::new(db.clone())),
        Arc::new(InMemoryMeiliProjectionIndex::new()),
        Arc::new(AdvancingClock::new(OffsetDateTime::now_utc(), 1)),
        RetryPolicy::default(),
    );
    let app = build_router(AppState::for_memory_ingest_test(
        AppConfig::for_test(),
        ProbeSnapshot::ready(),
        db.clone(),
    ));

    let result = register
        .execute(RegisterSourceCommand {
            external_id: "https://api.cherry-pick.net/cc/v1p3/example.edu:status-map-001".to_owned(),
            title: "Status map".to_owned(),
            summary: None,
            document_type: DocumentType::Text,
            authoritative_content: "hello status".to_owned(),
            source_metadata: serde_json::json!({}),
            semantic_payload_hash: "status-hash-001".to_owned(),
            original_standard_id: None,
            raw_body_hash: None,
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

    let public_source = get_json(&app, &format!("/sources/{}", result.source_id)).await;
    assert_eq!(public_source.0, StatusCode::OK);
    assert_eq!(public_source.1["indexing_status"], "indexed");
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
        Arc::new(InMemoryIndexingOutboxRepository::new(db.clone())),
        Arc::new(projection),
        Arc::new(AdvancingClock::new(OffsetDateTime::now_utc(), 60)),
        RetryPolicy {
            max_retries: 1,
            retry_delay: Duration::from_secs(0),
            poll_interval: Duration::from_millis(1),
        },
    );
    let app = build_router(AppState::for_memory_ingest_test_with_projection(
        AppConfig::for_test(),
        ProbeSnapshot::new(
            app_server::state::ProbeStatus::Ready,
            app_server::state::ProbeStatus::Ready,
            app_server::state::ProbeStatus::Degraded,
        ),
        db.clone(),
        false,
    ));

    let result = register
        .execute(RegisterSourceCommand {
            external_id: "https://api.cherry-pick.net/cc/v1p3/example.edu:status-map-dead-letter-001".to_owned(),
            title: "Dead letter".to_owned(),
            summary: None,
            document_type: DocumentType::Text,
            authoritative_content: "dead letter content".to_owned(),
            source_metadata: serde_json::json!({}),
            semantic_payload_hash: "status-hash-dead-letter-001".to_owned(),
            original_standard_id: None,
            raw_body_hash: None,
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

    let internal_status = db
        .latest_index_job(result.source_id)
        .expect("job should exist")
        .status;
    assert_eq!(internal_status, "dead_letter");

    let public_source = get_json(&app, &format!("/sources/{}", result.source_id)).await;
    assert_eq!(public_source.0, StatusCode::OK);
    assert_eq!(public_source.1["indexing_status"], "deferred");
    for internal in ["pending", "processing", "retryable", "completed", "dead_letter"] {
        assert_ne!(public_source.1["indexing_status"], internal);
    }
}

async fn post_json(app: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    let status = response.status();
    (status, decode_json(response).await)
}

async fn get_json(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must succeed");
    let status = response.status();
    (status, decode_json(response).await)
}

async fn decode_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body must decode");
    serde_json::from_slice(&body).expect("response must be valid json")
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

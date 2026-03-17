use std::sync::Arc;
use std::time::Duration;

use core_infra::{NoopGraphProjectionAdapter, surrealdb::InMemorySurrealDb};
use core_shared::DefaultIdGenerator;
use mod_memory::application::register_source::{
    RegisterSourceCommand, RegisterSourceService, SystemClock,
};
use mod_memory::domain::source::{DocumentType, IngestKind};
use mod_memory::infra::indexer::{OutboxOnlyIndexer, OutboxStatus, derive_public_indexing_status};
use mod_memory::infra::surreal_memory_repo::SurrealMemoryRepository;
use mod_memory::infra::surreal_source_repo::SurrealSourceRepository;

#[tokio::test]
async fn outbox_rows_can_rehydrate_projection_inputs_from_authoritative_rows() {
    let db = Arc::new(InMemorySurrealDb::new());
    let service = RegisterSourceService::new(
        Arc::new(SurrealSourceRepository::new(db.clone())),
        Arc::new(SurrealMemoryRepository::new(db.clone())),
        Arc::new(OutboxOnlyIndexer::new(true)),
        Arc::new(NoopGraphProjectionAdapter),
        Arc::new(SystemClock),
        Arc::new(DefaultIdGenerator),
        Duration::from_secs(30),
    );

    let result = service
        .execute(RegisterSourceCommand {
            external_id: "projection-source".to_owned(),
            title: "Projection".to_owned(),
            summary: None,
            document_type: DocumentType::Markdown,
            authoritative_content: "# Intro\n\nHello world".to_owned(),
            source_metadata: serde_json::json!({"projection": true}),
            canonical_payload_hash: "projection-hash".to_owned(),
            ingest_kind: IngestKind::Canonical,
        })
        .await
        .expect("registration should succeed");

    let authoritative_bundle = db
        .get_source_bundle(result.source_id)
        .expect("authoritative bundle should exist");
    let projection = db
        .rehydrate_projection(result.source_id)
        .expect("projection rehydration should work from source_id");

    assert_eq!(
        authoritative_bundle
            .latest_job
            .as_ref()
            .map(|job| job.source_id),
        Some(result.source_id)
    );
    assert_eq!(projection.source.source_id, result.source_id);
    assert_eq!(projection.memory_items.len(), result.memory_items.len());
    assert_eq!(projection.memory_items[0].urn, result.memory_items[0].urn);
    assert_eq!(
        projection.memory_items[0].sequence,
        result.memory_items[0].sequence
    );
    assert_eq!(projection.source.document_type, "markdown");
    assert!(projection.memory_items[0].content.starts_with("# Intro"));

    assert_eq!(
        derive_public_indexing_status(Some(OutboxStatus::Pending), true).as_str(),
        "queued"
    );
    assert_eq!(
        derive_public_indexing_status(Some(OutboxStatus::Completed), true).as_str(),
        "indexed"
    );
    assert_eq!(
        derive_public_indexing_status(Some(OutboxStatus::Retryable), true).as_str(),
        "deferred"
    );
    assert_eq!(
        derive_public_indexing_status(Some(OutboxStatus::Pending), false).as_str(),
        "deferred"
    );
}

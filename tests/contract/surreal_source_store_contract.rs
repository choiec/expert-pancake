//! Fixture-level authoritative storage contract proofs.
//!
//! These tests exercise `InMemorySurrealDb`, which deliberately models the
//! same uniqueness, replay, and readiness semantics as the runtime adapter.

use std::sync::Arc;
use std::time::Duration;

use core_infra::{NoopGraphProjectionAdapter, surrealdb::InMemorySurrealDb};
use core_shared::{DefaultIdGenerator, ErrorCode};
use mod_memory::application::register_source::{
    RegisterSourceCommand, RegisterSourceService, SystemClock,
};
use mod_memory::domain::source::{DocumentType, IngestKind};
use mod_memory::infra::indexer::OutboxOnlyIndexer;
use mod_memory::infra::surreal_memory_repo::SurrealMemoryRepository;
use mod_memory::infra::surreal_source_repo::SurrealSourceRepository;

fn register_service(db: Arc<InMemorySurrealDb>) -> RegisterSourceService {
    RegisterSourceService::new(
        Arc::new(SurrealSourceRepository::new(db.clone())),
        Arc::new(SurrealMemoryRepository::new(db)),
        Arc::new(OutboxOnlyIndexer::new(true)),
        Arc::new(NoopGraphProjectionAdapter),
        Arc::new(SystemClock),
        Arc::new(DefaultIdGenerator),
        Duration::from_secs(30),
    )
}

#[tokio::test]
async fn readiness_probe_tracks_authoritative_write_availability() {
    let db = InMemorySurrealDb::new();
    assert!(db.readiness_probe().is_ok());

    db.set_write_available(false);
    let error = db
        .readiness_probe()
        .expect_err("write outage should fail readiness");

    assert_eq!(error.kind(), ErrorCode::StorageUnavailable);
}

#[tokio::test]
async fn identical_external_id_and_hash_replay_existing_authoritative_identifiers() {
    let db = Arc::new(InMemorySurrealDb::new());
    let service = register_service(db);

    let first = service
        .execute(RegisterSourceCommand {
            external_id: "https://api.cherry-pick.net/cc/v1p3/example.edu:source-001".to_owned(),
            title: "Rust Notes".to_owned(),
            summary: Some("source replay contract".to_owned()),
            document_type: DocumentType::Text,
            authoritative_content: "alpha\n\n beta".to_owned(),
            source_metadata: serde_json::json!({"topic": "rust"}),
            semantic_payload_hash: "hash-001".to_owned(),
            original_standard_id: None,
            raw_body_hash: None,
            ingest_kind: IngestKind::Canonical,
        })
        .await
        .expect("first registration should succeed");

    let replay = service
        .execute(RegisterSourceCommand {
            external_id: "https://api.cherry-pick.net/cc/v1p3/example.edu:source-001".to_owned(),
            title: "Rust Notes".to_owned(),
            summary: Some("source replay contract".to_owned()),
            document_type: DocumentType::Text,
            authoritative_content: "alpha\n\n beta".to_owned(),
            source_metadata: serde_json::json!({"topic": "rust"}),
            semantic_payload_hash: "hash-001".to_owned(),
            original_standard_id: None,
            raw_body_hash: None,
            ingest_kind: IngestKind::Canonical,
        })
        .await
        .expect("replay should succeed");

    assert_eq!(first.source_id, replay.source_id);
    assert_eq!(first.memory_items, replay.memory_items);
    assert!(replay.replayed);
}

#[tokio::test]
async fn conflicting_payload_hash_for_same_external_id_returns_conflict() {
    let db = Arc::new(InMemorySurrealDb::new());
    let service = register_service(db);

    service
        .execute(RegisterSourceCommand {
            external_id: "https://api.cherry-pick.net/cc/v1p3/example.edu:source-002".to_owned(),
            title: "Rust Notes".to_owned(),
            summary: None,
            document_type: DocumentType::Text,
            authoritative_content: "alpha".to_owned(),
            source_metadata: serde_json::json!({}),
            semantic_payload_hash: "hash-a".to_owned(),
            original_standard_id: None,
            raw_body_hash: None,
            ingest_kind: IngestKind::Canonical,
        })
        .await
        .expect("initial registration should succeed");

    let error = service
        .execute(RegisterSourceCommand {
            external_id: "https://api.cherry-pick.net/cc/v1p3/example.edu:source-002".to_owned(),
            title: "Rust Notes".to_owned(),
            summary: None,
            document_type: DocumentType::Text,
            authoritative_content: "beta".to_owned(),
            source_metadata: serde_json::json!({}),
            semantic_payload_hash: "hash-b".to_owned(),
            original_standard_id: None,
            raw_body_hash: None,
            ingest_kind: IngestKind::Canonical,
        })
        .await
        .expect_err("conflicting replay should fail");

    assert_eq!(error.kind(), ErrorCode::Conflict);
}

//! Fixture-level authoritative retention contract proofs.
//!
//! These tests pin no-TTL semantics against the deterministic in-memory
//! fixture, not against a live SurrealDB process.

use std::sync::Arc;
use std::time::Duration;

use core_infra::{NoopGraphProjectionAdapter, surrealdb::InMemorySurrealDb};
use core_shared::DefaultIdGenerator;
use mod_memory::application::register_source::{
    RegisterSourceCommand, RegisterSourceService, SystemClock,
};
use mod_memory::domain::source::{DocumentType, IngestKind};
use mod_memory::infra::indexer::OutboxOnlyIndexer;
use mod_memory::infra::surreal_memory_repo::SurrealMemoryRepository;
use mod_memory::infra::surreal_source_repo::SurrealSourceRepository;

#[tokio::test]
async fn authoritative_records_have_no_ttl_or_implicit_purge_baseline() {
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
            external_id: "https://api.cherry-pick.net/cc/v1p3/example.edu:retention-source"
                .to_owned(),
            title: "Retention".to_owned(),
            summary: Some("baseline".to_owned()),
            document_type: DocumentType::Markdown,
            authoritative_content: "# One\n\nalpha\n\n# Two\n\nbeta".to_owned(),
            source_metadata: serde_json::json!({"retention": "indefinite"}),
            semantic_payload_hash: "retention-hash".to_owned(),
            original_standard_id: None,
            raw_body_hash: None,
            ingest_kind: IngestKind::Canonical,
        })
        .await
        .expect("registration should succeed");

    let initial_bundle = db
        .get_source_bundle(result.source_id)
        .expect("authoritative bundle should exist");
    assert_eq!(initial_bundle.memory_items.len(), 2);

    db.set_search_available(false);
    let retained_bundle = db
        .get_source_bundle(result.source_id)
        .expect("authoritative bundle should still exist");
    assert_eq!(
        retained_bundle.source.external_id,
        "https://api.cherry-pick.net/cc/v1p3/example.edu:retention-source"
    );
    assert_eq!(retained_bundle.memory_items.len(), 2);
    assert!(db.rehydrate_projection(result.source_id).is_some());
}

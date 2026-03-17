use std::sync::Arc;
use std::time::Duration;

use core_infra::{
    NoopGraphProjectionAdapter,
    surrealdb::{
        InMemorySurrealDb, PersistedIndexJobRecord, PersistedMemoryItemRecord,
        PersistedSourceRecord,
    },
};
use core_shared::{DefaultIdGenerator, ErrorCode};
use mod_memory::application::register_source::{
    RegisterSourceCommand, RegisterSourceService, SystemClock,
};
use mod_memory::domain::source::{DocumentType, IngestKind};
use mod_memory::infra::indexer::OutboxOnlyIndexer;
use mod_memory::infra::surreal_memory_repo::SurrealMemoryRepository;
use mod_memory::infra::surreal_source_repo::SurrealSourceRepository;
use time::OffsetDateTime;
use uuid::Uuid;

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
async fn failed_authoritative_commit_leaves_no_partial_state() {
    let db = Arc::new(InMemorySurrealDb::new());
    db.fail_next_commit();
    let service = register_service(db.clone());

    let error = service
        .execute(RegisterSourceCommand {
            external_id: "source-rollback".to_owned(),
            title: "Rollback".to_owned(),
            summary: None,
            document_type: DocumentType::Text,
            authoritative_content: "alpha\n\nbeta".to_owned(),
            source_metadata: serde_json::json!({"case": "rollback"}),
            canonical_payload_hash: "rollback-hash".to_owned(),
            ingest_kind: IngestKind::Canonical,
        })
        .await
        .expect_err("simulated transaction failure should surface");

    assert_eq!(error.kind(), ErrorCode::StorageUnavailable);
    assert!(db.lookup_source_by_external_id("source-rollback").is_none());
}

#[test]
fn duplicate_sequence_and_urn_constraints_are_enforced_atomically() {
    let db = InMemorySurrealDb::new();
    let now = OffsetDateTime::now_utc();
    let source_id = Uuid::new_v4();

    let source = PersistedSourceRecord {
        source_id,
        external_id: "sequence-source".to_owned(),
        title: "Source".to_owned(),
        summary: None,
        document_type: "text".to_owned(),
        source_metadata: serde_json::json!({"system": {"canonical_payload_hash": "hash-1"}}),
        created_at: now,
        updated_at: now,
    };
    let base_item = PersistedMemoryItemRecord {
        urn: "urn:memory-item:duplicate-a".to_owned(),
        source_id,
        sequence: 0,
        unit_type: "paragraph".to_owned(),
        start_offset: 0,
        end_offset: 5,
        version: "v1".to_owned(),
        content: "alpha".to_owned(),
        content_hash: "hash-alpha".to_owned(),
        item_metadata: serde_json::json!({}),
        created_at: now,
        updated_at: now,
    };

    let duplicate_sequence_error = db
        .commit_registration(
            source.clone(),
            vec![
                base_item.clone(),
                PersistedMemoryItemRecord {
                    urn: "urn:memory-item:duplicate-b".to_owned(),
                    sequence: 0,
                    ..base_item.clone()
                },
            ],
            PersistedIndexJobRecord {
                job_id: Uuid::new_v4(),
                source_id,
                status: "pending".to_owned(),
                retry_count: 0,
                last_error: None,
                available_at: now,
                created_at: now,
                updated_at: now,
            },
        )
        .expect_err("duplicate sequence should fail");
    assert_eq!(duplicate_sequence_error.kind(), ErrorCode::Conflict);
    assert!(db.lookup_source_by_external_id("sequence-source").is_none());

    db.commit_registration(
        source.clone(),
        vec![base_item.clone()],
        PersistedIndexJobRecord {
            job_id: Uuid::new_v4(),
            source_id,
            status: "pending".to_owned(),
            retry_count: 0,
            last_error: None,
            available_at: now,
            created_at: now,
            updated_at: now,
        },
    )
    .expect("first commit should succeed");

    let second_source_id = Uuid::new_v4();
    let duplicate_urn_error = db
        .commit_registration(
            PersistedSourceRecord {
                source_id: second_source_id,
                external_id: "urn-source".to_owned(),
                title: "Source 2".to_owned(),
                summary: None,
                document_type: "text".to_owned(),
                source_metadata: serde_json::json!({"system": {"canonical_payload_hash": "hash-2"}}),
                created_at: now,
                updated_at: now,
            },
            vec![PersistedMemoryItemRecord {
                source_id: second_source_id,
                ..base_item
            }],
            PersistedIndexJobRecord {
                job_id: Uuid::new_v4(),
                source_id: second_source_id,
                status: "pending".to_owned(),
                retry_count: 0,
                last_error: None,
                available_at: now,
                created_at: now,
                updated_at: now,
            },
        )
        .expect_err("duplicate URN should fail");
    assert_eq!(duplicate_urn_error.kind(), ErrorCode::Conflict);
    assert!(db.lookup_source_by_external_id("urn-source").is_none());
}

use std::sync::Arc;
use std::time::Duration;

use core_infra::{NoopGraphProjectionAdapter, surrealdb::InMemorySurrealDb};
use core_shared::{DefaultIdGenerator, ErrorCode, MemoryItemUrn};
use mod_memory::application::get_memory_item::GetMemoryItemService;
use mod_memory::application::register_source::{
    RegisterSourceCommand, RegisterSourceService, SystemClock,
};
use mod_memory::domain::normalization::{normalized_json_hash_from_str, raw_body_hash_from_str};
use mod_memory::domain::source::{DocumentType, IngestKind};
use mod_memory::infra::indexer::OutboxOnlyIndexer;
use mod_memory::infra::surreal_memory_query::SurrealMemoryQueryRepository;
use mod_memory::infra::surreal_memory_repo::SurrealMemoryRepository;
use mod_memory::infra::surreal_source_repo::SurrealSourceRepository;

fn fixture(path: &str) -> &'static str {
    match path {
        "open_badges_compact" => {
            include_str!("../../../repo_tests/fixtures/register_source/replay_hashing/open_badges_compact.json")
        }
        "open_badges_pretty" => {
            include_str!("../../../repo_tests/fixtures/register_source/replay_hashing/open_badges_pretty.json")
        }
        "clr_compact" => {
            include_str!("../../../repo_tests/fixtures/register_source/replay_hashing/clr_compact.json")
        }
        "clr_pretty" => {
            include_str!("../../../repo_tests/fixtures/register_source/replay_hashing/clr_pretty.json")
        }
        "clr_conflict" => {
            include_str!("../../../repo_tests/fixtures/register_source/replay_hashing/clr_conflict.json")
        }
        "open_badges_conflict" => {
            include_str!("../../../repo_tests/fixtures/register_source/standards/open_badges_conflict.json")
        }
        _ => panic!("unknown fixture path: {path}"),
    }
}

fn build_service(db: Arc<InMemorySurrealDb>) -> RegisterSourceService {
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

const BADGE_EXTERNAL_ID: &str =
    "https://api.cherry-pick.net/ob/v2p0/issuer.example.org:urn%3Abadge%3A001";
const CLR_EXTERNAL_ID: &str = "https://api.cherry-pick.net/clr/v2p0/issuer.example.org:https%3A%2F%2Fclr.example%2Fcredentials%2F123";

#[tokio::test]
async fn formatting_only_replay_returns_the_first_authoritative_json_body() {
    let db = Arc::new(InMemorySurrealDb::new());
    let service = build_service(db.clone());
    let query_service =
        GetMemoryItemService::new(Arc::new(SurrealMemoryQueryRepository::new(db.clone())));

    let first_body = fixture("open_badges_compact");
    let replay_body = fixture("open_badges_pretty");
    let canonical_hash = normalized_json_hash_from_str(first_body).expect("first payload hashes");

    let created = service
        .execute(RegisterSourceCommand {
            external_id: BADGE_EXTERNAL_ID.to_owned(),
            title: "Rust Badge".to_owned(),
            summary: Some("direct standard replay test".to_owned()),
            document_type: DocumentType::Json,
            authoritative_content: first_body.to_owned(),
            source_metadata: serde_json::json!({"issuer": "issuer.example.org"}),
            semantic_payload_hash: canonical_hash.clone(),
            original_standard_id: Some("urn:badge:001".to_owned()),
            raw_body_hash: Some(raw_body_hash_from_str(first_body)),
            ingest_kind: IngestKind::DirectStandard,
        })
        .await
        .expect("first registration should succeed");

    let replay = service
        .execute(RegisterSourceCommand {
            external_id: BADGE_EXTERNAL_ID.to_owned(),
            title: "Rust Badge".to_owned(),
            summary: Some("direct standard replay test".to_owned()),
            document_type: DocumentType::Json,
            authoritative_content: replay_body.to_owned(),
            source_metadata: serde_json::json!({"issuer": "issuer.example.org"}),
            semantic_payload_hash: normalized_json_hash_from_str(replay_body)
                .expect("replay payload hashes"),
            original_standard_id: Some("urn:badge:001".to_owned()),
            raw_body_hash: Some(raw_body_hash_from_str(replay_body)),
            ingest_kind: IngestKind::DirectStandard,
        })
        .await
        .expect("formatting-only replay should succeed");

    assert!(!created.replayed);
    assert!(replay.replayed);
    assert_eq!(created.source_id, replay.source_id);
    assert_eq!(created.memory_items, replay.memory_items);

    let memory_item = query_service
        .execute(&MemoryItemUrn::new(created.memory_items[0].urn.clone()))
        .await
        .expect("authoritative memory item should be readable");

    assert_eq!(memory_item.content, first_body);
    assert_eq!(memory_item.document_type, DocumentType::Json);
    assert_eq!(memory_item.unit_type, "json_document");
}

#[tokio::test]
async fn clr_formatting_only_replay_preserves_the_first_authoritative_raw_body() {
    let db = Arc::new(InMemorySurrealDb::new());
    let service = build_service(db.clone());
    let query_service =
        GetMemoryItemService::new(Arc::new(SurrealMemoryQueryRepository::new(db.clone())));

    let first_body = fixture("clr_compact");
    let replay_body = fixture("clr_pretty");

    let created = service
        .execute(RegisterSourceCommand {
            external_id: CLR_EXTERNAL_ID.to_owned(),
            title: "Rust CLR".to_owned(),
            summary: Some("clr replay test".to_owned()),
            document_type: DocumentType::Json,
            authoritative_content: first_body.to_owned(),
            source_metadata: serde_json::json!({"issuer": "issuer.example.org", "standard": "clr"}),
            semantic_payload_hash: normalized_json_hash_from_str(first_body)
                .expect("compact CLR should hash"),
            original_standard_id: Some("https://clr.example/credentials/123".to_owned()),
            raw_body_hash: Some(raw_body_hash_from_str(first_body)),
            ingest_kind: IngestKind::DirectStandard,
        })
        .await
        .expect("first CLR registration should succeed");

    let replay = service
        .execute(RegisterSourceCommand {
            external_id: CLR_EXTERNAL_ID.to_owned(),
            title: "Rust CLR".to_owned(),
            summary: Some("clr replay test".to_owned()),
            document_type: DocumentType::Json,
            authoritative_content: replay_body.to_owned(),
            source_metadata: serde_json::json!({"issuer": "issuer.example.org", "standard": "clr"}),
            semantic_payload_hash: normalized_json_hash_from_str(replay_body)
                .expect("formatted CLR should hash"),
            original_standard_id: Some("https://clr.example/credentials/123".to_owned()),
            raw_body_hash: Some(raw_body_hash_from_str(replay_body)),
            ingest_kind: IngestKind::DirectStandard,
        })
        .await
        .expect("formatting-only CLR replay should succeed");

    assert!(!created.replayed);
    assert!(replay.replayed);
    assert_eq!(created.source_id, replay.source_id);
    assert_eq!(created.memory_items, replay.memory_items);

    let memory_item = query_service
        .execute(&MemoryItemUrn::new(created.memory_items[0].urn.clone()))
        .await
        .expect("CLR memory item should be readable");

    assert_eq!(memory_item.content, first_body);
    assert_eq!(memory_item.document_type, DocumentType::Json);
    assert_eq!(memory_item.unit_type, "json_document");
}

#[tokio::test]
async fn clr_semantic_conflict_returns_conflict_without_overwriting_first_body() {
    let db = Arc::new(InMemorySurrealDb::new());
    let service = build_service(db.clone());
    let query_service =
        GetMemoryItemService::new(Arc::new(SurrealMemoryQueryRepository::new(db.clone())));

    let first_body = fixture("clr_compact");
    let conflicting_body = fixture("clr_conflict");

    let created = service
        .execute(RegisterSourceCommand {
            external_id: CLR_EXTERNAL_ID.to_owned(),
            title: "Rust CLR".to_owned(),
            summary: Some("clr conflict test".to_owned()),
            document_type: DocumentType::Json,
            authoritative_content: first_body.to_owned(),
            source_metadata: serde_json::json!({"issuer": "issuer.example.org", "standard": "clr"}),
            semantic_payload_hash: normalized_json_hash_from_str(first_body)
                .expect("compact CLR should hash"),
            original_standard_id: Some("https://clr.example/credentials/123".to_owned()),
            raw_body_hash: Some(raw_body_hash_from_str(first_body)),
            ingest_kind: IngestKind::DirectStandard,
        })
        .await
        .expect("first CLR registration should succeed");

    let error = service
        .execute(RegisterSourceCommand {
            external_id: CLR_EXTERNAL_ID.to_owned(),
            title: "Rust CLR".to_owned(),
            summary: Some("clr conflict test".to_owned()),
            document_type: DocumentType::Json,
            authoritative_content: conflicting_body.to_owned(),
            source_metadata: serde_json::json!({"issuer": "issuer.example.org", "standard": "clr"}),
            semantic_payload_hash: normalized_json_hash_from_str(conflicting_body)
                .expect("conflicting CLR should hash"),
            original_standard_id: Some("https://clr.example/credentials/123".to_owned()),
            raw_body_hash: Some(raw_body_hash_from_str(conflicting_body)),
            ingest_kind: IngestKind::DirectStandard,
        })
        .await
        .expect_err("semantic conflict should fail");

    assert_eq!(error.kind(), ErrorCode::Conflict);

    let memory_item = query_service
        .execute(&MemoryItemUrn::new(created.memory_items[0].urn.clone()))
        .await
        .expect("original CLR memory item should be readable");

    assert_eq!(memory_item.content, first_body);
}

#[tokio::test]
async fn open_badges_semantic_conflict_returns_conflict_without_overwriting_first_body() {
    let db = Arc::new(InMemorySurrealDb::new());
    let service = build_service(db.clone());
    let query_service =
        GetMemoryItemService::new(Arc::new(SurrealMemoryQueryRepository::new(db.clone())));

    let first_body = fixture("open_badges_compact");
    let conflicting_body = fixture("open_badges_conflict");

    let created = service
        .execute(RegisterSourceCommand {
            external_id: BADGE_EXTERNAL_ID.to_owned(),
            title: "Rust Badge".to_owned(),
            summary: Some("badge conflict test".to_owned()),
            document_type: DocumentType::Json,
            authoritative_content: first_body.to_owned(),
            source_metadata: serde_json::json!({"issuer": "issuer.example.org"}),
            semantic_payload_hash: normalized_json_hash_from_str(first_body)
                .expect("compact badge should hash"),
            original_standard_id: Some("urn:badge:001".to_owned()),
            raw_body_hash: Some(raw_body_hash_from_str(first_body)),
            ingest_kind: IngestKind::DirectStandard,
        })
        .await
        .expect("first badge registration should succeed");

    let error = service
        .execute(RegisterSourceCommand {
            external_id: BADGE_EXTERNAL_ID.to_owned(),
            title: "Rust Badge".to_owned(),
            summary: Some("badge conflict test".to_owned()),
            document_type: DocumentType::Json,
            authoritative_content: conflicting_body.to_owned(),
            source_metadata: serde_json::json!({"issuer": "issuer.example.org"}),
            semantic_payload_hash: normalized_json_hash_from_str(conflicting_body)
                .expect("conflicting badge should hash"),
            original_standard_id: Some("urn:badge:001".to_owned()),
            raw_body_hash: Some(raw_body_hash_from_str(conflicting_body)),
            ingest_kind: IngestKind::DirectStandard,
        })
        .await
        .expect_err("semantic conflict should fail");

    assert_eq!(error.kind(), ErrorCode::Conflict);

    let memory_item = query_service
        .execute(&MemoryItemUrn::new(created.memory_items[0].urn.clone()))
        .await
        .expect("original badge memory item should be readable");

    assert_eq!(memory_item.content, first_body);
    assert_eq!(memory_item.document_type, DocumentType::Json);
    assert_eq!(memory_item.unit_type, "json_document");
}

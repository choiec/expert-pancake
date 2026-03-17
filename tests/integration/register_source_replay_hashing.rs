use std::sync::Arc;
use std::time::Duration;

use core_infra::{NoopGraphProjectionAdapter, surrealdb::InMemorySurrealDb};
use core_shared::{DefaultIdGenerator, MemoryItemUrn};
use mod_memory::application::get_memory_item::GetMemoryItemService;
use mod_memory::application::register_source::{
    RegisterSourceCommand, RegisterSourceService, SystemClock,
};
use mod_memory::domain::normalization::normalized_json_hash_from_str;
use mod_memory::domain::source::{DocumentType, IngestKind};
use mod_memory::infra::indexer::OutboxOnlyIndexer;
use mod_memory::infra::surreal_memory_query::SurrealMemoryQueryRepository;
use mod_memory::infra::surreal_memory_repo::SurrealMemoryRepository;
use mod_memory::infra::surreal_source_repo::SurrealSourceRepository;

#[tokio::test]
async fn formatting_only_replay_returns_the_first_authoritative_json_body() {
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
    let query_service =
        GetMemoryItemService::new(Arc::new(SurrealMemoryQueryRepository::new(db.clone())));

    let first_body = r#"{"id":"urn:badge:001","name":"Rust Badge","issuer":{"id":"https://issuer.example.org"}}"#;
    let replay_body = r#"
    {
      "issuer": {
        "id": "https://issuer.example.org"
      },
      "name": "Rust Badge",
      "id": "urn:badge:001"
    }
    "#;
    let canonical_hash = normalized_json_hash_from_str(first_body).expect("first payload hashes");

    let created = service
        .execute(RegisterSourceCommand {
            external_id: "urn:badge:001".to_owned(),
            title: "Rust Badge".to_owned(),
            summary: Some("direct standard replay test".to_owned()),
            document_type: DocumentType::Json,
            authoritative_content: first_body.to_owned(),
            source_metadata: serde_json::json!({"issuer": "issuer.example.org"}),
            canonical_payload_hash: canonical_hash.clone(),
            ingest_kind: IngestKind::DirectStandard,
        })
        .await
        .expect("first registration should succeed");

    let replay = service
        .execute(RegisterSourceCommand {
            external_id: "urn:badge:001".to_owned(),
            title: "Rust Badge".to_owned(),
            summary: Some("direct standard replay test".to_owned()),
            document_type: DocumentType::Json,
            authoritative_content: replay_body.to_owned(),
            source_metadata: serde_json::json!({"issuer": "issuer.example.org"}),
            canonical_payload_hash: normalized_json_hash_from_str(replay_body)
                .expect("replay payload hashes"),
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

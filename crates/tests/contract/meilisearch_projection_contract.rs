use mod_memory::infra::indexer::{ProjectionIndexPort, ProjectionInput, ProjectionSearchQuery};
use mod_memory::infra::meili_indexer::InMemoryMeiliProjectionIndex;
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn meilisearch_projection_contract_pins_index_settings() {
    let adapter = InMemoryMeiliProjectionIndex::new();

    adapter
        .ensure_index()
        .await
        .expect("settings bootstrap should succeed");

    let settings = adapter
        .settings_snapshot()
        .expect("settings snapshot should be recorded");
    assert_eq!(settings.index_uid, "memory_items_v1");
    assert_eq!(settings.primary_key, "urn");
    assert_eq!(
        settings.filterable_attributes,
        ["source_id", "document_type"]
    );
    assert_eq!(
        settings.sortable_attributes,
        ["sequence", "created_at", "updated_at"]
    );
    assert_eq!(
        settings.searchable_attributes,
        ["content_preview", "urn", "source_id", "content_hash"]
    );
}

#[tokio::test]
async fn meilisearch_projection_contract_is_idempotent_and_filterable() {
    let adapter = InMemoryMeiliProjectionIndex::new();
    let source_id = Uuid::new_v4();
    let created_at = OffsetDateTime::now_utc();

    adapter
        .upsert(&[ProjectionInput {
            urn: "urn:memory-item:1".to_owned(),
            source_id,
            sequence: 0,
            document_type: "json".to_owned(),
            content_preview: "Rust badge preview".to_owned(),
            content_hash: "hash-1".to_owned(),
            created_at,
            updated_at: created_at,
        }])
        .await
        .expect("first upsert should succeed");
    adapter
        .upsert(&[ProjectionInput {
            urn: "urn:memory-item:1".to_owned(),
            source_id,
            sequence: 0,
            document_type: "json".to_owned(),
            content_preview: "Updated Rust badge preview".to_owned(),
            content_hash: "hash-2".to_owned(),
            created_at,
            updated_at: created_at,
        }])
        .await
        .expect("idempotent replace should succeed");

    let docs = adapter.documents();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].content_hash, "hash-2");

    let result = adapter
        .search(&ProjectionSearchQuery {
            query: Some("Rust".to_owned()),
            source_id: Some(source_id),
            document_type: Some("json".to_owned()),
            limit: 10,
            offset: 0,
        })
        .await
        .expect("filtered search should succeed");
    assert_eq!(result.total, 1);
    assert_eq!(result.items[0].document_type, "json");
    assert!(
        result.items[0]
            .content_preview
            .contains("Updated Rust badge preview")
    );
}

#[tokio::test]
async fn meilisearch_projection_contract_surfaces_degraded_availability() {
    let adapter = InMemoryMeiliProjectionIndex::new();
    adapter.set_available(false);

    let error = adapter
        .search(&ProjectionSearchQuery {
            query: Some("hello".to_owned()),
            source_id: None,
            document_type: None,
            limit: 10,
            offset: 0,
        })
        .await
        .expect_err("search should fail when projection is unavailable");

    assert_eq!(error.error_code(), "SEARCH_UNAVAILABLE");
}

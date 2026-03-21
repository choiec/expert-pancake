use core_shared::DefaultIdGenerator;
use mod_memory::domain::{
    normalization::{NormalizationInput, normalize_source, normalized_json_hash},
    source::DocumentType,
};
use time::OffsetDateTime;
use uuid::Uuid;

#[test]
fn text_normalization_uses_utf8_byte_offsets() {
    let items = normalize_source(
        &NormalizationInput {
            source_id: Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap(),
            external_id: "utf8-source",
            title: "UTF8",
            summary: None,
            document_type: DocumentType::Text,
            authoritative_content: "가나다\n\nRust",
            source_metadata: &serde_json::json!({}),
            created_at: OffsetDateTime::now_utc(),
        },
        &DefaultIdGenerator,
    )
    .unwrap();

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].start_offset, 0);
    assert_eq!(items[0].end_offset, 9);
    assert_eq!(items[1].start_offset, 11);
    assert_eq!(items[1].end_offset, 15);
}

#[test]
fn formatting_only_json_replays_share_hash() {
    let compact = r#"{"id":"1","name":"A","type":"AchievementCredential","@context":"https://purl.imsglobal.org/spec/ob/v3p0/context.json"}"#;
    let pretty = "{\n  \"name\": \"A\",\n  \"type\": \"AchievementCredential\",\n  \"@context\": \"https://purl.imsglobal.org/spec/ob/v3p0/context.json\",\n  \"id\": \"1\"\n}";
    assert_eq!(
        normalized_json_hash(compact).unwrap(),
        normalized_json_hash(pretty).unwrap()
    );
}

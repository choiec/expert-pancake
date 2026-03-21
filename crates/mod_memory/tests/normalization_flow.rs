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
    let compact = r#"{"@context":["https://www.w3.org/ns/credentials/v2","https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json"],"id":"https://example.com/credential/1","type":["VerifiableCredential","AchievementCredential"],"name":"A","issuer":"https://issuer.example.com/issuers/1","validFrom":"2025-01-01T00:00:00Z","credentialSubject":{"type":"AchievementSubject","achievement":{"id":"https://example.com/achievements/a","type":"Achievement","name":"A","description":"A desc","criteria":{}}}}"#;
    let pretty = "{\n  \"credentialSubject\": {\n    \"achievement\": {\n      \"criteria\": {},\n      \"description\": \"A desc\",\n      \"id\": \"https://example.com/achievements/a\",\n      \"name\": \"A\",\n      \"type\": \"Achievement\"\n    },\n    \"type\": \"AchievementSubject\"\n  },\n  \"validFrom\": \"2025-01-01T00:00:00Z\",\n  \"issuer\": \"https://issuer.example.com/issuers/1\",\n  \"name\": \"A\",\n  \"type\": [\n    \"VerifiableCredential\",\n    \"AchievementCredential\"\n  ],\n  \"id\": \"https://example.com/credential/1\",\n  \"@context\": [\n    \"https://www.w3.org/ns/credentials/v2\",\n    \"https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json\"\n  ]\n}";
    assert_eq!(
        normalized_json_hash(compact).unwrap(),
        normalized_json_hash(pretty).unwrap()
    );
}

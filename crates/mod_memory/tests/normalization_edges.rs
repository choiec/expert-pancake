use core_shared::DefaultIdGenerator;
use mod_memory::domain::normalization::{NormalizationInput, normalize_source};
use mod_memory::domain::source::DocumentType;
use time::OffsetDateTime;
use uuid::Uuid;

#[test]
fn empty_content_creates_a_metadata_placeholder_item() {
    let source_id =
        Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").expect("valid source id");
    let created_at = OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid timestamp");
    let generator = DefaultIdGenerator;

    let items = normalize_source(
        &NormalizationInput {
            source_id,
            external_id: "empty-source",
            title: "Empty Source",
            summary: Some("placeholder"),
            document_type: DocumentType::Text,
            authoritative_content: "",
            source_metadata: &serde_json::json!({"topic": "empty"}),
            created_at,
        },
        &generator,
    )
    .expect("empty content should normalize");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].sequence, 0);
    assert_eq!(items[0].unit_type.as_str(), "metadata_placeholder");
    assert_eq!(items[0].start_offset, 0);
    assert_eq!(items[0].end_offset, 0);
    assert_eq!(items[0].content, "");
}

#[test]
fn normalization_is_deterministic_for_sequence_and_urns() {
    let source_id =
        Uuid::parse_str("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb").expect("valid source id");
    let created_at = OffsetDateTime::from_unix_timestamp(1_700_000_001).expect("valid timestamp");
    let generator = DefaultIdGenerator;
    let input = NormalizationInput {
        source_id,
        external_id: "deterministic-source",
        title: "Deterministic Source",
        summary: None,
        document_type: DocumentType::Text,
        authoritative_content: "alpha\n\nbeta\n\ngamma",
        source_metadata: &serde_json::json!({"topic": "deterministic"}),
        created_at,
    };

    let first = normalize_source(&input, &generator).expect("first normalization should succeed");
    let second = normalize_source(&input, &generator).expect("second normalization should succeed");

    assert_eq!(first.len(), 3);
    assert_eq!(first.len(), second.len());

    for (index, (left, right)) in first.iter().zip(second.iter()).enumerate() {
        assert_eq!(left.sequence as usize, index);
        assert_eq!(left.sequence, right.sequence);
        assert_eq!(left.urn, right.urn);
        assert_eq!(left.start_offset, right.start_offset);
        assert_eq!(left.end_offset, right.end_offset);
    }
}

#[test]
fn direct_standard_json_creates_one_json_document_item() {
    let source_id =
        Uuid::parse_str("cccccccc-cccc-4ccc-8ccc-cccccccccccc").expect("valid source id");
    let created_at = OffsetDateTime::from_unix_timestamp(1_700_000_002).expect("valid timestamp");
    let generator = DefaultIdGenerator;
    let raw_body =
        include_str!("../../repo_tests/fixtures/register_source/replay_hashing/clr_compact.json");

    let items = normalize_source(
        &NormalizationInput {
            source_id,
            external_id: "https://clr.example/credentials/123",
            title: "Rust CLR",
            summary: Some("json document"),
            document_type: DocumentType::Json,
            authoritative_content: raw_body,
            source_metadata: &serde_json::json!({"standard": "clr"}),
            created_at,
        },
        &generator,
    )
    .expect("json normalization should succeed");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].unit_type.as_str(), "json_document");
    assert_eq!(items[0].sequence, 0);
    assert_eq!(items[0].start_offset, 0);
    assert_eq!(items[0].end_offset as usize, raw_body.len());
    assert_eq!(items[0].content, raw_body);
}

#[test]
fn utf8_offsets_follow_raw_byte_ranges() {
    let source_id =
        Uuid::parse_str("dddddddd-dddd-4ddd-8ddd-dddddddddddd").expect("valid source id");
    let created_at = OffsetDateTime::from_unix_timestamp(1_700_000_003).expect("valid timestamp");
    let generator = DefaultIdGenerator;
    let raw_body = "가나다\n\nRust";

    let items = normalize_source(
        &NormalizationInput {
            source_id,
            external_id: "utf8-source",
            title: "UTF-8 Source",
            summary: None,
            document_type: DocumentType::Text,
            authoritative_content: raw_body,
            source_metadata: &serde_json::json!({}),
            created_at,
        },
        &generator,
    )
    .expect("utf-8 content should normalize");

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].content, "가나다");
    assert_eq!(items[0].start_offset, 0);
    assert_eq!(items[0].end_offset, 9);
    assert_eq!(items[1].content, "Rust");
    assert_eq!(items[1].start_offset, 11);
    assert_eq!(items[1].end_offset, 15);
    assert_eq!(
        &raw_body[items[0].start_offset as usize..items[0].end_offset as usize],
        "가나다"
    );
    assert_eq!(
        &raw_body[items[1].start_offset as usize..items[1].end_offset as usize],
        "Rust"
    );
}

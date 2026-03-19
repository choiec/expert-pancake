use mod_memory::domain::normalization::normalized_json_hash_from_str;

#[test]
fn formatting_only_json_variants_share_the_same_hash() {
    let compact = r#"{"id":"urn:badge:001","name":"Rust Badge","nested":{"b":2,"a":1},"list":[{"y":2,"z":1}]}"#;
    let formatted = r#"
    {
      "list": [
        {
          "z": 1,
          "y": 2
        }
      ],
      "nested": {
        "a": 1,
        "b": 2
      },
      "name": "Rust Badge",
      "id": "urn:badge:001"
    }
    "#;

    let compact_hash = normalized_json_hash_from_str(compact).expect("compact JSON should hash");
    let formatted_hash =
        normalized_json_hash_from_str(formatted).expect("formatted JSON should hash");

    assert_eq!(compact_hash, formatted_hash);
}

#[test]
fn semantic_json_changes_produce_a_different_hash() {
    let left = r#"{"id":"urn:badge:001","name":"Rust Badge","nested":{"a":1,"b":2}}"#;
    let right = r#"{"id":"urn:badge:001","name":"Rust Badge","nested":{"a":1,"b":3}}"#;

    let left_hash = normalized_json_hash_from_str(left).expect("left JSON should hash");
    let right_hash = normalized_json_hash_from_str(right).expect("right JSON should hash");

    assert_ne!(left_hash, right_hash);
}

#[test]
fn clr_formatting_only_variants_share_the_same_hash() {
    let compact =
        include_str!("../../../tests/fixtures/register_source/replay_hashing/clr_compact.json");
    let formatted =
        include_str!("../../../tests/fixtures/register_source/replay_hashing/clr_pretty.json");

    let compact_hash = normalized_json_hash_from_str(compact).expect("compact CLR should hash");
    let formatted_hash =
        normalized_json_hash_from_str(formatted).expect("formatted CLR should hash");

    assert_eq!(compact_hash, formatted_hash);
}

#[test]
fn open_badges_fixture_variants_share_the_same_hash() {
    let compact = include_str!(
        "../../../tests/fixtures/register_source/replay_hashing/open_badges_compact.json"
    );
    let formatted = include_str!(
        "../../../tests/fixtures/register_source/replay_hashing/open_badges_pretty.json"
    );

    let compact_hash = normalized_json_hash_from_str(compact).expect("compact badge should hash");
    let formatted_hash =
        normalized_json_hash_from_str(formatted).expect("formatted badge should hash");

    assert_eq!(compact_hash, formatted_hash);
}

#[test]
fn fixture_semantic_changes_produce_different_hashes() {
    let open_badges = include_str!(
        "../../../tests/fixtures/register_source/replay_hashing/open_badges_compact.json"
    );
    let clr =
        include_str!("../../../tests/fixtures/register_source/replay_hashing/clr_compact.json");
    let clr_conflict =
        include_str!("../../../tests/fixtures/register_source/replay_hashing/clr_conflict.json");

    let open_badges_hash =
        normalized_json_hash_from_str(open_badges).expect("badge payload should hash");
    let clr_hash = normalized_json_hash_from_str(clr).expect("CLR payload should hash");
    let clr_conflict_hash =
        normalized_json_hash_from_str(clr_conflict).expect("conflicting CLR should hash");

    assert_ne!(open_badges_hash, clr_hash);
    assert_ne!(clr_hash, clr_conflict_hash);
}

#[test]
fn normalized_hashing_is_deterministic_across_repeated_runs() {
    let payload = include_str!(
        "../../../tests/fixtures/register_source/replay_hashing/open_badges_pretty.json"
    );

    let first = normalized_json_hash_from_str(payload).expect("first hash should succeed");
    let second = normalized_json_hash_from_str(payload).expect("second hash should succeed");
    let third = normalized_json_hash_from_str(payload).expect("third hash should succeed");

    assert_eq!(first, second);
    assert_eq!(second, third);
}

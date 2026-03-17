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
    let compact = include_str!("../fixtures/register_source/replay_hashing/clr_compact.json");
    let formatted = include_str!("../fixtures/register_source/replay_hashing/clr_pretty.json");

    let compact_hash = normalized_json_hash_from_str(compact).expect("compact CLR should hash");
    let formatted_hash =
        normalized_json_hash_from_str(formatted).expect("formatted CLR should hash");

    assert_eq!(compact_hash, formatted_hash);
}

use core_shared::{AppError, AppResult, IdGenerator};
use serde_json::json;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::{
    memory_item::{MemoryItem, UnitType},
    source::DocumentType,
};

#[derive(Debug)]
pub struct NormalizationInput<'a> {
    pub source_id: Uuid,
    pub external_id: &'a str,
    pub title: &'a str,
    pub summary: Option<&'a str>,
    pub document_type: DocumentType,
    pub authoritative_content: &'a str,
    pub source_metadata: &'a serde_json::Value,
    pub created_at: OffsetDateTime,
}

pub fn normalize_source(
    input: &NormalizationInput<'_>,
    id_generator: &dyn IdGenerator,
) -> AppResult<Vec<MemoryItem>> {
    if input.authoritative_content.is_empty() {
        return Ok(vec![build_item(
            input,
            id_generator,
            0,
            UnitType::MetadataPlaceholder,
            0,
            0,
            "",
        )]);
    }

    match input.document_type {
        DocumentType::Json => Ok(vec![build_item(
            input,
            id_generator,
            0,
            UnitType::JsonDocument,
            0,
            input.authoritative_content.len() as u32,
            input.authoritative_content,
        )]),
        DocumentType::Text => {
            let ranges = split_paragraphs(input.authoritative_content);
            Ok(ranges
                .into_iter()
                .enumerate()
                .map(|(index, (start, end))| {
                    build_item(
                        input,
                        id_generator,
                        index as u32,
                        UnitType::Paragraph,
                        start as u32,
                        end as u32,
                        &input.authoritative_content[start..end],
                    )
                })
                .collect())
        }
        DocumentType::Markdown => {
            let ranges = split_markdown_sections(input.authoritative_content);
            Ok(ranges
                .into_iter()
                .enumerate()
                .map(|(index, (start, end))| {
                    build_item(
                        input,
                        id_generator,
                        index as u32,
                        UnitType::Section,
                        start as u32,
                        end as u32,
                        &input.authoritative_content[start..end],
                    )
                })
                .collect())
        }
    }
}

pub fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn normalized_json_hash(raw_body: &str) -> AppResult<String> {
    let value: serde_json::Value = serde_json::from_str(raw_body)
        .map_err(|_| AppError::validation("invalid supported-standard json payload"))?;
    Ok(sha256_hex(&canonical_json_string(&value)))
}

fn canonical_json_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_owned(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => serde_json::to_string(value).unwrap_or_default(),
        serde_json::Value::Array(values) => {
            let inner = values
                .iter()
                .map(canonical_json_string)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{inner}]")
        }
        serde_json::Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            let inner = entries
                .into_iter()
                .map(|(key, value)| {
                    let key = serde_json::to_string(key).unwrap_or_default();
                    format!("{key}:{}", canonical_json_string(value))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{inner}}}")
        }
    }
}

fn build_item(
    input: &NormalizationInput<'_>,
    id_generator: &dyn IdGenerator,
    sequence: u32,
    unit_type: UnitType,
    start_offset: u32,
    end_offset: u32,
    content: &str,
) -> MemoryItem {
    let content_hash = sha256_hex(content);
    let urn = id_generator
        .memory_item_urn(
            input.source_id,
            sequence,
            start_offset,
            end_offset,
            &content_hash,
        )
        .to_string();

    MemoryItem {
        urn,
        source_id: input.source_id,
        sequence,
        unit_type,
        start_offset,
        end_offset,
        version: "1".to_owned(),
        content: content.to_owned(),
        content_hash,
        item_metadata: json!({
            "unit_type": unit_type.as_str(),
            "start_offset": start_offset,
            "end_offset": end_offset,
            "version": "1",
        }),
        created_at: input.created_at,
        updated_at: input.created_at,
    }
}

fn split_paragraphs(content: &str) -> Vec<(usize, usize)> {
    split_on_blank_runs(content, false)
}

fn split_markdown_sections(content: &str) -> Vec<(usize, usize)> {
    let mut starts = Vec::new();
    let mut offset = 0usize;
    for line in content.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            starts.push(offset);
        }
        offset += line.len();
    }
    if starts.is_empty() {
        return split_on_blank_runs(content, true);
    }

    let mut ranges = Vec::new();
    for (index, start) in starts.iter().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(content.len());
        ranges.push((*start, trim_trailing_newlines(content, *start, end)));
    }
    ranges
}

fn split_on_blank_runs(content: &str, whole_if_single: bool) -> Vec<(usize, usize)> {
    let bytes = content.as_bytes();
    let mut ranges = Vec::new();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            let start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if content[start..cursor].contains("\n\n") {
                break;
            }
        }
        if cursor >= bytes.len() {
            break;
        }

        let start = cursor;
        let mut saw_blank = false;
        while cursor < bytes.len() {
            if bytes[cursor] == b'\n' {
                let mut probe = cursor;
                while probe < bytes.len() && bytes[probe].is_ascii_whitespace() {
                    probe += 1;
                }
                if content[cursor..probe].contains("\n\n") {
                    saw_blank = true;
                    break;
                }
            }
            cursor += 1;
        }

        let end = trim_trailing_newlines(content, start, cursor);
        if start < end {
            ranges.push((start, end));
        }

        if saw_blank {
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
        }
    }

    if ranges.is_empty() || (whole_if_single && ranges.len() == 1) {
        return vec![(0, trim_trailing_newlines(content, 0, content.len()))];
    }

    ranges
}

fn trim_trailing_newlines(content: &str, start: usize, mut end: usize) -> usize {
    while end > start && content.as_bytes()[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    end
}

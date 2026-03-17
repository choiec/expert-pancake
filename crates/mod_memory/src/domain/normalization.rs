use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use core_shared::{AppError, AppResult, IdGenerator};

use crate::domain::memory_item::{MEMORY_ITEM_VERSION, MemoryItem, MemoryUnitType};
use crate::domain::source::DocumentType;

pub struct NormalizationInput<'a> {
    pub source_id: Uuid,
    pub external_id: &'a str,
    pub title: &'a str,
    pub summary: Option<&'a str>,
    pub document_type: DocumentType,
    pub authoritative_content: &'a str,
    pub source_metadata: &'a Value,
    pub created_at: OffsetDateTime,
}

pub fn normalize_source(
    input: &NormalizationInput<'_>,
    id_generator: &dyn IdGenerator,
) -> AppResult<Vec<MemoryItem>> {
    if input.authoritative_content.is_empty() {
        return Ok(vec![build_placeholder_item(input, id_generator)]);
    }

    let parts = match input.document_type {
        DocumentType::Text => split_text(input.authoritative_content)
            .into_iter()
            .map(|(start, end)| (MemoryUnitType::Paragraph, start, end))
            .collect::<Vec<_>>(),
        DocumentType::Markdown => split_markdown(input.authoritative_content)
            .into_iter()
            .map(|(start, end)| (MemoryUnitType::Section, start, end))
            .collect::<Vec<_>>(),
        DocumentType::Json => vec![(
            MemoryUnitType::JsonDocument,
            0_u32,
            input.authoritative_content.len() as u32,
        )],
    };

    parts
        .into_iter()
        .enumerate()
        .map(|(sequence, (unit_type, start, end))| {
            let content = input.authoritative_content[start as usize..end as usize].to_owned();
            build_memory_item(
                input.source_id,
                sequence as u32,
                unit_type,
                start,
                end,
                content,
                Value::Object(Map::new()),
                input.created_at,
                id_generator,
            )
        })
        .collect()
}

pub fn normalized_json_hash_from_str(raw_body: &str) -> AppResult<String> {
    let value: Value = serde_json::from_str(raw_body)
        .map_err(|error| AppError::validation(format!("invalid JSON payload: {error}")))?;
    Ok(sha256_hex(canonical_json_string(&value).as_bytes()))
}

fn build_placeholder_item(
    input: &NormalizationInput<'_>,
    id_generator: &dyn IdGenerator,
) -> MemoryItem {
    let placeholder = json!({
        "system": {
            "placeholder": true,
            "title": input.title,
            "external_id": input.external_id,
            "summary": input.summary,
            "source_metadata": input.source_metadata,
        }
    });
    build_memory_item(
        input.source_id,
        0,
        MemoryUnitType::MetadataPlaceholder,
        0,
        0,
        String::new(),
        placeholder,
        input.created_at,
        id_generator,
    )
    .expect("placeholder construction is valid")
}

fn build_memory_item(
    source_id: Uuid,
    sequence: u32,
    unit_type: MemoryUnitType,
    start_offset: u32,
    end_offset: u32,
    content: String,
    item_metadata: Value,
    created_at: OffsetDateTime,
    id_generator: &dyn IdGenerator,
) -> AppResult<MemoryItem> {
    if start_offset > end_offset {
        return Err(AppError::validation("invalid normalization offsets"));
    }
    let content_hash = sha256_hex(content.as_bytes());
    let urn =
        id_generator.memory_item_urn(source_id, sequence, start_offset, end_offset, &content_hash);
    Ok(MemoryItem {
        urn,
        source_id,
        sequence,
        unit_type,
        start_offset,
        end_offset,
        version: MEMORY_ITEM_VERSION.to_owned(),
        content,
        content_hash,
        item_metadata,
        created_at,
        updated_at: created_at,
    })
}

fn split_text(content: &str) -> Vec<(u32, u32)> {
    let mut parts = Vec::new();
    let mut cursor = 0_usize;
    let mut current_start = None;
    let mut current_end = 0_usize;

    for chunk in content.split_inclusive('\n') {
        let chunk_start = cursor;
        cursor += chunk.len();
        let logical_end = chunk_start + trim_line_ending(chunk).len();
        if chunk.trim().is_empty() {
            if let Some(start) = current_start.take() {
                parts.push((start as u32, current_end as u32));
            }
            continue;
        }
        if current_start.is_none() {
            current_start = Some(chunk_start);
        }
        current_end = logical_end;
    }

    if let Some(start) = current_start {
        parts.push((start as u32, current_end as u32));
    }

    if parts.is_empty() && !content.is_empty() {
        parts.push((0, content.len() as u32));
    }
    parts
}

fn split_markdown(content: &str) -> Vec<(u32, u32)> {
    let mut heading_starts = Vec::new();
    let mut cursor = 0_usize;
    for chunk in content.split_inclusive('\n') {
        let chunk_start = cursor;
        cursor += chunk.len();
        if trim_line_ending(chunk).trim_start().starts_with('#') {
            heading_starts.push(chunk_start as u32);
        }
    }

    if heading_starts.is_empty() {
        return vec![(0, content.len() as u32)];
    }

    if heading_starts.first().copied() != Some(0) {
        heading_starts.insert(0, 0);
    }

    let mut sections = Vec::new();
    for window in heading_starts.windows(2) {
        sections.push((window[0], window[1]));
    }
    if let Some(last_start) = heading_starts.last().copied() {
        sections.push((last_start, content.len() as u32));
    }

    sections
        .into_iter()
        .filter(|(start, end)| start < end)
        .collect()
}

fn canonical_json_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => serde_json::to_string(string).expect("string serialization works"),
        Value::Array(items) => {
            let rendered = items
                .iter()
                .map(canonical_json_string)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{rendered}]")
        }
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let rendered = keys
                .into_iter()
                .map(|key| {
                    let key_json = serde_json::to_string(&key).expect("key serialization works");
                    let value_json = canonical_json_string(&map[&key]);
                    format!("{key_json}:{value_json}")
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{rendered}}}")
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\n', '\r'])
}

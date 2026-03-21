use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde_json::json;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::{middleware::error_response, state::AppState};

pub async fn get_source(State(state): State<AppState>, Path(source_id): Path<Uuid>) -> Response {
    match state.memory_ingest().get_source(source_id) {
        Ok(source) => Json(json!({
            "source_id": source.source_id,
            "external_id": source.external_id,
            "title": source.title,
            "summary": source.summary,
            "document_type": source.document_type,
            "created_at": source.created_at.format(&Rfc3339).unwrap_or_else(|_| source.created_at.unix_timestamp().to_string()),
            "updated_at": source.created_at.format(&Rfc3339).unwrap_or_else(|_| source.created_at.unix_timestamp().to_string()),
            "indexing_status": source.indexing_status,
            "source_metadata": source.source_metadata,
            "memory_items": source.memory_items.iter().map(|item| {
                json!({
                    "urn": item.urn,
                    "source_id": item.source_id,
                    "sequence": item.sequence,
                    "content": item.content,
                    "created_at": item.created_at.format(&Rfc3339).unwrap_or_else(|_| item.created_at.unix_timestamp().to_string()),
                    "updated_at": item.updated_at.format(&Rfc3339).unwrap_or_else(|_| item.updated_at.unix_timestamp().to_string()),
                    "item_metadata": item.item_metadata,
                    "source_metadata": item.source_metadata,
                })
            }).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(error) => error_response(error),
    }
}

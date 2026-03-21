use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde_json::json;
use time::format_description::well_known::Rfc3339;

use crate::{middleware::error_response, state::AppState};

pub async fn get_memory_item(State(state): State<AppState>, Path(urn): Path<String>) -> Response {
    match state.memory_ingest().get_memory_item(&urn) {
        Ok(item) => Json(json!({
            "urn": item.urn,
            "source_id": item.source_id,
            "sequence": item.sequence,
            "content": item.content,
            "item_metadata": item.item_metadata,
            "created_at": item.created_at.format(&Rfc3339).unwrap_or_else(|_| item.created_at.unix_timestamp().to_string()),
            "updated_at": item.updated_at.format(&Rfc3339).unwrap_or_else(|_| item.updated_at.unix_timestamp().to_string()),
            "source_metadata": item.source_metadata,
        }))
        .into_response(),
        Err(error) => error_response(error),
    }
}

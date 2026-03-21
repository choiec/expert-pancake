use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{middleware::error_response, state::AppState};

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    q: Option<String>,
    #[serde(rename = "source-id")]
    source_id: Option<Uuid>,
    #[serde(rename = "document-type")]
    document_type: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

pub async fn search_memory_items(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Response {
    match state
        .memory_ingest()
        .search_memory_items(mod_memory::SearchQuery {
            q: params.q,
            source_id: params.source_id,
            document_type: params.document_type,
            limit: params.limit.unwrap_or(20).min(100),
            offset: params.offset.unwrap_or(0),
        }) {
        Ok(result) => Json(json!({
            "total": result.total,
            "limit": result.limit,
            "offset": result.offset,
            "items": result.items.into_iter().map(|item| {
                json!({
                    "urn": item.urn,
                    "source_id": item.source_id,
                    "sequence": item.sequence,
                    "document_type": item.document_type,
                    "content_preview": item.content_preview,
                    "score": item.score,
                })
            }).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(error) => error_response(error),
    }
}

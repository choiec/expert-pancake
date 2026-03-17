use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    response::{IntoResponse, Response},
    routing::get,
};
use core_shared::{AppError, MemoryItemUrn};
use serde_json::json;

use crate::{
    middleware::{RequestContext, map_app_error},
    state::{AppState, MetricsLabels},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/memory-items/{urn}", get(get_memory_item))
}

async fn get_memory_item(
    State(state): State<AppState>,
    Path(urn): Path<String>,
    Extension(context): Extension<RequestContext>,
) -> Response {
    match get_memory_item_inner(state, urn).await {
        Ok(response) => response,
        Err(error) => map_app_error(error, &context),
    }
}

async fn get_memory_item_inner(state: AppState, urn: String) -> Result<Response, AppError> {
    let result = state
        .memory_ingest()
        .ok_or_else(|| AppError::storage_unavailable("memory ingest services are not configured"))?
        .get_memory_item()
        .execute(&MemoryItemUrn::new(urn))
        .await?;

    let payload = json!({
        "urn": result.urn,
        "source_id": result.source_id,
        "sequence": result.sequence,
        "content": result.content,
        "item_metadata": {
            "unit_type": result.unit_type,
            "start_offset": result.start_offset,
            "end_offset": result.end_offset,
            "version": result.version,
        },
        "created_at": result.created_at,
        "updated_at": result.updated_at,
    });

    let mut response = Json(payload).into_response();
    MetricsLabels::new()
        .with_document_type(result.document_type.as_str())
        .insert_response_extension(&mut response);
    Ok(response)
}

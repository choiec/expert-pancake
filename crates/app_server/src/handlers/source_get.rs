use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    response::{IntoResponse, Response},
    routing::get,
};
use core_shared::AppError;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    middleware::{RequestContext, map_app_error},
    state::{AppState, MetricsLabels},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/sources/{source-id}", get(get_source))
}

async fn get_source(
    State(state): State<AppState>,
    Path(source_id): Path<String>,
    Extension(context): Extension<RequestContext>,
) -> Response {
    match get_source_inner(state, source_id).await {
        Ok(response) => response,
        Err(error) => map_app_error(error, &context),
    }
}

async fn get_source_inner(state: AppState, source_id: String) -> Result<Response, AppError> {
    let source_id = Uuid::parse_str(&source_id)
        .map_err(|_| AppError::validation("source-id must be a valid UUID"))?;
    let result = state
        .memory_ingest()
        .ok_or_else(|| AppError::storage_unavailable("memory ingest services are not configured"))?
        .get_source()
        .execute(source_id)
        .await?;

    let memory_items = result
        .memory_items
        .into_iter()
        .map(|item| {
            json!({
                "urn": item.urn,
                "source_id": item.source_id,
                "sequence": item.sequence,
                "content": item.content,
                "item_metadata": {
                    "unit_type": item.unit_type,
                    "start_offset": item.start_offset,
                    "end_offset": item.end_offset,
                    "version": item.version,
                },
                "created_at": item.created_at,
                "updated_at": item.updated_at,
            })
        })
        .collect::<Vec<Value>>();

    let payload = json!({
        "source_id": result.source_id,
        "external_id": result.external_id,
        "title": result.title,
        "summary": result.summary,
        "document_type": result.document_type.as_str(),
        "created_at": result.created_at,
        "indexing_status": result.indexing_status.as_str(),
        "memory_items": memory_items,
    });

    let mut response = Json(payload).into_response();
    MetricsLabels::new()
        .with_document_type(result.document_type.as_str())
        .insert_response_extension(&mut response);
    Ok(response)
}

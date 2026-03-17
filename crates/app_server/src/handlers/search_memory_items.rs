use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use core_shared::AppError;
use mod_memory::application::search_memory_items::{
    SearchMemoryItemsQuery, SearchMemoryItemsResult,
};
use mod_memory::domain::source::DocumentType;

use crate::middleware::{RequestContext, map_app_error};
use crate::state::{AppState, MetricsLabels};

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct SearchMemoryItemsParams {
    pub q: Option<String>,
    pub source_id: Option<Uuid>,
    pub document_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchMemoryItemsResponse {
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
    pub items: Vec<SearchMemoryItemsHitResponse>,
}

#[derive(Debug, Serialize)]
pub struct SearchMemoryItemsHitResponse {
    pub urn: String,
    pub source_id: Uuid,
    pub sequence: u32,
    pub document_type: String,
    pub content_preview: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/search/memory-items", get(search_memory_items))
}

async fn search_memory_items(
    State(state): State<AppState>,
    Query(params): Query<SearchMemoryItemsParams>,
    Extension(context): Extension<RequestContext>,
) -> Response {
    let Some(memory_ingest) = state.memory_ingest() else {
        return map_app_error(
            AppError::internal("memory ingest services are not configured"),
            &context,
        );
    };

    let requested_document_type = params.document_type.clone();
    let query = match into_query(params) {
        Ok(query) => query,
        Err(error) => return map_app_error(error, &context),
    };

    match memory_ingest.search_memory_items().execute(query).await {
        Ok(result) => {
            let mut response = Json(into_response(result)).into_response();
            if let Some(document_type) = requested_document_type {
                MetricsLabels::new()
                    .with_document_type(document_type)
                    .insert_response_extension(&mut response);
            }
            response
        }
        Err(error) => map_app_error(error, &context),
    }
}

fn into_query(params: SearchMemoryItemsParams) -> Result<SearchMemoryItemsQuery, AppError> {
    Ok(SearchMemoryItemsQuery {
        query: params.q,
        source_id: params.source_id,
        document_type: params
            .document_type
            .as_deref()
            .map(parse_document_type)
            .transpose()?,
        limit: params.limit.unwrap_or(20),
        offset: params.offset.unwrap_or(0),
    })
}

fn parse_document_type(value: &str) -> Result<DocumentType, AppError> {
    match value {
        "text" => Ok(DocumentType::Text),
        "markdown" => Ok(DocumentType::Markdown),
        "json" => Ok(DocumentType::Json),
        _ => Err(AppError::validation(format!(
            "unsupported document-type '{value}'",
        ))),
    }
}

fn into_response(result: SearchMemoryItemsResult) -> SearchMemoryItemsResponse {
    SearchMemoryItemsResponse {
        total: result.total,
        limit: result.limit,
        offset: result.offset,
        items: result
            .items
            .into_iter()
            .map(|item| SearchMemoryItemsHitResponse {
                urn: item.urn,
                source_id: item.source_id,
                sequence: item.sequence,
                document_type: item.document_type.as_str().to_owned(),
                content_preview: item.content_preview,
                score: item.score,
            })
            .collect(),
    }
}

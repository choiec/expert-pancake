use axum::Json;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use core_shared::{ApiError, error_body};
use mod_memory::domain::credential::{CredentialFamily, SearchCredentialsQuery};

use crate::middleware::RequestId;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SearchQueryParams {
    q: Option<String>,
    family: Option<CredentialFamily>,
    #[serde(rename = "issuer-id")]
    issuer_id: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

pub async fn credential_search(
    State(state): State<AppState>,
    request_id: Option<axum::extract::Extension<RequestId>>,
    Query(query): Query<SearchQueryParams>,
) -> Response {
    let request_id = request_id
        .as_ref()
        .map(|id| id.0.0.clone())
        .unwrap_or_else(|| "unknown-request".to_string());

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    if !(1..=100).contains(&limit) {
        return (
            ApiError::invalid_input("Query parameter `limit` must be between 1 and 100", None)
                .status(),
            Json(error_body(
                &ApiError::invalid_input("Query parameter `limit` must be between 1 and 100", None),
                &request_id,
            )),
        )
            .into_response();
    }

    match state
        .module
        .search_service
        .search(SearchCredentialsQuery {
            q: query.q,
            family: query.family,
            issuer_id: query.issuer_id,
            limit,
            offset,
        })
        .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => (error.status(), Json(error_body(&error, &request_id))).into_response(),
    }
}

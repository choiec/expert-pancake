use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};

use core_shared::{decode_credential_id, error_body};

use crate::middleware::RequestId;
use crate::state::AppState;

pub async fn credential_get(
    State(state): State<AppState>,
    request_id: Option<axum::extract::Extension<RequestId>>,
    Path(credential_id): Path<String>,
) -> Response {
    let request_id = request_id
        .as_ref()
        .map(|id| id.0.0.clone())
        .unwrap_or_else(|| "unknown-request".to_string());
    let credential_id = decode_credential_id(&credential_id);

    match state.module.get_service.get(&credential_id).await {
        Ok(credential) => {
            axum::response::IntoResponse::into_response(Json(credential.response_document()))
        }
        Err(error) => (error.status(), Json(error_body(&error, &request_id))).into_response(),
    }
}

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::state::{AppState, HealthResponse, ReadinessResponse};

pub async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        service: "alive",
    })
}

pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let authoritative_ready = state.module.authoritative_probe.is_ready().await;
    let search_ready = state.module.search_probe.is_ready().await;

    let response = if authoritative_ready && search_ready {
        ReadinessResponse {
            status: "ready",
            authoritative_store: "ready",
            search: "ready",
        }
    } else if authoritative_ready {
        ReadinessResponse {
            status: "degraded",
            authoritative_store: "ready",
            search: "degraded",
        }
    } else {
        ReadinessResponse {
            status: "unavailable",
            authoritative_store: "unavailable",
            search: if search_ready { "ready" } else { "degraded" },
        }
    };

    let status = if authoritative_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(response))
}

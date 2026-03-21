use axum::{Json, extract::State};
use serde_json::json;

use crate::state::{AppState, ProbeStatus};

pub async fn health(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ready",
        "components": {
            "service": "ready",
        }
    }))
}

pub async fn ready(State(state): State<AppState>) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let snapshot = state.readiness().await;
    let status = if snapshot.database == ProbeStatus::Down {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    } else {
        axum::http::StatusCode::OK
    };
    let overall = match (snapshot.database, snapshot.search) {
        (ProbeStatus::Down, _) => "down",
        (_, ProbeStatus::Degraded) => "degraded",
        _ => "ready",
    };
    (
        status,
        Json(json!({
            "status": overall,
            "components": {
                "service": probe(snapshot.service),
                "database": probe(snapshot.database),
                "search": probe(snapshot.search),
            }
        })),
    )
}

fn probe(status: ProbeStatus) -> &'static str {
    match status {
        ProbeStatus::Ready => "ready",
        ProbeStatus::Degraded => "degraded",
        ProbeStatus::Down => "down",
    }
}

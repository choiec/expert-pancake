use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};

use crate::state::{AppState, ProbeSnapshot, ProbeStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: ProbeStatus,
    pub components: HealthComponents,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthComponents {
    pub service: ProbeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadinessResponse {
    pub status: ProbeStatus,
    pub components: ReadinessComponents,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadinessComponents {
    pub service: ProbeStatus,
    pub database: ProbeStatus,
    pub search: ProbeStatus,
}

/// Probe routes are kept isolated here so probe semantics remain explicit and
/// easy to validate against the OpenAPI contract.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
}

/// `/health` is local-only liveness.
///
/// It intentionally does not read application state or probe external services,
/// so it stays fast and returns service-local status only.
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: ProbeStatus::Ready,
        components: HealthComponents {
            service: ProbeStatus::Ready,
        },
    })
}

/// `/ready` is authoritative readiness.
///
/// SurrealDB controls the HTTP status because it is the authoritative write path.
/// Meilisearch degradation is surfaced in the body but does not fail readiness
/// while the database remains ready, matching the published OpenAPI semantics.
async fn ready(State(state): State<AppState>) -> (StatusCode, Json<ReadinessResponse>) {
    let probe = state.readiness().await;
    let status = overall_status(probe);
    let http_status = if probe.database == ProbeStatus::Down {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (
        http_status,
        Json(ReadinessResponse {
            status,
            components: ReadinessComponents {
                service: probe.service,
                database: probe.database,
                search: probe.search,
            },
        }),
    )
}

fn overall_status(probe: ProbeSnapshot) -> ProbeStatus {
    if probe.database == ProbeStatus::Down {
        ProbeStatus::Down
    } else if probe.search == ProbeStatus::Ready {
        ProbeStatus::Ready
    } else {
        ProbeStatus::Degraded
    }
}

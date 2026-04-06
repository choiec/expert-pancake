use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use tower_http::trace::TraceLayer;

use crate::handlers::{credential_get, credential_register, credential_search, health, ready};
use crate::middleware::request_id;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/credentials/register", post(credential_register))
        .route("/credentials/{credential_id}", get(credential_get))
        .route("/credentials/search", get(credential_search))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(request_id))
        .with_state(state)
}

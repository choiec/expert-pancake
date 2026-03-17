use axum::{Router, extract::DefaultBodyLimit, middleware};

use crate::{handlers, middleware as app_middleware, state::AppState};

/// Central router composition point.
///
/// Future US1-US4 route modules must be merged here so request-id, trace-context,
/// error mapping, and latency metrics stay applied automatically across the app.
pub fn build_router(state: AppState) -> Router {
    handlers::routes()
        .with_state(state.clone())
        .layer(DefaultBodyLimit::max(state.max_request_body_bytes()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            app_middleware::latency_metrics,
        ))
        .layer(middleware::from_fn(app_middleware::request_context))
}

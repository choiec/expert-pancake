use axum::{Router, extract::DefaultBodyLimit, middleware};

use crate::{handlers, middleware as app_middleware, state::AppState};

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

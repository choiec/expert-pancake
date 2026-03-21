use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::{handlers, state::AppState};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        .route(
            "/sources/register",
            post(handlers::source_register::register_source),
        )
        .route(
            "/sources/{source_id}",
            get(handlers::source_get::get_source),
        )
        .route(
            "/memory-items/{urn}",
            get(handlers::memory_item_get::get_memory_item),
        )
        .route(
            "/search/memory-items",
            get(handlers::search_memory_items::search_memory_items),
        )
        .layer(middleware::from_fn(crate::middleware::request_context))
        .with_state(state)
}

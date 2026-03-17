use axum::Router;

use crate::state::AppState;

pub mod health;
pub mod memory_item_get;
pub mod search_memory_items;
pub mod source_get;
pub mod source_register;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(source_register::routes())
        .merge(memory_item_get::routes())
        .merge(search_memory_items::routes())
        .merge(source_get::routes())
}

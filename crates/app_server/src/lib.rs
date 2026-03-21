pub mod config;
pub mod handlers;
pub mod middleware;
pub mod router;
pub mod state;

use axum::Router;
use core_shared::StartupError;

use crate::{config::AppConfig, state::AppState};

pub async fn build_app(config: AppConfig) -> Result<(Router, AppState), StartupError> {
    let state = AppState::bootstrap(config).await?;
    Ok((router::build_router(state.clone()), state))
}

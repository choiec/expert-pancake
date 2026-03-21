use std::net::SocketAddr;

use app_server::{AppState, build_router};
use core_infra::build_infra_bundle;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();

    let listen_addr = std::env::var("APP_LISTEN_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
        .parse::<SocketAddr>()
        .expect("APP_LISTEN_ADDR must be a valid socket address");

    let bundle = build_infra_bundle();
    let state = AppState::new(bundle.module);
    let listener = tokio::net::TcpListener::bind(listen_addr)
        .await
        .expect("listener binds");

    axum::serve(listener, build_router(state))
        .await
        .expect("server runs");
}

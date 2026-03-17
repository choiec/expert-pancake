use app_server::{build_app, config::AppConfig};
use core_shared::StartupError;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    init_tracing();

    let config = AppConfig::from_env()?;
    let listen_addr = config.http.listen_addr;
    let (router, _) = build_app(config).await?;

    let listener =
        TcpListener::bind(listen_addr)
            .await
            .map_err(|error| StartupError::ServerBind {
                address: listen_addr.to_string(),
                reason: error.to_string(),
            })?;

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|error| StartupError::ServerStart {
            reason: error.to_string(),
        })
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hyper=warn,tower_http=warn"));

    let _ = fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .json()
        .try_init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

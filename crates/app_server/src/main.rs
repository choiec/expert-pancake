use std::error::Error;

use app_server::{build_app, config::AppConfig};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().with_target(false).init();

    let config = AppConfig::from_env()?;
    let listen_addr = config.http.listen_addr;
    let (app, _state) = build_app(config).await?;
    let listener = TcpListener::bind(listen_addr).await?;

    tracing::info!(%listen_addr, "app_server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

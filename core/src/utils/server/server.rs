use crate::state::server_common::app_state::ServerAppState;
use anyhow::{Context, Result};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::IntoResponse;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{AllowHeaders, CorsLayer};
use tracing::info;

pub async fn start_server(server_app_state: Arc<ServerAppState>, app: Router) -> Result<()> {
    let allowed_origins = server_app_state
        .config
        .cors
        .allowed_origins
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(AllowHeaders::any())
        .allow_credentials(false);

    let app = app.fallback(handler_404).layer(cors);

    let addr: SocketAddr = format!(
        "{}:{}",
        server_app_state.config.server.host, server_app_state.config.server.port
    )
    .parse()
    .context("Invalid host or port configuration")?;

    info!("Starting server on {}...", addr);
    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

pub async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found")
}

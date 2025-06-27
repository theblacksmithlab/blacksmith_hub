use crate::state::server_common::app_state::ServerAppState;
use anyhow::{Context, Result};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::options;
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
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

    let app = app
        .route("/{*path}", options(|| async { StatusCode::OK }))
        .fallback(handler_404)
        .layer(cors);

    let tls_config = RustlsConfig::from_pem_file(
        &server_app_state.config.tls.cert_path,
        &server_app_state.config.tls.key_path,
    )
    .await?;

    let addr: SocketAddr = format!(
        "{}:{}",
        server_app_state.config.server.host, server_app_state.config.server.port
    )
        .parse()
        .context("Invalid host or port configuration")?;

    info!("Starting server on {}...", addr);

    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

pub async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found")
}

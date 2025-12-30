use anyhow::{Context, Result};
use axum::routing::post;
use axum::Router;
use config::{Config, File};
use core::config::server_config::AppConfig;
use core::utils::server::server::start_server;
use dotenv::dotenv;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod handlers;

use crate::handlers::handle_transcribe;
use core::state::server_common::app_state::ServerAppState;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    info!("Starting Whisper Transcription Service...");

    let config_path = get_config_path();
    info!("Loading config from: {}", config_path);

    let config_builder = Config::builder().add_source(File::with_name(&config_path));
    let config: AppConfig = config_builder
        .build()
        .context("Failed to build config")?
        .try_deserialize()
        .context("Failed to deserialize config")?;

    let config = Arc::new(config);

    let server_app_state = Arc::new(ServerAppState::new(config.clone()));

    let router = get_whisper_router();

    info!(
        "Whisper service configured on {}:{}",
        server_app_state.config.server.host, server_app_state.config.server.port
    );

    start_server(server_app_state, router).await
}

fn get_whisper_router() -> Router {
    Router::new().route("/transcribe", post(handle_transcribe))
}

fn get_config_path() -> String {
    if let Ok(path) = std::env::var("CONFIG_PATH") {
        return path;
    }

    let container_path = "/app/config.yaml";
    if std::path::Path::new(container_path).exists() {
        return container_path.to_string();
    }

    "tooling/whisper/config.yaml".to_string()
}

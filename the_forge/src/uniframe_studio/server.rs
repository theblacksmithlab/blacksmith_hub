use crate::uniframe_studio::handlers::{
    get_dubbing_pipeline_status, prepare_dubbing_pipeline, start_dubbing_pipeline,
};
use anyhow::Result;
use axum::routing::{get, post};
use axum::Router;
use core::state::server_common::app_state::ServerAppState;
use core::state::uniframe_studio::app_state::UniframeStudioAppState;
use core::utils::server::server::start_server;
use http::StatusCode;
use rustls::crypto::{aws_lc_rs, CryptoProvider};
use std::sync::Arc;
use tracing::info;

pub async fn start_uniframe_studio_server(server_app_state: Arc<ServerAppState>) -> Result<()> {
    let _ = CryptoProvider::install_default(aws_lc_rs::default_provider());

    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;

    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let dubbing_service_url =
        std::env::var("DUBBING_SERVICE_URL").unwrap_or("http://localhost:8000".to_string());

    let uniframe_studio_app_state =
        Arc::new(UniframeStudioAppState::new(s3_client, dubbing_service_url));

    let router = get_uniframe_studio_router(uniframe_studio_app_state);

    info!("Starting Uniframe Studio server...");

    start_server(server_app_state, router).await
}

fn get_uniframe_studio_router(uniframe_studio_app_state: Arc<UniframeStudioAppState>) -> Router {
    Router::new()
        .route(
            "/api/dubbing/pipeline/prepare",
            post(prepare_dubbing_pipeline).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/dubbing/pipeline/start",
            post(start_dubbing_pipeline).options(|| async { StatusCode::OK }),
        )
        .route(
            "/api/dubbing/pipeline/{pipeline_id}/status",
            get(get_dubbing_pipeline_status).options(|| async { StatusCode::OK }),
        )
        .with_state(uniframe_studio_app_state)
}

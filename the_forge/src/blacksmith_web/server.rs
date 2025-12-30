use crate::blacksmith_web::handlers::{
    handle_blacksmith_web_chat_fetch, handle_blacksmith_web_tts_request,
    handle_blacksmith_web_user_request,
};
use anyhow::Result;
use async_openai::Client as LLM_Client;
use axum::routing::{get, post};
use axum::Router;
use core::local_db::local_db::setup_app_db_pool;
use core::models::common::app_name::AppName;
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;
use core::state::server_common::app_state::ServerAppState;
use core::utils::server::server::start_server;
use http::StatusCode;
use qdrant_client::Qdrant;
use std::env;
use std::sync::Arc;
use tracing::info;

pub async fn start_blacksmith_web_server(server_app_state: Arc<ServerAppState>) -> Result<()> {
    let qdrant_client = Arc::new(Qdrant::from_url(&env::var("QDRANT_LOCAL_URL")?).build()?);

    let llm_client = LLM_Client::new();

    let blacksmith_lab_db_pool = setup_app_db_pool(&AppName::BlacksmithWeb).await?;

    let blacksmith_web_app_state = Arc::new(BlacksmithWebAppState::new(
        llm_client,
        qdrant_client,
        blacksmith_lab_db_pool,
    ));

    let router = get_blacksmith_web_router(blacksmith_web_app_state);

    info!("Starting Blacksmith Web server...");

    start_server(server_app_state, router).await
}

fn get_blacksmith_web_router(blacksmith_web_app_state: Arc<BlacksmithWebAppState>) -> Router {
    Router::new()
        .route(
            "/blacksmith_web_user_request",
            post(handle_blacksmith_web_user_request).options(|| async { StatusCode::OK }),
        )
        .route(
            "/blacksmith_web_chat_fetch",
            get(handle_blacksmith_web_chat_fetch).options(|| async { StatusCode::OK }),
        )
        .route(
            "/blacksmith_web_tts_request",
            post(handle_blacksmith_web_tts_request).options(|| async { StatusCode::OK }),
        )
        .with_state(blacksmith_web_app_state)
}

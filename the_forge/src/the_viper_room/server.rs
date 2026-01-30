use anyhow::Result;
use async_openai::Client as OpenAIClient;
use axum::routing::{get, post};
use axum::Router;
use core::ai::anthropic_client::AnthropicClient;
use core::ai::google_client::GoogleClient;
use core::state::server_common::app_state::ServerAppState;
use core::state::the_viper_room::app_state::TheViperRoomAppState;
use core::utils::server::server::start_server;
use http::StatusCode;
use std::sync::Arc;
use tracing::info;

pub async fn start_the_viper_room_server(server_app_state: Arc<ServerAppState>) -> Result<()> {
    let openai_client = OpenAIClient::new();
    let anthropic_client = AnthropicClient::new()?;
    let google_client = GoogleClient::new()?;

    let the_viper_room_app_state = Arc::new(TheViperRoomAppState::new(
        openai_client,
        anthropic_client,
        google_client,
    ));

    let router = get_the_viper_room_router(the_viper_room_app_state);

    info!("Starting The Viper Room server...");

    start_server(server_app_state, router).await
}

fn get_the_viper_room_router(the_viper_room_app_state: Arc<TheViperRoomAppState>) -> Router {
    use crate::the_viper_room::handlers::handle_the_viper_room_user_request;
    use core::utils::common::get_user_avatar;

    Router::new()
        .route(
            "/the_viper_room_user_request",
            post(handle_the_viper_room_user_request).options(|| async { StatusCode::OK }),
        )
        .route(
            "/the_viper_room_avatar_request",
            get(get_user_avatar).options(|| async { StatusCode::OK }),
        )
        .with_state(the_viper_room_app_state)
}

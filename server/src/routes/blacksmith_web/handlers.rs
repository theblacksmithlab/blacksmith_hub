use std::sync::Arc;
use axum::extract::State;
use axum::Json;
use tracing::log::info;
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;
use core::models::blacksmith_web::blacksmith_web::{BlacksmithWebUserAction, BlacksmithWebServerResponse};
use core::models::common::app_name::AppName;
use crate::routes::blacksmith_web::default_message_handler::default_message_handler;

pub(crate) async fn handle_blacksmith_web_user_action(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Json(action): Json<BlacksmithWebUserAction>,
) -> Json<BlacksmithWebServerResponse> {
    let app_name = AppName::W3AWeb;
    let chat_id = action.user_id;
    let action_text = action.text.as_str();
    let user_raw_request = action.text.to_string();
    info!(
        "Got message: {} from: {}",
        user_raw_request,
        chat_id
    );

    let response = default_message_handler(
        action_text.to_string(),
        blacksmith_web_app_state,
        chat_id,
        app_name
    ).await;

    Json(BlacksmithWebServerResponse { text: response })
}
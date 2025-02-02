use std::sync::Arc;
use axum::extract::State;
use axum::Json;
use tracing::log::info;
use tracing::warn;
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;
use core::models::blacksmith_web::blacksmith_web::{BlacksmithWebUserAction, BlacksmithWebServerResponse};
use core::models::common::app_name::AppName;
use crate::routes::blacksmith_web::default_message_handler::default_message_handler;

pub(crate) async fn handle_blacksmith_web_user_action(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Json(action): Json<BlacksmithWebUserAction>,
) -> Json<BlacksmithWebServerResponse> {
    let app_name: AppName = match action.app_name.as_str() {
        "W3AWeb" => AppName::W3AWeb,
        _ => {
            warn!("Unsupported app type of the app: {}", action.app_name);
            AppName::BlacksmithWeb
        }
    };
    
    let user_id = action.user_id;
    let action_text = action.text.as_str();
    info!(
        "Got message: {} from user: {}",
        action_text,
        user_id
    );

    let response = default_message_handler(
        action_text.to_string(),
        blacksmith_web_app_state,
        user_id,
        app_name
    ).await;

    Json(BlacksmithWebServerResponse { text: response })
}
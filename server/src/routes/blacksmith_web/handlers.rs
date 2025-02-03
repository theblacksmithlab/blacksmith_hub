use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use axum::extract::{Query, State};
use axum::Json;
use tracing::log::info;
use tracing::warn;
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;
use core::models::blacksmith_web::blacksmith_web::{BlacksmithWebUserAction, BlacksmithWebServerResponse};
use core::models::common::app_name::AppName;
use crate::routes::blacksmith_web::default_message_handler::default_message_handler;
use core::models::blacksmith_web::blacksmith_web::ChatMessage;
use core::local_db::local_db::fetch_chat_history_from_db;

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

    let temp_user_id = 1;
    
    let response = default_message_handler(
        action_text.to_string(),
        blacksmith_web_app_state,
        temp_user_id,
        app_name
    ).await;

    Json(BlacksmithWebServerResponse { text: response })
}

pub(crate) async fn handle_blacksmith_web_chat_fetch(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<ChatMessage>> {
    let user_id = match params.get("user_id") {
        Some(id) => id.clone(),
        None => return Json(vec![]),
    };

    let app_name = match params.get("app_name") {
        Some(name) => match AppName::from_str(name) {
            Ok(app) => app,
            Err(_) => return Json(vec![]),
        },
        None => return Json(vec![]),
    };

    match fetch_chat_history_from_db(&blacksmith_web_app_state.local_db_pool, &user_id, app_name.as_str()).await {
        Ok(chat_history) => Json(chat_history),
        Err(_) => Json(vec![]),
    }
}
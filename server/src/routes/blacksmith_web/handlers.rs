use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use axum::extract::{Query, State};
use axum::Json;
use tracing::log::info;
use tracing::warn;
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;
use core::models::blacksmith_web::blacksmith_web::{BlacksmithWebUserAction, BlacksmithWebServerResponse, BlacksmithWebTTSRequest, BlacksmithWebTTSResponse};
use core::models::common::app_name::AppName;
use crate::routes::blacksmith_web::default_message_handler::default_message_handler;
use core::models::blacksmith_web::blacksmith_web::ChatMessage;
use core::local_db::local_db::fetch_chat_history_from_db;
use tokio::time::{sleep, Duration};

pub(crate) async fn handle_blacksmith_web_user_action(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Json(action): Json<BlacksmithWebUserAction>,
) -> Json<BlacksmithWebServerResponse> {
    let app_name = match AppName::from_str(&action.app_name) {
        Ok(app) => app,
        Err(_) => {
            warn!("Unsupported app type: {}", action.app_name);
            AppName::BlacksmithWeb
        }
    };
    
    let user_id = action.user_id;
    let action_text = action.text;
    
    info!(
        "Got message: {} from user: {}",
        action_text,
        user_id
    );
    
    let response = default_message_handler(
        &action_text,
        blacksmith_web_app_state,
        &user_id,
        app_name
    ).await;

    Json(BlacksmithWebServerResponse { text: response })
}

pub(crate) async fn handle_blacksmith_web_chat_fetch(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<ChatMessage>> {
    info!("Fetching chat history for web application");
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

    info!("Fetching history for user_id={} with app_name={}", user_id, app_name);
    match fetch_chat_history_from_db(&blacksmith_web_app_state.local_db_pool, &user_id, app_name.as_str()).await {
        Ok(chat_history) => Json(chat_history),
        Err(_) => Json(vec![]),
    }
}

pub(crate) async fn handle_blacksmith_web_tts_input(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Json(action): Json<BlacksmithWebTTSRequest>,
) -> Json<BlacksmithWebTTSResponse> {
    let user_id = action.user_id;
    let action_text = action.text;

    info!(
        "Got message: {} from user: {}",
        action_text,
        user_id
    );

    sleep(Duration::from_secs(5)).await;
    
    Json(BlacksmithWebTTSResponse { audio_data: "All's good!".to_string() })
}
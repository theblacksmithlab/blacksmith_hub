use std::sync::Arc;
use axum::extract::State;
use axum::Json;
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;
use core::models::blacksmith_web::blacksmith_web::{BlacksmithWebUserAction, BlacksmithWebServerResponse};

pub(crate) async fn handle_blacksmith_web_user_action(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Json(action): Json<BlacksmithWebUserAction>,
) -> Json<BlacksmithWebServerResponse> {
    let action_text = action.message.as_str();
    
    if action_text == "test" {
        Json(BlacksmithWebServerResponse{
            text: "Всё работает! Отлично!".to_string(),
        }) 
    } else {
        Json(BlacksmithWebServerResponse{
            text: "И так все работает!".to_string(),
        })
    }
}
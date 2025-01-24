use std::sync::Arc;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use tracing::error;
use core::state::tg_bot::app_state::BotAppState;
use core::models::tg_bot::probiot::get_system_role_model::ProbiotRoleType;
use core::utils::common::get_system_role_or_fallback;
use core::ai::ai::{raw_llm_processing_json, raw_llm_processing};
use core::utils::common::LlmModel;

pub fn create_tts_button() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::default().append_row(vec![
        InlineKeyboardButton::callback("Озвучить ответ", "tts"),
    ])
}

pub async fn check_request_for_crap_content(
    user_raw_request: String,
    clarified_request: String,
    current_cache: String,
    app_state: Arc<BotAppState>
) -> anyhow::Result<bool> {
    let system_role = get_system_role_or_fallback(
        "probiot",
        ProbiotRoleType::CrapDetection,
        None);

    let llm_message = format!(
        "User query: {}\nClarified user query: {}\nCurrent cache: {}",
        user_raw_request, clarified_request, current_cache
    );

    let crap_detection_result = raw_llm_processing_json(system_role, llm_message, app_state, LlmModel::Complex).await?;

    let is_crap: bool = match serde_json::from_str::<serde_json::Value>(&crap_detection_result) {
        Ok(json) => json.get("is_crap").and_then(|v| v.as_bool()).unwrap_or(true),
        Err(err) => {
            error!("Failed to parse JSON: {}", err);
            true
        }
    };

    Ok(is_crap)
}

pub async fn clarify_request(user_raw_request: String, app_state: Arc<BotAppState>) -> anyhow::Result<String> {
    let system_role = get_system_role_or_fallback(
        "probiot",
        ProbiotRoleType::ClarifyRequest,
        None);

    match raw_llm_processing(system_role, user_raw_request.clone(), app_state, LlmModel::Complex).await {
        Ok(clarified_request) => Ok(clarified_request),
        Err(err) => {
            error!("Error in raw_llm_processing: {}", err);
            Ok(user_raw_request)
        }
    }
}

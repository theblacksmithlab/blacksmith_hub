use anyhow::Result;
use core::ai::common::common::{raw_llm_processing, raw_llm_processing_json};
use core::models::common::dialogue_cache::DialogueCache;
use core::models::common::system_messages::ProbiotMessages;
use core::models::tg_bot::probiot::get_system_role_model::ProbiotRoleType;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::common::get_system_role_or_fallback;
use core::utils::common::LlmModel;
use core::utils::tg_bot::tg_bot::get_user_message_count;
use std::sync::Arc;
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup};
use tracing::{error, info};

pub fn create_tts_button(chat_id: ChatId, message_id: String) -> InlineKeyboardMarkup {
    let callback_data = format!("tts:{}:{}", chat_id, message_id);
    InlineKeyboardMarkup::default().append_row(vec![InlineKeyboardButton::callback(
        "Озвучить ответ",
        &callback_data,
    )])
}

pub async fn save_tts_payload(
    app_state: Arc<BotAppState>,
    chat_id: ChatId,
    message_id: String,
    tts_payload: String,
) {
    let mut cache = app_state.temp_cache.lock().await;
    let dialogue_cache = cache
        .entry(chat_id)
        .or_insert_with(|| DialogueCache::new(100));
    dialogue_cache.add_tts_payload(message_id, tts_payload);
}

pub async fn get_and_remove_tts_payload(
    app_state: Arc<BotAppState>,
    chat_id: ChatId,
    message_id: String,
) -> Option<String> {
    let mut cache = app_state.temp_cache.lock().await;
    if let Some(dialogue_cache) = cache.get_mut(&chat_id) {
        dialogue_cache.get_and_remove_tts_payload(message_id)
    } else {
        None
    }
}

pub async fn check_request_for_crap_content(
    user_raw_request: String,
    clarified_request: String,
    current_cache: String,
    app_state: Arc<BotAppState>,
) -> Result<bool> {
    let system_role = get_system_role_or_fallback("probiot", ProbiotRoleType::CrapDetection, None);

    let llm_message = format!(
        "User's current query: {}\nUser's refined query: {}\nChat history: {}",
        user_raw_request, clarified_request, current_cache
    );

    let crap_detection_result =
        raw_llm_processing_json(system_role, llm_message, app_state, LlmModel::Light).await?;

    let is_crap: bool = match serde_json::from_str::<serde_json::Value>(&crap_detection_result) {
        Ok(json) => json
            .get("is_crap")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        Err(err) => {
            error!("Failed to parse JSON: {}", err);
            true
        }
    };

    Ok(is_crap)
}

pub async fn clarify_request(
    user_raw_request: String,
    current_cache: String,
    app_state: Arc<BotAppState>,
) -> Result<String> {
    let llm_message = format!(
        "User's current query: {}\nChat history: {}",
        user_raw_request, current_cache
    );

    let system_role = get_system_role_or_fallback("probiot", ProbiotRoleType::ClarifyRequest, None);

    match raw_llm_processing(system_role, llm_message, app_state, LlmModel::Complex).await {
        Ok(clarified_request) => {
            info!("User's raw request clarified successfully");
            Ok(clarified_request)
        }
        Err(err) => {
            error!("Error in raw_llm_processing: {}", err);
            Ok(user_raw_request)
        }
    }
}

pub async fn append_footer_if_needed(
    app_name: &str,
    llm_response: String,
    app_state: Arc<BotAppState>,
    chat_id: ChatId,
) -> Result<String> {
    let message_count = get_user_message_count(&app_state, chat_id).await;

    if message_count > 0 && message_count % 3 == 0 {
        let footer_message = get_message(
            Some(app_name),
            ProbiotMessages::ResponseFooter.as_str(),
            false,
        )
        .await?;
        Ok(format!("{}\n{}", llm_response, footer_message))
    } else {
        Ok(llm_response)
    }
}

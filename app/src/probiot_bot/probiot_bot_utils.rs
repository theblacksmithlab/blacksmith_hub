use anyhow::Result;
use core::ai::common::common::{raw_llm_processing, raw_llm_processing_json};
use core::models::common::app_name::AppName;
use core::models::common::dialogue_cache::DialogueCache;
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::ProbiotBotMessages;
use core::models::common::system_messages::W3ABotMessages;
use core::models::common::system_roles::{AppsSystemRoles, ProbiotRoleType, W3ARoleType};
use core::rag_system::types::RAGConfig;
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
    app_name: AppName,
) -> Result<bool> {
    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(ProbiotRoleType::CrapDetection)),
        AppName::W3ABot => Some(AppsSystemRoles::W3A(W3ARoleType::CrapDetection)),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "CrapDetection role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };
    
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
    app_name: AppName,
) -> Result<String> {
    let llm_message = format!(
        "User's current query: {}\nChat history: {}",
        user_raw_request, current_cache
    );

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(ProbiotRoleType::ClarifyRequest)),
        AppName::W3ABot => Some(AppsSystemRoles::W3A(W3ARoleType::ClarifyRequest)),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "ClarifyRequest role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

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
    llm_response: String,
    app_state: Arc<BotAppState>,
    chat_id: ChatId,
    app_name: AppName,
) -> Result<String> {
    let message_count = get_user_message_count(&app_state, chat_id).await;

    if message_count > 0 && message_count % 3 == 0 {
        let footer_message = match app_name {
            AppName::ProbiotBot => {
                get_message(AppsSystemMessages::Probiot(
                    ProbiotBotMessages::ResponseFooter,
                ))
                .await?
            }
            AppName::W3ABot => {
                get_message(AppsSystemMessages::W3ABot(W3ABotMessages::ResponseFooter)).await?
            }
            _ => "".to_string(),
        };

        if !footer_message.is_empty() {
            return Ok(format!("{}\n{}", llm_response, footer_message));
        }
    }

    Ok(llm_response)
}

pub fn _get_default_rag_config() -> RAGConfig {
    RAGConfig::Default {
        max_documents: 12,
        similarity_threshold: 0.3,
    }
}

pub fn get_advanced_rag_config() -> RAGConfig {
    RAGConfig::Advanced {
        base_max_documents: 5,
        base_similarity_threshold: 0.4,
        related_max_documents: 5,
        related_similarity_threshold: 0.4,
    }
}

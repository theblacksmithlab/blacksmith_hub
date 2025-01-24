use std::sync::Arc;
use teloxide::types::ChatId;
use anyhow::Result;
use tracing::error;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::{add_user_message_to_cache, get_cache_as_string};
use core::state::tg_bot::app_state::BotAppState;
use core::models::tg_bot::probiot::get_system_role_model::ProbiotRoleType;
use core::utils::common::get_system_role_or_fallback;
use core::ai::ai::{raw_llm_processing_json, raw_llm_processing};
use core::utils::common::LlmModel;


pub async fn process_user_raw_request(
    chat_id: ChatId,
    user_raw_request: String,
    app_state: Arc<BotAppState>,
    initiator_app_name: String
) -> Result<String> {
    add_user_message_to_cache(app_state.clone(), chat_id, user_raw_request.clone()).await;
    
    let clarified_request = clarify_request(user_raw_request.clone(), app_state.clone()).await?;

    let current_cache = get_cache_as_string(app_state.clone(), chat_id).await;
    
    let is_crap = check_request_for_crap_content(user_raw_request, clarified_request.clone(), current_cache, app_state.clone()).await?;

    if is_crap {
        let response_for_crap_request = get_message(Some(&initiator_app_name), "response_for_crap_request", false).await?;
        Ok(response_for_crap_request)
    } else {
        let response_for_valid_request = handle_valid_request().await?;
        Ok(response_for_valid_request)
    }
}

pub async fn check_request_for_crap_content(
    user_raw_request: String,
    clarified_request: String,
    current_cache: String,
    app_state: Arc<BotAppState>
) -> Result<bool> {
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

pub async fn clarify_request(user_raw_request: String, app_state: Arc<BotAppState>) -> Result<String> {
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

pub async fn handle_valid_request() ->Result<String> {
    Ok("Нормальный запрос, одобряю".to_string())
}
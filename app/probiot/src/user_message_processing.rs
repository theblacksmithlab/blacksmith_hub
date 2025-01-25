use crate::probiot_utils::{check_request_for_crap_content, clarify_request};
use anyhow::Result;
use core::ai::ai::raw_llm_processing;
use core::ai::ai::tokenize_and_truncate;
use core::models::common::qdrant_collection_manager::ApplicationManager;
use core::models::tg_bot::probiot::get_system_role_model::ProbiotRoleType;
use core::rag_system::get_results_via_rag_system::get_results_via_rag_system::get_results_via_rag_system;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::common::get_system_role_or_fallback;
use core::utils::common::LlmModel;
use core::utils::tg_bot::tg_bot::{add_user_message_to_cache, get_cache_as_string};
use std::sync::Arc;
use teloxide::types::ChatId;
use tracing::info;

pub async fn process_user_raw_request(
    chat_id: ChatId,
    user_raw_request: String,
    app_state: Arc<BotAppState>,
    initiator_app_name: String,
) -> Result<String> {
    add_user_message_to_cache(app_state.clone(), chat_id, user_raw_request.clone()).await;

    let clarified_request = clarify_request(user_raw_request.clone(), app_state.clone()).await?;

    let current_cache = get_cache_as_string(app_state.clone(), chat_id).await;

    let is_crap = check_request_for_crap_content(
        user_raw_request.clone(),
        clarified_request.clone(),
        current_cache.clone(),
        app_state.clone(),
    )
    .await?;

    if is_crap {
        let response_for_crap_request = get_message(
            Some(&initiator_app_name),
            "response_for_crap_request",
            false,
        )
        .await?;
        Ok(response_for_crap_request)
    } else {
        let response_for_valid_request = handle_valid_request(
            user_raw_request,
            clarified_request,
            app_state,
            current_cache,
        )
        .await?;
        Ok(response_for_valid_request)
    }
}

pub async fn handle_valid_request(
    user_raw_request: String,
    clarified_request: String,
    app_state: Arc<BotAppState>,
    current_cache: String,
) -> Result<String> {
    let qdrant_collection_manager = ApplicationManager::new();

    let collection_names: Vec<String> = qdrant_collection_manager
        .get_probiot_collections()
        .iter()
        .map(|collection| collection.as_str().to_string())
        .collect();

    let search_results = get_results_via_rag_system(
        clarified_request.clone(), // Check results providing user_raw_request/clarified_request
        collection_names,
        10,
        0.3,
        app_state.clone(),
    )
    .await?;

    let search_results_text_payload = search_results.context;

    let processed_data = tokenize_and_truncate(search_results_text_payload.clone())
        .await
        .unwrap_or_else(|_| search_results_text_payload);

    let llm_message = format!(
        "User's current query: {}\nUser's refined query: {}\nChat history: {}\nUseful information from the database: {}",
        user_raw_request, clarified_request, current_cache, processed_data
    );

    info!("TEMP log: LLM message: {}", llm_message);

    let system_role = get_system_role_or_fallback("probiot", ProbiotRoleType::MainProcessing, None);

    let llm_response =
        raw_llm_processing(system_role, llm_message, app_state, LlmModel::Complex).await?;

    Ok(llm_response)
}

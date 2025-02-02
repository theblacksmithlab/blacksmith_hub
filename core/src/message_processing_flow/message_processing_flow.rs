use crate::rag_system::get_default_rag_config;
use crate::message_processing_flow::check_request_for_crap_content::check_request_for_crap_content;
use crate::message_processing_flow::clarify_request::clarify_request;
use anyhow::Result;
use crate::ai::common::common::raw_llm_processing;
use crate::ai::common::common::tokenize_and_truncate;
use crate::models::common::app_name::AppName;
use crate::models::common::qdrant_collection_manager::AppsCollections;
use crate::models::common::system_roles::ProbiotRoleType;
use crate::rag_system::get_results_via_rag_system::get_results_via_rag_system::get_results_via_rag_system;
use crate::utils::common::get_system_role_or_fallback;
use crate::utils::common::LlmModel;
use crate::utils::tg_bot::tg_bot::{add_user_message_to_cache, get_cache_as_string};
use std::sync::Arc;
use tracing::{error, info};
use crate::models::common::system_roles::{AppsSystemRoles, W3ARoleType};
use crate::state::llm_client_init_trait::LlmProcessing;
use crate::state::qdrant_client_init_trait::QdrantClientInit;
use crate::temp_cache::temp_cache_traits::TempCacheInit;

pub async fn process_user_raw_request<T: LlmProcessing + QdrantClientInit + TempCacheInit + Send + Sync>(
    chat_id: i64,
    user_raw_request: String,
    app_state: Arc<T>,
    app_name: AppName,
) -> Result<String> {
    info!("Start processing user raw request...");
    add_user_message_to_cache(app_state.clone(), chat_id, user_raw_request.clone()).await;

    let current_cache = get_cache_as_string(app_state.clone(), chat_id).await;

    let clarified_request = clarify_request(
        user_raw_request.clone(),
        current_cache.clone(),
        app_state.clone(),
        app_name.clone(),
    )
        .await?;

    let is_crap = check_request_for_crap_content(
        user_raw_request.clone(),
        clarified_request.clone(),
        current_cache.clone(),
        app_state.clone(),
        app_name.clone(),
    )
        .await?;

    if is_crap {
        info!("Crap request detected, sending message to handle_crap_request fn");
        let response_for_crap_request = handle_crap_request(
            user_raw_request,
            app_state.clone(),
            clarified_request.clone(),
            app_name.clone(),
        )
            .await?;
        Ok(response_for_crap_request)
    } else {
        info!("Valid request detected, sending message to handle_valid_request fn");
        let response_for_valid_request = handle_valid_request(
            user_raw_request,
            clarified_request,
            app_state,
            current_cache,
            app_name.clone(),
        )
            .await?;
        Ok(response_for_valid_request)
    }
}

pub async fn handle_valid_request<T: LlmProcessing + QdrantClientInit + Send + Sync>(
    user_raw_request: String,
    clarified_request: String,
    app_state: Arc<T>,
    current_cache: String,
    app_name: AppName,
) -> Result<String> {
    let collection_names: Vec<String> =
        AppsCollections::all_collections_for_app(app_name.clone())
            .iter()
            .map(|collection| collection.as_str().to_string())
            .collect();

    // RAG system mode
    let rag_config = get_default_rag_config();

    info!("TEMP log: collection names: {:?}", collection_names);

    let search_results = get_results_via_rag_system(
        clarified_request.clone(),
        collection_names,
        rag_config,
        app_state.clone(),
    )
        .await?;

    let rag_system_search_result_payload = search_results.context;

    let processed_data = tokenize_and_truncate(rag_system_search_result_payload.clone())
        .await
        .unwrap_or_else(|_| rag_system_search_result_payload);

    let llm_message = format!(
        "User's current query: {}\nUser's refined query: {}\nChat history: {}\nUseful information from the database:\n{}",
        user_raw_request, clarified_request, current_cache, processed_data
    );

    info!("TEMP log: LLM message: {}", llm_message);

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(ProbiotRoleType::MainProcessing)),
        AppName::W3ABot => Some(AppsSystemRoles::W3A(W3ARoleType::MainProcessing)),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::MainProcessing)),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "MainProcessing role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    let llm_response =
        raw_llm_processing(system_role, llm_message, app_state, LlmModel::Complex).await?;

    Ok(llm_response)
}

pub async fn handle_crap_request<T: LlmProcessing + Send + Sync>(
    user_raw_request: String,
    app_state: Arc<T>,
    current_cache: String,
    app_name: AppName,
) -> Result<String> {
    let llm_message = format!(
        "User's current query: {}\nChat history: {}",
        user_raw_request, current_cache
    );

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(ProbiotRoleType::CrapRequestProcessing)),
        AppName::W3ABot => Some(AppsSystemRoles::W3A(W3ARoleType::CrapRequestProcessing)),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::CrapRequestProcessing)),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "CrapRequestProcessing role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    let llm_response =
        raw_llm_processing(system_role, llm_message, app_state, LlmModel::Light).await?;

    Ok(llm_response)
}

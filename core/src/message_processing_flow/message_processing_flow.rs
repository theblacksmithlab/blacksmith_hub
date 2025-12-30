use crate::ai::common::common::raw_llm_processing;
use crate::ai::common::common::tokenize_and_truncate;
use crate::message_processing_flow::check_request_type::check_request_type;
use crate::message_processing_flow::clarify_request::clarify_request;
use crate::models::common::ai::LlmModel;
use crate::models::common::app_name::AppName;
use crate::models::common::qdrant_collection_manager::AppsCollections;
use crate::models::common::request_type::RequestType;
use crate::models::common::system_roles::{AppsSystemRoles, W3ARoleType};
use crate::models::common::system_roles::{BlacksmithLabRoleType, ProbiotRoleType};
use crate::rag_system::get_results_via_rag_system::get_results_via_rag_system::get_results_via_rag_system;
use crate::rag_system::types::{DocumentType, RAGConfig};
use crate::rag_system::{get_advanced_rag_config, get_hybrid_search_rag_config};
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::state::qdrant_client_init_trait::QdrantClientInit;
use crate::temp_cache::temp_cache_traits::TempCacheInit;
use crate::utils::common::get_system_role_or_fallback;
use crate::utils::tg_bot::tg_bot::{add_user_message_to_cache, get_cache_as_string};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

pub async fn process_user_raw_request<
    T: OpenAIClientInit + QdrantClientInit + TempCacheInit + Send + Sync,
>(
    user_id: &str,
    user_raw_request: &str,
    app_state: Arc<T>,
    app_name: AppName,
) -> Result<(String, HashMap<String, String>)> {
    add_user_message_to_cache(app_state.clone(), user_id, user_raw_request).await;

    let current_cache = get_cache_as_string(app_state.clone(), user_id).await;

    let request_type = check_request_type(
        user_raw_request,
        &current_cache,
        app_state.clone(),
        app_name.clone(),
    )
    .await?;

    match request_type {
        RequestType::Common => {
            info!("Common case request detected");
            let response_for_common_case_request = handle_common_case_request(
                user_raw_request,
                app_state.clone(),
                &current_cache,
                app_name.clone(),
            )
            .await?;

            Ok((response_for_common_case_request, HashMap::new()))
        }
        RequestType::Special => {
            info!("Special case request detected");
            let clarified_request = clarify_request(
                user_raw_request,
                &current_cache,
                app_state.clone(),
                app_name.clone(),
            )
            .await?;

            let (response_for_special_case_request, extra_data) = handle_special_case_request(
                user_raw_request,
                &clarified_request,
                app_state,
                &current_cache,
                app_name.clone(),
            )
            .await?;

            Ok((response_for_special_case_request, extra_data))
        }
        RequestType::Invalid => {
            info!("Invalid case request detected");
            let response_for_invalid_request = handle_invalid_request(
                user_raw_request,
                app_state.clone(),
                &current_cache,
                app_name.clone(),
            )
            .await?;

            Ok((response_for_invalid_request, HashMap::new()))
        }
    }
}

pub async fn handle_special_case_request<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    user_raw_request: &str,
    clarified_request: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: AppName,
) -> Result<(String, HashMap<String, String>)> {
    let collection_names: Vec<String> = AppsCollections::all_collections_for_app(app_name.clone())
        .iter()
        .map(|collection| collection.as_str().to_string())
        .collect();

    let rag_config = match app_name {
        AppName::W3AWeb => get_hybrid_search_rag_config(),
        AppName::BlacksmithWeb => get_advanced_rag_config(),
        _ => get_advanced_rag_config(),
    };

    let rag_system_search_result = get_results_via_rag_system(
        clarified_request,
        &collection_names,
        rag_config.clone(),
        app_state.clone(),
    )
    .await?;

    let search_result_content = rag_system_search_result.context;

    let max_tokens = 8192;

    let post_processed_initial_search_result_content =
        tokenize_and_truncate(&search_result_content, max_tokens)
            .await
            .unwrap_or_else(|_| search_result_content.clone());

    let (final_context, extra_data) = if matches!(rag_config, RAGConfig::HybridSearch { .. }) {
        // NEW: HybridSearch - extract title and lesson_link from metadata
        let extra_data_map: HashMap<String, String> = rag_system_search_result
            .documents
            .iter()
            .filter_map(|doc| match doc {
                DocumentType::HybridSearch(d) => {
                    let title = d.metadata.title.clone();
                    let link = d.metadata.extra.clone()?;
                    Some((title, link))
                }
                _ => None,
            })
            .collect();

        (
            post_processed_initial_search_result_content.clone(),
            extra_data_map,
        )
    } else if matches!(rag_config, RAGConfig::PayloadKeyBased { .. }) {
        let initial_search_result_lesson_learned = rag_system_search_result
            .documents
            .first()
            .and_then(|doc| match doc {
                DocumentType::W3A(d) => Some(d.lesson_title.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let initial_search_result_titled_content = format!(
            "Название урока: {}.\nСостав урока:\n{}",
            initial_search_result_lesson_learned, post_processed_initial_search_result_content
        );

        info!(
            "Base search result's lesson title: {}",
            initial_search_result_lesson_learned
        );

        (
            initial_search_result_titled_content,
            HashMap::new(), // PayloadKeyBased no longer returns extra_data (deprecated workflow)
        )
    } else {
        info!("No context extension needed for this RAGConfig type.");
        (
            post_processed_initial_search_result_content.clone(),
            HashMap::new(),
        )
    };

    // let llm_message = format!(
    //     "Текущий запрос пользователя: {}\nУточнение запроса: {}\nИстория чата: {}\nИнформация из базы данных для формирования ответа: {}",
    //     user_raw_request,
    //     clarified_request,
    //     current_cache,
    //     final_context,
    // );

    let llm_message = format!(
        "<knowledge_base>\n{}\n</knowledge_base>\n\n<chat_history>\n{}\n</chat_history>\n\n<clarified_request>\n{}\n</clarified_request>\n\n<user_request>\n{}\n</user_request>",
        final_context,
        current_cache,
        clarified_request,
        user_raw_request,
    );

    info!(
        "LLM message for user's request main processing:\n{}",
        llm_message
    );

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(ProbiotRoleType::MainProcessing)),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::MainProcessing)),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::MainProcessing,
        )),
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
        raw_llm_processing(&system_role, &llm_message, app_state, LlmModel::ComplexFast).await?;

    Ok((llm_response, extra_data))
}

pub async fn handle_common_case_request<T: OpenAIClientInit + Send + Sync>(
    user_raw_request: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: AppName,
) -> Result<String> {
    let llm_message = format!(
        "<user_request>\n{}\n</user_request>\n\n<chat_history>\n{}\n</chat_history>",
        user_raw_request, current_cache
    );

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(
            ProbiotRoleType::CommonCaseRequestProcessing,
        )),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(
            W3ARoleType::CommonCaseRequestProcessing,
        )),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::CommonCaseRequestProcessing,
        )),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "CommonCaseRequestProcessing role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    let llm_response =
        raw_llm_processing(&system_role, &llm_message, app_state, LlmModel::Light).await?;

    Ok(llm_response)
}

pub async fn handle_invalid_request<T: OpenAIClientInit + Send + Sync>(
    user_raw_request: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: AppName,
) -> Result<String> {
    let llm_message = format!(
        "<user_request>\n{}\n</user_request>\n\n<chat_history>\n{}\n</chat_history>",
        user_raw_request, current_cache
    );

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(
            ProbiotRoleType::InvalidCaseRequestProcessing,
        )),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(
            W3ARoleType::InvalidCaseRequestProcessing,
        )),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::InvalidCaseRequestProcessing,
        )),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "InvalidCaseRequestProcessing role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    let llm_response =
        raw_llm_processing(&system_role, &llm_message, app_state, LlmModel::Light).await?;

    Ok(llm_response)
}

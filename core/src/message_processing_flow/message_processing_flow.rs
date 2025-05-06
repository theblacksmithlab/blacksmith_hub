use crate::ai::common::common::tokenize_and_truncate;
use crate::ai::common::common::{raw_llm_processing, raw_llm_processing_json};
use crate::message_processing_flow::check_request_for_crap_content::check_request_for_crap_content;
use crate::message_processing_flow::clarify_request::clarify_request;
use crate::models::common::app_name::AppName;
use crate::models::common::qdrant_collection_manager::AppsCollections;
use crate::models::common::system_messages::{AppsSystemMessages, W3AMessages};
use crate::models::common::system_roles::{BlacksmithLabRoleType, ProbiotRoleType};
use crate::models::common::system_roles::{AppsSystemRoles, W3ARoleType};
use crate::rag_system::context_builder::DefaultContextBuilder;
use crate::rag_system::get_results_via_rag_system::get_results_via_rag_system::get_results_via_rag_system;
use crate::rag_system::retriever::QdrantRetriever;
use crate::rag_system::types::DocumentType;
use crate::rag_system::{
    get_advanced_rag_config, get_payload_key_based_rag_config, ContextBuilder,
    PayloadKeyBasedRetriever,
};
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::state::qdrant_client_init_trait::QdrantClientInit;
use crate::temp_cache::temp_cache_traits::TempCacheInit;
use crate::utils::common::{get_message, get_system_role_or_fallback};
use crate::utils::tg_bot::tg_bot::{add_user_message_to_cache, get_cache_as_string};
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tracing::{error, info, warn};
use crate::models::common::ai::LlmModel;


pub async fn process_user_raw_request<
    T: OpenAIClientInit + QdrantClientInit + TempCacheInit + Send + Sync,
>(
    user_id: &str,
    user_raw_request: &str,
    app_state: Arc<T>,
    app_name: AppName,
) -> Result<(String, Vec<String>)> {
    add_user_message_to_cache(app_state.clone(), user_id, user_raw_request).await;

    let current_cache = get_cache_as_string(app_state.clone(), user_id).await;

    let clarified_request = clarify_request(
        user_raw_request,
        &current_cache,
        app_state.clone(),
        app_name.clone(),
    )
    .await?;

    let is_crap = check_request_for_crap_content(
        user_raw_request,
        &clarified_request,
        &current_cache,
        app_state.clone(),
        app_name.clone(),
    )
    .await?;

    if is_crap {
        info!("Crap request detected, sending message to handle_crap_request fn");
        let response_for_crap_request = handle_crap_request(
            user_raw_request,
            app_state.clone(),
            &clarified_request,
            app_name.clone(),
        )
        .await?;

        Ok((response_for_crap_request, Vec::new()))
    } else {
        info!("Valid request detected, sending message to handle_valid_request fn");
        let (response_for_valid_request, extra_data) = handle_valid_request(
            user_raw_request,
            &clarified_request,
            app_state,
            &current_cache,
            app_name.clone(),
        )
        .await?;

        Ok((response_for_valid_request, extra_data))
    }
}

pub async fn handle_valid_request<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    user_raw_request: &str,
    clarified_request: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: AppName,
) -> Result<(String, Vec<String>)> {
    let collection_names: Vec<String> = AppsCollections::all_collections_for_app(app_name.clone())
        .iter()
        .map(|collection| collection.as_str().to_string())
        .collect();

    let rag_config = match app_name {
        AppName::W3AWeb | AppName::W3ABot => get_payload_key_based_rag_config(),
        AppName::BlacksmithWeb => get_advanced_rag_config(),
        _ => get_advanced_rag_config(),
    };

    info!(
        "Collections names for RAG system: {:?}",
        collection_names
    );

    let rag_system_search_result = get_results_via_rag_system(
        clarified_request,
        &collection_names,
        rag_config,
        app_state.clone(),
    )
    .await?;

    let search_result_content = rag_system_search_result.context;

    let max_tokens = 8192;
    let min_tokens = 4096;

    let (post_processed_initial_search_result_content, token_count) =
        tokenize_and_truncate(&search_result_content, max_tokens)
            .await
            .unwrap_or_else(|_| (search_result_content.clone(), max_tokens));

    let (additional_context, extra_data) = if matches!(app_name, AppName::W3AWeb | AppName::W3ABot) {
        let initial_search_result_lesson_learned = rag_system_search_result
            .documents
            .first()
            .and_then(|doc| {
                if let DocumentType::W3A(d) = doc {
                    Some(d.lesson_title.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let initial_search_result_titled_content = format!(
            "Lesson title: {}.\nLesson content:\n{}",
            initial_search_result_lesson_learned, post_processed_initial_search_result_content
        );

        info!(
            "Base search result's lesson title: {}",
            initial_search_result_lesson_learned
        );

        fetch_additional_context(
            user_raw_request,
            clarified_request,
            app_state.clone(),
            current_cache,
            &app_name,
            &collection_names,
            &initial_search_result_titled_content,
            token_count,
            max_tokens,
            min_tokens,
            vec![initial_search_result_lesson_learned],
            5,
            0,
        )
        .await?
    } else {
        (String::new(), Vec::new())
    };

    let llm_message = format!(
        "User's current query: {}\nUser's refined query: {}\nChat history: {}\nRelevant information from the database: {}",
        user_raw_request,
        clarified_request,
        current_cache,
        if matches!(app_name, AppName::W3AWeb | AppName::W3ABot) {
            additional_context
        } else {
            post_processed_initial_search_result_content
        }
    );

    info!("LLM message for user's request main processing: {}", llm_message);

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(ProbiotRoleType::MainProcessing)),
        AppName::W3ABot => Some(AppsSystemRoles::W3A(W3ARoleType::MainProcessing)),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::MainProcessing)),
        AppName::BlacksmithWeb =>Some(AppsSystemRoles::BlacksmithLab(BlacksmithLabRoleType::MainProcessing)),
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
        raw_llm_processing(&system_role, &llm_message, app_state, LlmModel::Complex).await?;

    Ok((llm_response, extra_data))
}

pub async fn handle_crap_request<T: OpenAIClientInit + Send + Sync>(
    user_raw_request: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: AppName,
) -> Result<String> {
    let llm_message = format!(
        "User's current query: {}\nChat history: {}",
        user_raw_request, current_cache
    );

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(
            ProbiotRoleType::CrapRequestProcessing,
        )),
        AppName::W3ABot => Some(AppsSystemRoles::W3A(W3ARoleType::CrapRequestProcessing)),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::CrapRequestProcessing)),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(BlacksmithLabRoleType::CrapRequestProcessing)),
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
        raw_llm_processing(&system_role, &llm_message, app_state, LlmModel::Light).await?;

    Ok(llm_response)
}

pub async fn get_llm_recommendation<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    user_raw_request: &str,
    clarified_request: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: &AppName,
    lesson_learned: &Vec<String>,
) -> Result<String> {
    let w3a_academy_learning_structure =
        get_message(AppsSystemMessages::W3A(W3AMessages::W3AStudyStructure)).await?;

    let system_role = get_system_role_or_fallback(&app_name, W3ARoleType::Recommendation, None);

    let llm_message = format!("User's current query: {}\nUser's refined query: {}\nChat history: {}\nWeb3 Academy learning structure:{}\nCompleted lessons:{:?}\n", user_raw_request, clarified_request, current_cache, w3a_academy_learning_structure, lesson_learned);

    let result =
        raw_llm_processing_json(&system_role, &llm_message, app_state, LlmModel::Light).await?;

    info!("LLM lesson recommendation: {}", result);

    let parsed_json: Value = serde_json::from_str(&result).map_err(|err| {
        error!("Failed to parse LLM response as JSON: {}", err);
        err
    })?;

    let llm_lesson_recommendation = parsed_json
        .get("Recommended lesson")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    info!("Extracted recommendation: {}", llm_lesson_recommendation);

    if !llm_lesson_recommendation.is_empty()
        && !lesson_exists_in_structure(&llm_lesson_recommendation, &w3a_academy_learning_structure)
    {
        warn!(
            "Recommended lesson '{}' not found in W3A structure, returning empty string",
            llm_lesson_recommendation
        );
        return Ok(String::new());
    }

    Ok(llm_lesson_recommendation)
}

fn lesson_exists_in_structure(lesson: &str, structure_json: &str) -> bool {
    if let Ok(structure) = serde_json::from_str::<Value>(structure_json) {
        if let Some(obj) = structure.as_object() {
            for (_, module_value) in obj {
                if let Some(module_obj) = module_value.as_object() {
                    for (_, block_value) in module_obj {
                        if let Some(lessons) = block_value.as_array() {
                            for lesson_value in lessons {
                                if let Some(lesson_str) = lesson_value.as_str() {
                                    if lesson_str.to_lowercase() == lesson {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

pub async fn get_additional_context_by_llm_recommendation<
    T: OpenAIClientInit + QdrantClientInit + Send + Sync,
>(
    collection_names: Vec<String>,
    llm_recommendation: &str,
    app_state: Arc<T>,
) -> Result<String> {
    if llm_recommendation.is_empty() {
        info!("Empty LLM recommendation received, returning empty context");
        return Ok(String::new());
    }

    let payload_key = "lesson_title".to_string();

    let payload_key_based_retriever = QdrantRetriever::new(app_state.clone(), collection_names);

    let context_builder = DefaultContextBuilder::new().with_separator("\n-\n".to_string());

    let additional_documents_by_llm_recommendation = payload_key_based_retriever
        .search_by_payload_key(&payload_key, &llm_recommendation)
        .await?;

    let final_results = additional_documents_by_llm_recommendation
        .into_iter()
        .map(DocumentType::W3A)
        .collect();

    let additional_context = context_builder.build_context(final_results)?;

    Ok(additional_context)
}

async fn fetch_additional_context<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    user_raw_request: &str,
    clarified_request: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: &AppName,
    collection_names: &Vec<String>,
    actual_context: &str,
    current_token_count: usize,
    max_tokens: usize,
    min_tokens: usize,
    lessons_learned: Vec<String>,
    max_attempts: usize,
    attempt_counter: usize,
) -> Result<(String, Vec<String>)> {
    if current_token_count >= min_tokens {
        info!(
            "Actual context token count: {} which is >= required minimum: {}, context is enough.",
            current_token_count, min_tokens
        );
        return Ok((actual_context.to_string(), lessons_learned));
    }

    info!(
        "Recursive search attempts counter: {}",
        attempt_counter
    );

    if attempt_counter >= max_attempts {
        warn!(
            "Reached max attempts ({}) of recursive search, stopping additional context search.",
            max_attempts
        );
        return Ok((actual_context.to_string(), lessons_learned));
    }

    info!(
        "Actual context token count: {} is below the threshold ({}), additional search is triggered. (attempt {})",
        current_token_count, min_tokens, max_attempts
    );

    info!("Actual learned lessons: {:?} at the step #{} of attempts counter", lessons_learned, attempt_counter);

    let llm_recommendation = get_llm_recommendation(
        user_raw_request,
        clarified_request,
        app_state.clone(),
        current_cache,
        app_name,
        &lessons_learned,
    )
    .await?;

    info!("LLM recommended lesson: {}", llm_recommendation);

    let mut updated_lessons_learned = lessons_learned.clone();
    updated_lessons_learned.push(llm_recommendation.clone());

    info!(
        "Actual learned lessons after update: {:?} at the step #{} of attempts counter",
        updated_lessons_learned,
        attempt_counter
    );

    let raw_additional_search_result_content = get_additional_context_by_llm_recommendation(
        collection_names.clone(),
        &llm_recommendation,
        app_state.clone(),
    )
    .await?;

    let max_additional_tokens = max_tokens - current_token_count;

    let (post_processed_additional_search_result_content, new_token_count) =
        tokenize_and_truncate(&raw_additional_search_result_content, max_additional_tokens)
            .await
            .unwrap_or_else(|_| {
                (
                    raw_additional_search_result_content.clone(),
                    max_additional_tokens,
                )
            });

    let additional_search_result_titled_content = format!(
        "Lesson title: {}.\nLesson content:\n{}",
        llm_recommendation, post_processed_additional_search_result_content
    );
    let updated_context = format!(
        "{}\n\n{}",
        actual_context, additional_search_result_titled_content
    );
    let updated_token_count = current_token_count + new_token_count;
    let updated_attempt_counter = attempt_counter + 1;

    let (final_context, final_lessons) = Box::pin(fetch_additional_context(
        user_raw_request,
        clarified_request,
        app_state,
        current_cache,
        app_name,
        collection_names,
        &updated_context,
        updated_token_count,
        max_tokens,
        min_tokens,
        updated_lessons_learned,
        max_attempts,
        updated_attempt_counter,
    ))
    .await?;
    
    Ok((final_context, final_lessons))
}

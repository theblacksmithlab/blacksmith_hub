use crate::ai::common::google::raw_google_processing;
use crate::ai::common::openai::raw_openai_processing;
use crate::ai::common::openai::tokenize_and_truncate;
use crate::message_processing_flow::analyze_query_complexity::analyze_query_complexity;
use crate::message_processing_flow::check_request_type::get_query_type;
use crate::message_processing_flow::clarify_request::clarify_query;
use crate::message_processing_flow::generate_aspects::generate_aspects;
use crate::models::common::ai::{GoogleModel, OpenAIModel};
use crate::models::common::app_name::AppName;
use crate::models::common::qdrant_collection_manager::AppsCollections;
use crate::models::common::query_type::QueryType;
use crate::models::common::system_roles::{AppsSystemRoles, W3ARoleType};
use crate::models::common::system_roles::{BlacksmithLabRoleType, ProbiotRoleType};
use crate::rag_system::get_results_via_rag_system::get_results_via_rag_system::get_results_via_rag_system;
use crate::rag_system::query_decompression_types::QueryComplexity;
use crate::rag_system::types::DocumentType;
use crate::rag_system::{
    get_advanced_rag_config, get_default_rag_config_with_params, get_hybrid_search_rag_config,
    get_hybrid_search_rag_config_with_params,
};
use crate::state::llm_client_init_trait::{GoogleClientInit, OpenAIClientInit};
use crate::state::qdrant_client_init_trait::QdrantClientInit;
use crate::temp_cache::temp_cache_traits::TempCacheInit;
use crate::utils::common::get_system_role;
use crate::utils::tg_bot::tg_bot::{add_user_message_to_cache, get_cache_as_string};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

pub async fn process_user_query<
    T: OpenAIClientInit + GoogleClientInit + QdrantClientInit + TempCacheInit + Send + Sync,
>(
    user_id: &str,
    user_raw_query: &str,
    app_state: Arc<T>,
    app_name: AppName,
) -> Result<(String, HashMap<String, String>)> {
    add_user_message_to_cache(app_state.clone(), user_id, user_raw_query).await;

    let current_cache = get_cache_as_string(app_state.clone(), user_id).await;

    let query_type = get_query_type(
        user_raw_query,
        &current_cache,
        app_state.clone(),
        app_name.clone(),
    )
    .await?;

    match query_type {
        QueryType::Common => {
            info!("Processing common case user query...");
            let response_for_common_case_query = handle_common_case_query(
                user_raw_query,
                app_state.clone(),
                &current_cache,
                app_name.clone(),
            )
            .await?;

            Ok((response_for_common_case_query, HashMap::new()))
        }
        QueryType::Special => {
            info!("Processing special case user query...");
            let clarified_query = clarify_query(
                user_raw_query,
                &current_cache,
                app_state.clone(),
                app_name.clone(),
            )
            .await?;

            let (response_for_special_case_query, extra_data) = handle_special_case_query(
                user_raw_query,
                &clarified_query,
                app_state,
                &current_cache,
                app_name.clone(),
            )
            .await?;

            Ok((response_for_special_case_query, extra_data))
        }
        QueryType::Invalid => {
            info!("Processing invalid case user query...");
            let response_for_invalid_query = handle_invalid_query(
                user_raw_query,
                app_state.clone(),
                &current_cache,
                app_name.clone(),
            )
            .await?;

            Ok((response_for_invalid_query, HashMap::new()))
        }
        QueryType::Support => {
            info!("Processing support case user query...");
            let response_for_support_query = handle_support_query(
                user_raw_query,
                app_state.clone(),
                &current_cache,
                app_name.clone(),
            )
            .await?;

            Ok((response_for_support_query, HashMap::new()))
        }
    }
}

pub async fn handle_special_case_query<
    T: OpenAIClientInit + QdrantClientInit + GoogleClientInit + Send + Sync,
>(
    user_raw_query: &str,
    clarified_query: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: AppName,
) -> Result<(String, HashMap<String, String>)> {
    let collection_names: Vec<String> = AppsCollections::all_collections_for_app(app_name.clone())
        .iter()
        .map(|collection| collection.as_str().to_string())
        .collect();

    let max_tokens = 10240;

    let query_complexity = analyze_query_complexity(
        clarified_query,
        current_cache,
        app_state.clone(),
        app_name.clone(),
    )
    .await?;

    info!("Query complexity: {:?}", query_complexity);

    let (final_context, extra_data) = match query_complexity {
        QueryComplexity::Base => {
            process_base_hybrid_search(
                clarified_query,
                &collection_names,
                app_state.clone(),
                max_tokens,
                app_name.clone(),
            )
            .await?
        }
        QueryComplexity::Complex => {
            info!("Complex query detected. Generating aspects...");

            let (final_context, extra_data_map) = match generate_aspects(
                clarified_query,
                current_cache,
                app_state.clone(),
                app_name.clone(),
            )
            .await
            {
                Ok(aspects) => {
                    info!("Aspects generated for user query: {:?}", aspects);

                    let documents = search_by_aspects(
                        aspects.clone(),
                        &collection_names,
                        app_state.clone(),
                        app_name.clone(),
                    )
                    .await?;

                    let structured_context =
                        build_structured_context_with_aspects(&aspects, &documents);

                    let final_context = tokenize_and_truncate(&structured_context, max_tokens)
                        .await
                        .unwrap_or_else(|_| structured_context.clone());

                    let extra_data_map: HashMap<String, String> = documents
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

                    (final_context, extra_data_map)
                }
                Err(err) => {
                    error!(
                        "Failed to generate aspects: {}. Falling back to Base mode.",
                        err
                    );

                    process_base_hybrid_search(
                        clarified_query,
                        &collection_names,
                        app_state.clone(),
                        max_tokens,
                        app_name.clone(),
                    )
                    .await?
                }
            };

            (final_context, extra_data_map)
        }
    };

    let llm_message = format!(
        "<knowledge_base>\n{}\n</knowledge_base>\n\n<chat_history>\n{}\n</chat_history>\n\n<clarified_query>\n{}\n</clarified_query>\n\n<user_query>\n{}\n</user_query>",
        final_context,
        current_cache,
        clarified_query,
        user_raw_query,
    );

    info!(
        "LLM message for user's query main processing:\n{}",
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
        Some(role) => get_system_role(&app_name, role.as_str())?,
        None => {
            return Err(anyhow::anyhow!(
                "MainProcessing system role is not defined for app '{}'",
                app_name.as_str()
            ));
        }
    };

    let llm_response = match raw_google_processing(
        &system_role,
        &llm_message,
        app_state.clone(),
        GoogleModel::Pro,
    )
    .await
    {
        Ok(result) => {
            info!("Google main processing succeeded");
            result
        }
        Err(e) => {
            warn!(
                "Google main processing failed: {}. Falling back to OpenAI.",
                e
            );
            raw_openai_processing(&system_role, &llm_message, app_state, OpenAIModel::GPT5).await?
        }
    };

    Ok((llm_response, extra_data))
}

pub async fn handle_common_case_query<T: OpenAIClientInit + GoogleClientInit + Send + Sync>(
    user_raw_query: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: AppName,
) -> Result<String> {
    let chat_history_section = if current_cache.trim().is_empty() {
        "<chat_history>Нет предыдущих сообщений</chat_history>".to_string()
    } else {
        format!("<chat_history>\n{}\n</chat_history>", current_cache)
    };

    let llm_message = format!(
        "{}\n\n<current_query>\n{}\n</current_query>",
        chat_history_section, user_raw_query
    );

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(
            ProbiotRoleType::CommonCaseQueryProcessing,
        )),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::CommonCaseQueryProcessing)),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::CommonCaseQueryProcessing,
        )),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role(&app_name, role.as_str())?,
        None => {
            return Err(anyhow::anyhow!(
                "CommonCaseQueryProcessing system role is not defined for app '{}'",
                app_name.as_str()
            ));
        }
    };

    let llm_response = match raw_google_processing(
        &system_role,
        &llm_message,
        app_state.clone(),
        GoogleModel::Flash,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            warn!(
                "Google common case processing failed: {}. Falling back to OpenAI.",
                e
            );
            raw_openai_processing(&system_role, &llm_message, app_state, OpenAIModel::GPT5lr)
                .await?
        }
    };

    Ok(llm_response)
}

pub async fn handle_invalid_query<T: OpenAIClientInit + GoogleClientInit + Send + Sync>(
    user_raw_query: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: AppName,
) -> Result<String> {
    let chat_history_section = if current_cache.trim().is_empty() {
        "<chat_history>Нет предыдущих сообщений</chat_history>".to_string()
    } else {
        format!("<chat_history>\n{}\n</chat_history>", current_cache)
    };

    let llm_message = format!(
        "{}\n\n<current_query>\n{}\n</current_query>",
        chat_history_section, user_raw_query
    );

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(
            ProbiotRoleType::InvalidCaseQueryProcessing,
        )),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(
            W3ARoleType::InvalidCaseQueryProcessing,
        )),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::InvalidCaseQueryProcessing,
        )),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role(&app_name, role.as_str())?,
        None => {
            return Err(anyhow::anyhow!(
                "InvalidCaseQueryProcessing system role is not defined for app '{}'",
                app_name.as_str()
            ));
        }
    };

    let llm_response = match raw_google_processing(
        &system_role,
        &llm_message,
        app_state.clone(),
        GoogleModel::Flash,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            warn!(
                "Google invalid case processing failed: {}. Falling back to OpenAI.",
                e
            );
            raw_openai_processing(&system_role, &llm_message, app_state, OpenAIModel::GPT5lr)
                .await?
        }
    };

    Ok(llm_response)
}

pub async fn handle_support_query<T: OpenAIClientInit + GoogleClientInit + Send + Sync>(
    user_raw_query: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: AppName,
) -> Result<String> {
    let chat_history_section = if current_cache.trim().is_empty() {
        "<chat_history>Нет предыдущих сообщений</chat_history>".to_string()
    } else {
        format!("<chat_history>\n{}\n</chat_history>", current_cache)
    };

    let llm_message = format!(
        "{}\n\n<current_query>\n{}\n</current_query>",
        chat_history_section, user_raw_query
    );

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(
            ProbiotRoleType::InvalidCaseQueryProcessing,
        )),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(
            W3ARoleType::SupportCaseQueryProcessing,
        )),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::InvalidCaseQueryProcessing,
        )),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role(&app_name, role.as_str())?,
        None => {
            return Err(anyhow::anyhow!(
                "SupportCaseQueryProcessing system role is not defined for app '{}'",
                app_name.as_str()
            ));
        }
    };

    let llm_response = match raw_google_processing(
        &system_role,
        &llm_message,
        app_state.clone(),
        GoogleModel::Flash,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            warn!(
                "Google support case processing failed: {}. Falling back to OpenAI.",
                e
            );
            raw_openai_processing(&system_role, &llm_message, app_state, OpenAIModel::GPT5lr)
                .await?
        }
    };

    Ok(llm_response)
}

async fn process_base_hybrid_search<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    clarified_query: &str,
    collection_names: &Vec<String>,
    app_state: Arc<T>,
    max_tokens: usize,
    app_name: AppName,
) -> Result<(String, HashMap<String, String>)> {
    let rag_config = match app_name {
        AppName::W3AWeb => get_hybrid_search_rag_config(),
        AppName::BlacksmithWeb => get_advanced_rag_config(),
        _ => get_advanced_rag_config(),
    };

    let rag_system_search_result = get_results_via_rag_system(
        clarified_query,
        collection_names,
        rag_config.clone(),
        app_state.clone(),
    )
    .await?;

    let search_result_content = rag_system_search_result.context;

    let post_processed_content = tokenize_and_truncate(&search_result_content, max_tokens)
        .await
        .unwrap_or_else(|_| search_result_content.clone());

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

    Ok((post_processed_content, extra_data_map))
}

async fn search_by_aspects<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    aspects: Vec<String>,
    collection_names: &Vec<String>,
    app_state: Arc<T>,
    app_name: AppName,
) -> Result<Vec<DocumentType>> {
    use std::collections::HashSet;

    let mut all_documents = Vec::new();
    let mut seen_document_ids: HashSet<String> = HashSet::new();

    for aspect in aspects {
        let rag_config = match app_name {
            AppName::W3AWeb => get_hybrid_search_rag_config_with_params(1),
            AppName::BlacksmithWeb => get_default_rag_config_with_params(1, 0.25),
            _ => get_default_rag_config_with_params(1, 0.25),
        };

        let rag_result =
            get_results_via_rag_system(&aspect, collection_names, rag_config, app_state.clone())
                .await?;

        // Collect documents with deduplication
        for doc in rag_result.documents {
            let doc_id = match &doc {
                DocumentType::HybridSearch(d) => d.document_id.clone(),
                DocumentType::W3A(d) => match &d.point_id {
                    crate::rag_system::types::PointId::Uuid(id) => id.clone(),
                    crate::rag_system::types::PointId::Num(id) => id.to_string(),
                },
                DocumentType::Default(d) => match &d.point_id {
                    crate::rag_system::types::PointId::Uuid(id) => id.clone(),
                    crate::rag_system::types::PointId::Num(id) => id.to_string(),
                },
            };

            if seen_document_ids.insert(doc_id) {
                all_documents.push(doc);
            }
        }
    }

    info!(
        "Search by aspects completed. Found {} unique documents after deduplication",
        all_documents.len()
    );

    Ok(all_documents)
}

fn build_structured_context_with_aspects(aspects: &[String], documents: &[DocumentType]) -> String {
    let mut context = String::new();

    context.push_str("Запрос проанализирован с точки зрения следующих аспектов:\n");
    for (i, aspect) in aspects.iter().enumerate() {
        context.push_str(&format!("- {}\n", aspect));
        if i == aspects.len() - 1 {
            context.push('\n');
        }
    }

    if !documents.is_empty() {
        context.push_str("На основе анализа вышеперечисленных аспектов найдена следующая информация в базе знаний:\n");
        for (i, doc) in documents.iter().enumerate() {
            let doc_text = match doc {
                DocumentType::HybridSearch(d) => {
                    let mut header = format!("=== {} ===", d.metadata.title);

                    if let Some(hierarchy) = &d.metadata.hierarchy {
                        header.push_str(&format!("\n{}", hierarchy));
                    }
                    format!("{}\n\n{}", header, &d.text)
                }
                DocumentType::W3A(d) => d.text.clone(),
                DocumentType::Default(d) => d.text.clone(),
            };

            context.push_str(&doc_text);

            if i < documents.len() - 1 {
                context.push_str("\n\n");
            }
        }
    }

    context
}

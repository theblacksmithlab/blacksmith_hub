use crate::ai::common::common::raw_llm_processing;
use crate::ai::common::common::tokenize_and_truncate;
use crate::message_processing_flow::analyze_query_complexity::analyze_query_complexity;
use crate::message_processing_flow::check_request_type::check_request_type;
use crate::message_processing_flow::clarify_request::clarify_request;
use crate::message_processing_flow::generate_aspects::generate_aspects;
use crate::models::common::ai::LlmModel;
use crate::models::common::app_name::AppName;
use crate::models::common::qdrant_collection_manager::AppsCollections;
use crate::models::common::request_type::RequestType;
use crate::models::common::system_roles::{AppsSystemRoles, W3ARoleType};
use crate::models::common::system_roles::{BlacksmithLabRoleType, ProbiotRoleType};
use crate::rag_system::get_results_via_rag_system::get_results_via_rag_system::get_results_via_rag_system;
use crate::rag_system::query_decompression_types::QueryComplexity;
use crate::rag_system::types::DocumentType;
use crate::rag_system::{
    get_advanced_rag_config, get_default_rag_config_with_params, get_hybrid_search_rag_config,
    get_hybrid_search_rag_config_with_params,
};
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::state::qdrant_client_init_trait::QdrantClientInit;
use crate::temp_cache::temp_cache_traits::TempCacheInit;
use crate::utils::common::get_system_role_or_fallback;
use crate::utils::tg_bot::tg_bot::{add_user_message_to_cache, get_cache_as_string};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

pub async fn process_user_query<
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

    let max_tokens = 10240;

    let query_complexity = analyze_query_complexity(
        clarified_request,
        current_cache,
        app_state.clone(),
        app_name.clone(),
    )
    .await?;

    info!("Query complexity: {:?}", query_complexity);

    let (final_context, extra_data) = match query_complexity {
        QueryComplexity::Base => {
            process_base_hybrid_search(
                clarified_request,
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
                clarified_request,
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
                        clarified_request,
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

async fn process_base_hybrid_search<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    clarified_request: &str,
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
        clarified_request,
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

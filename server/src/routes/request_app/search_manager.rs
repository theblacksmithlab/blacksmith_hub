use core::models::request_app::request_app::RequestAppSystemRoleType;
use anyhow::Result;
use core::ai::ai::{raw_llm_processing_json, vectorize};
use core::state::request_app::app_state::RequestAppState;
use core::utils::common::get_system_role_or_fallback;
use core::vector_db::vector_db::{
    get_llm_order_from_response, prepare_search_results_for_llm, qdrant_search,
    store_search_results, update_search_results_order, ExecutionMode, QdrantSearchResult,
};
use std::sync::Arc;
use teloxide::prelude::ChatId;
use tracing::info;
use core::utils::common::LlmModel;

pub(crate) async fn activate_search_manager(
    user_request: String,
    user_id: ChatId,
    app_state: Arc<RequestAppState>,
) -> Result<String> {
    info!("Fn: activate_search_manager | status: activated");

    let request_vector = vectorize(user_request.clone(), app_state.clone()).await?;

    let collection_name = "outside_app".to_string();

    let search_results = qdrant_search(
        app_state.clone(),
        request_vector,
        &collection_name,
        0.3,
        6,
        user_id,
        ExecutionMode::Partial,
    )
    .await?;

    if let QdrantSearchResult::Partial(extracted_filtered_results) = search_results {
        if extracted_filtered_results.is_empty() {
            info!("Fn: activate_search_manager | status: No results found in qdrant_db");
            // String to be returned just for logging the result of the function
            return Ok("No results found".to_string());
        }
        info!("Fn: activate_search_manager | status: Store results in AppState");
        store_search_results(app_state.clone(), user_id, extracted_filtered_results).await;
    } else {
        // TODO: get rid of panic!
        panic!(
            "Unexpected result: ExecutionMode::Partial did not return QdrantSearchResult::Partial"
        );
    }

    let search_results_string = prepare_search_results_for_llm(app_state.clone(), user_id).await;
    let message_for_llm = format!("User's query: {}\nA list of search results from the Qdrant vector database based on the user's query: {}", user_request, search_results_string);
    info!("Fn: activate_search_manager | message: {}", message_for_llm);

    let system_role = get_system_role_or_fallback(
        "request_app",
        RequestAppSystemRoleType::ReorderingResults,
        None
    );

    let llm_order_processing =
        raw_llm_processing_json(system_role, message_for_llm, app_state.clone(), LlmModel::Complex).await;

    info!(
        "Fn: activate_search_manager | Llm texts ordering process result: {:?}",
        llm_order_processing
    );

    match llm_order_processing {
        Ok(response) => {
            match get_llm_order_from_response(&response).await {
                Ok(new_order) => {
                    update_search_results_order(app_state, user_id, new_order).await;
                    // String to be returned just for logging the result of the function
                    Ok("All's good".to_string())
                }
                Err(err) => Err(anyhow::anyhow!("Failed to parse LLM response: {}", err)),
            }
        }
        Err(err) => Err(anyhow::anyhow!("Failed to get response from LLM: {}", err)),
    }
}

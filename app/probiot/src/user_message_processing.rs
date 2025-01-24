use std::sync::Arc;
use teloxide::types::ChatId;
use anyhow::Result;
use qdrant_client::qdrant::qdrant_client::QdrantClient;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::{add_user_message_to_cache, get_cache_as_string};
use core::state::tg_bot::app_state::BotAppState;
use crate::probiot_utils::{check_request_for_crap_content, clarify_request};
use core::rag_system::vectorizer::OpenAIVectorizer;
use core::rag_system::retriever::QdrantRetriever;
use core::rag_system::context_builder::DefaultContextBuilder;
use core::rag_system::types::RAGConfig;
use core::rag_system::RAGSystem;

pub async fn process_user_raw_request(
    chat_id: ChatId,
    user_raw_request: String,
    app_state: Arc<BotAppState>,
    initiator_app_name: String
) -> Result<String> {
    add_user_message_to_cache(app_state.clone(), chat_id, user_raw_request.clone()).await;
    
    let clarified_request = clarify_request(user_raw_request.clone(), app_state.clone()).await?;

    let current_cache = get_cache_as_string(app_state.clone(), chat_id).await;
    
    let is_crap = check_request_for_crap_content(user_raw_request.clone(), clarified_request.clone(), current_cache, app_state.clone()).await?;

    if is_crap {
        let response_for_crap_request = get_message(Some(&initiator_app_name), "response_for_crap_request", false).await?;
        Ok(response_for_crap_request)
    } else {
        let response_for_valid_request = handle_valid_request(user_raw_request, clarified_request, app_state).await?;
        Ok(response_for_valid_request)
    }
}

pub async fn handle_valid_request(user_raw_request: String, clarified_request: String, app_state: Arc<BotAppState>) ->Result<String> {
    let vectorizer = OpenAIVectorizer::new(app_state.clone());

    let retriever = QdrantRetriever::new(app_state.qdrant_client.clone(), "probio_collection".to_string());

    let context_builder = DefaultContextBuilder::new()
        .with_separator("\n\n".to_string());

    let config = RAGConfig {
        max_documents: 5,
        similarity_threshold: 0.7,
    };

    let rag_system = RAGSystem::new(
        vectorizer,
        retriever,
        context_builder,
        config,
    );

    let result = rag_system.process(&user_raw_request).await?;
    
    let result_string = result.context;
    
    Ok(result_string)
}
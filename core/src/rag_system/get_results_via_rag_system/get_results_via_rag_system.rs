use crate::rag_system::context_builder::DefaultContextBuilder;
use crate::rag_system::retriever::QdrantRetriever;
use crate::rag_system::types::{RAGConfig, RetrievedContext};
use crate::rag_system::vectorizer::OpenAIVectorizer;
use crate::rag_system::RAGSystem;
use crate::state::tg_bot::app_state::BotAppState;
use std::sync::Arc;

pub async fn get_results_via_rag_system(
    input_data: String,
    collection_names: Vec<String>,
    max_documents: usize,
    similarity_threshold: f32,
    app_state: Arc<BotAppState>,
) -> anyhow::Result<RetrievedContext> {
    let vectorizer = OpenAIVectorizer::new(app_state.clone());

    let retriever = QdrantRetriever::new(app_state.qdrant_client.clone(), collection_names);

    let context_builder = DefaultContextBuilder::new().with_separator("\n\n".to_string());

    let config = RAGConfig {
        max_documents,
        similarity_threshold,
    };

    let rag_system = RAGSystem::new(vectorizer, retriever, context_builder, config);

    let result = rag_system.process(&input_data).await?;

    Ok(result)
}

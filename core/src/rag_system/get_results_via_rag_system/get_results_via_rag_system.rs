use crate::rag_system::context_builder::DefaultContextBuilder;
use crate::rag_system::retriever::QdrantRetriever;
use crate::rag_system::types::{RAGConfig, RetrievedContext};
use crate::rag_system::vectorizer::OpenAIVectorizer;
use crate::rag_system::RAGSystem;
use std::sync::Arc;
use tracing::info;
use crate::state::llm_client_init_trait::LlmProcessing;
use crate::state::qdrant_client_init_trait::QdrantClientInit;

pub async fn get_results_via_rag_system<T: LlmProcessing + QdrantClientInit + Send + Sync>(
    input_data: String,
    collection_names: Vec<String>,
    config: RAGConfig,
    app_state: Arc<T>,
) -> anyhow::Result<RetrievedContext> {
    let vectorizer = OpenAIVectorizer::new(app_state.clone());

    let retriever = QdrantRetriever::new(app_state.clone(), collection_names);

    let context_builder = DefaultContextBuilder::new().with_separator("\n-\n".to_string());

    let rag_system = RAGSystem::new(vectorizer, retriever, context_builder, config);

    let result = rag_system.process(&input_data).await?;

    let total_resulting_docs_quantity = result.documents.len();
    info!(
        "Total resulting docs retrieved by the RAG system: {}",
        total_resulting_docs_quantity
    );

    Ok(result)
}

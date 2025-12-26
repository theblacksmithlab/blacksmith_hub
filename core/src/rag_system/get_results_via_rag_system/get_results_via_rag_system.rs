use crate::rag_system::context_builder::DefaultContextBuilder;
use crate::rag_system::retriever::qdrant::QdrantHybridSearchRetriever;
use crate::rag_system::retriever::QdrantRetriever;
use crate::rag_system::types::{RAGConfig, RetrievedContext};
use crate::rag_system::vectorizer::OpenAIVectorizer;
use crate::rag_system::RAGSystem;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::state::qdrant_client_init_trait::QdrantClientInit;
use std::sync::Arc;

pub async fn get_results_via_rag_system<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    input_data: &str,
    collection_names: &Vec<String>,
    config: RAGConfig,
    app_state: Arc<T>,
) -> anyhow::Result<RetrievedContext> {
    let vectorizer = OpenAIVectorizer::new(app_state.clone());
    let retriever = QdrantRetriever::new(app_state.clone(), collection_names.clone());
    let context_builder = DefaultContextBuilder::new().with_separator("\n-\n".to_string());
    let hybrid_search_retriever =
        QdrantHybridSearchRetriever::new(app_state.clone(), collection_names.clone());

    let rag_system = RAGSystem::new(
        vectorizer,
        retriever.clone(),
        retriever,
        hybrid_search_retriever,
        context_builder,
        config,
    );

    let result = rag_system.process(input_data).await?;

    Ok(result)
}

use crate::rag_system::Vectorizer;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::LlmModel;
use anyhow::Result;
use async_openai::types::{CreateEmbeddingRequestArgs, CreateEmbeddingResponse};
use async_trait::async_trait;
use std::sync::Arc;

pub struct OpenAIVectorizer<T: OpenAIClientInit> {
    app_state: Arc<T>,
}

impl<T: OpenAIClientInit> OpenAIVectorizer<T> {
    pub fn new(app_state: Arc<T>) -> Self {
        Self { app_state }
    }
}

#[async_trait]
impl<T: OpenAIClientInit + Send + Sync> Vectorizer for OpenAIVectorizer<T> {
    async fn vectorize(&self, text: &str) -> Result<Vec<f32>> {
        let llm_client = self.app_state.get_llm_client();
        let llm_model = LlmModel::TextEmbedding3Large;

        let request = CreateEmbeddingRequestArgs::default()
            .model(llm_model.as_str())
            .input(text)
            .build()?;

        let response: CreateEmbeddingResponse = llm_client.embeddings().create(request).await?;
        let embedding = response.data.into_iter().next().unwrap().embedding;

        Ok(embedding)
    }
}

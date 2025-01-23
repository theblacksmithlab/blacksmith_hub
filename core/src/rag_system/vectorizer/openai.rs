use crate::rag_system::Vectorizer;
use crate::state::llm_client_init_trait::LlmProcessing;
use anyhow::Result;
use async_openai::types::{CreateEmbeddingRequestArgs, CreateEmbeddingResponse};
use async_trait::async_trait;
use std::sync::Arc;

pub struct OpenAIVectorizer<T: LlmProcessing> {
    app_state: Arc<T>,
}

impl<T: LlmProcessing> OpenAIVectorizer<T> {
    pub fn new(app_state: Arc<T>) -> Self {
        Self { app_state }
    }
}

#[async_trait]
impl<T: LlmProcessing + Send + Sync> Vectorizer for OpenAIVectorizer<T> {
    async fn vectorize(&self, text: &str) -> Result<Vec<f32>> {
        let llm_client = self.app_state.get_llm_client();

        let request = CreateEmbeddingRequestArgs::default()
            .model("text-embedding-3-large")
            .input(text)
            .build()?;

        let response: CreateEmbeddingResponse = llm_client.embeddings().create(request).await?;
        let embedding = response.data.into_iter().next().unwrap().embedding;

        Ok(embedding)
    }
}

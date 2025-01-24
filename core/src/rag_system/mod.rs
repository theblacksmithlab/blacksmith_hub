use crate::rag_system::types::{Document, RAGConfig, RetrievedContext};
use anyhow::Result;
use async_trait::async_trait;

pub mod context_builder;
pub mod get_results_via_rag_system;
pub mod retriever;
pub mod types;
pub mod vectorizer;

#[async_trait]
pub trait Vectorizer {
    async fn vectorize(&self, text: &str) -> Result<Vec<f32>>;
}

#[async_trait]
pub trait Retriever {
    async fn search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        similarity_threshold: f32,
    ) -> Result<Vec<Document>>;
}

#[async_trait]
pub trait ContextBuilder {
    fn build_context(&self, documents: Vec<Document>) -> Result<String>;
}

pub struct RAGSystem<V, R, C>
where
    V: Vectorizer,
    R: Retriever,
    C: ContextBuilder,
{
    vectorizer: V,
    retriever: R,
    context_builder: C,
    config: RAGConfig,
}

impl<V, R, C> RAGSystem<V, R, C>
where
    V: Vectorizer,
    R: Retriever,
    C: ContextBuilder,
{
    pub fn new(vectorizer: V, retriever: R, context_builder: C, config: RAGConfig) -> Self {
        Self {
            vectorizer,
            retriever,
            context_builder,
            config,
        }
    }

    pub async fn process(&self, query: &str) -> Result<RetrievedContext> {
        let vector = self.vectorizer.vectorize(query).await?;
        let documents = self
            .retriever
            .search(
                vector,
                self.config.max_documents,
                self.config.similarity_threshold,
            )
            .await?;
        let context = self.context_builder.build_context(documents.clone())?;

        Ok(RetrievedContext { context, documents })
    }
}

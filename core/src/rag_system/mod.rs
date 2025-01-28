use std::collections::HashSet;
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
        match &self.config {
            RAGConfig::Default {
                max_documents,
                similarity_threshold,
            } => {
                let vector = self.vectorizer.vectorize(query).await?;
                let base_results = self
                    .retriever
                    .search(vector.clone(), *max_documents, *similarity_threshold)
                    .await?;
                let context = self.context_builder.build_context(base_results.clone())?;
                return Ok(RetrievedContext { context, documents: base_results });
            }
            RAGConfig::Advanced {
                base_max_documents,
                base_similarity_threshold,
                related_max_documents,
                related_similarity_threshold,
            } => {
                let vector = self.vectorizer.vectorize(query).await?;
                let base_results = self
                    .retriever
                    .search(vector.clone(), *base_max_documents, *base_similarity_threshold)
                    .await?;

                let mut all_results = base_results.clone();
                let mut seen_ids = base_results.iter().map(|doc| doc.point_id.clone()).collect::<HashSet<_>>();

                for base_result in &base_results {
                    // TODO: Consider using vector from qdrant result point
                    let base_vector = self.vectorizer.vectorize(&base_result.content).await?;
                    let related_results = self
                        .retriever
                        .search(
                            base_vector,
                            *related_max_documents,
                            *related_similarity_threshold,
                        )
                        .await?;

                    for related_result in related_results {
                        if seen_ids.insert(related_result.point_id.clone()) {
                            all_results.push(related_result);
                        }
                    }
                }

                all_results.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                let context = self.context_builder.build_context(all_results.clone())?;
                return Ok(RetrievedContext { context, documents: all_results });
            }
        }
    }
}

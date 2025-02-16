use crate::rag_system::types::{Document, DocumentType, RAGConfig, RetrievedContext, W3ADocument};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashSet;
use tracing::info;

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
pub trait PayloadKeyBasedRetriever {
    async fn search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        similarity_threshold: f32,
    ) -> Result<Vec<W3ADocument>>;
    async fn search_by_payload_key(
        &self,
        payload_key: &str,
        payload_value: &str,
    ) -> Result<Vec<W3ADocument>>;
}

#[async_trait]
pub trait ContextBuilder {
    fn build_context(&self, documents: Vec<DocumentType>) -> Result<String>;
}

pub struct RAGSystem<V, R, K, C>
where
    V: Vectorizer,
    R: Retriever,
    K: PayloadKeyBasedRetriever,
    C: ContextBuilder,
{
    vectorizer: V,
    retriever: R,
    payload_key_based_retriever: K,
    context_builder: C,
    config: RAGConfig,
}

impl<V, R, L, C> RAGSystem<V, R, L, C>
where
    V: Vectorizer,
    R: Retriever,
    L: PayloadKeyBasedRetriever,
    C: ContextBuilder,
{
    pub fn new(
        vectorizer: V,
        retriever: R,
        payload_key_based_retriever: L,
        context_builder: C,
        config: RAGConfig,
    ) -> Self {
        Self {
            vectorizer,
            retriever,
            payload_key_based_retriever,
            context_builder,
            config,
        }
    }

    pub async fn process(&self, query: &str) -> Result<RetrievedContext> {
        let vector = self.vectorizer.vectorize(query).await?;

        let results: Vec<DocumentType> = match &self.config {
            RAGConfig::Default {
                max_documents,
                similarity_threshold,
            } => {
                let results = self
                    .retriever
                    .search(vector.clone(), *max_documents, *similarity_threshold)
                    .await?;
                results.into_iter().map(DocumentType::Default).collect()
            }
            RAGConfig::Advanced {
                base_max_documents,
                base_similarity_threshold,
                related_max_documents,
                related_similarity_threshold,
            } => {
                let base_results = self
                    .retriever
                    .search(
                        vector.clone(),
                        *base_max_documents,
                        *base_similarity_threshold,
                    )
                    .await?;
                info!(
                    "TEMP LOG: Documents quantity in base results: {}",
                    base_results.len()
                );

                let mut all_results = base_results.clone();

                let mut seen_ids = base_results
                    .iter()
                    .map(|doc| doc.point_id.clone())
                    .collect::<HashSet<_>>();

                for result in &base_results {
                    let base_vector = match &result.vector {
                        Some(vector) => {
                            info!("TEMP LOG: Vector used from Document performing search with Advanced RAGConfig...Ok");
                            vector.clone()
                        }
                        None => self.vectorizer.vectorize(&result.text).await?,
                    };

                    let related_results = self
                        .retriever
                        .search(
                            base_vector,
                            *related_max_documents,
                            *related_similarity_threshold,
                        )
                        .await?;

                    info!(
                        "TEMP LOG: Documents quantity in related per one iteration: {}",
                        related_results.len()
                    );

                    for related_result in related_results {
                        if seen_ids.insert(related_result.point_id.clone()) {
                            info!("TEMP LOG: Related point is unique. All's good");
                            all_results.push(related_result);
                        } else {
                            info!("TEMP LOG: Related point is a duplicate. No need to push");
                        }
                    }
                }

                info!(
                    "TEMP LOG: Documents quantity in the end of the search: {}",
                    all_results.len()
                );

                all_results.into_iter().map(DocumentType::Default).collect()
            }
            RAGConfig::PayloadKeyBased {
                max_documents,
                similarity_threshold,
            } => {
                let payload_key = "lesson_title".to_string();
                
                let initial_results = self
                    .payload_key_based_retriever
                    .search(vector.clone(), *max_documents, *similarity_threshold)
                    .await?;

                let mut all_documents_by_payload_key = Vec::new();

                if let Some(first_doc) = initial_results.first() {
                    let lesson_title = &first_doc.lesson_title;
                    let all_results_by_payload_key = self
                        .payload_key_based_retriever
                        .search_by_payload_key(&payload_key, lesson_title)
                        .await?;

                    all_documents_by_payload_key.extend(all_results_by_payload_key);
                }

                all_documents_by_payload_key
                    .into_iter()
                    .map(DocumentType::W3A)
                    .collect()
            }
        };

        let context = self.context_builder.build_context(results.clone())?;

        Ok(RetrievedContext {
            context,
            documents: results,
        })
    }
}

pub fn get_default_rag_config() -> RAGConfig {
    RAGConfig::Default {
        max_documents: 12,
        similarity_threshold: 0.3,
    }
}

pub fn get_advanced_rag_config() -> RAGConfig {
    RAGConfig::Advanced {
        base_max_documents: 5,
        base_similarity_threshold: 0.4,
        related_max_documents: 4,
        related_similarity_threshold: 0.4,
    }
}

pub fn get_payload_key_based_rag_config() -> RAGConfig {
    RAGConfig::PayloadKeyBased {
        max_documents: 1,
        similarity_threshold: 0.5,  
    }
}

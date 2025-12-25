use crate::rag_system::hybrid_search_types::{
    ChunkSearchResult, DescriptionSearchResult, DocumentAggregation, HybridSearchDocument,
};
use crate::rag_system::types::{
    Document, DocumentType, PointId, RAGConfig, RankingMethod, RetrievedContext, W3ADocument,
};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

pub mod context_builder;
pub mod get_results_via_rag_system;
pub mod hybrid_search_types;
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
pub trait HybridSearchRetriever {
    async fn search_chunks(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        similarity_threshold: f32,
    ) -> Result<Vec<ChunkSearchResult>>;

    async fn search_descriptions(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        similarity_threshold: f32,
    ) -> Result<Vec<DescriptionSearchResult>>;

    async fn load_full_document(&self, document_id: &str) -> Result<Vec<ChunkSearchResult>>;
}

#[async_trait]
pub trait ContextBuilder {
    fn build_context(&self, documents: Vec<DocumentType>) -> Result<String>;
}

pub struct RAGSystem<V, R, K, H, C>
where
    V: Vectorizer,
    R: Retriever,
    K: PayloadKeyBasedRetriever,
    H: HybridSearchRetriever,
    C: ContextBuilder,
{
    vectorizer: V,
    retriever: R,
    payload_key_based_retriever: K,
    hybrid_search_retriever: H,
    context_builder: C,
    config: RAGConfig,
}

impl<V, R, K, H, C> RAGSystem<V, R, K, H, C>
where
    V: Vectorizer,
    R: Retriever,
    K: PayloadKeyBasedRetriever,
    H: HybridSearchRetriever,
    C: ContextBuilder,
{
    pub fn new(
        vectorizer: V,
        retriever: R,
        payload_key_based_retriever: K,
        hybrid_search_retriever: H,
        context_builder: C,
        config: RAGConfig,
    ) -> Self {
        Self {
            vectorizer,
            retriever,
            payload_key_based_retriever,
            hybrid_search_retriever,
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

                let mut all_results = base_results.clone();

                let mut seen_ids = base_results
                    .iter()
                    .map(|doc| doc.point_id.clone())
                    .collect::<HashSet<_>>();

                for result in &base_results {
                    let base_vector = match &result.vector {
                        Some(vector) => vector.clone(),
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

                    for related_result in related_results {
                        if seen_ids.insert(related_result.point_id.clone()) {
                            all_results.push(related_result);
                        }
                    }
                }

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
                    let document_title = &first_doc.lesson_title;
                    let all_results_by_payload_key = self
                        .payload_key_based_retriever
                        .search_by_payload_key(&payload_key, document_title)
                        .await?;

                    all_documents_by_payload_key.extend(all_results_by_payload_key);
                }

                all_documents_by_payload_key
                    .into_iter()
                    .map(DocumentType::W3A)
                    .collect()
            }
            RAGConfig::HybridSearch {
                top_k_chunks,
                chunks_similarity_threshold,
                top_k_descriptions,
                descriptions_similarity_threshold,
                ranking_method,
                final_documents_count,
            } => {
                self.process_hybrid_search(
                    vector,
                    *top_k_chunks,
                    *chunks_similarity_threshold,
                    *top_k_descriptions,
                    *descriptions_similarity_threshold,
                    ranking_method,
                    *final_documents_count,
                )
                .await?
            }
        };

        let context = self.context_builder.build_context(results.clone())?;

        Ok(RetrievedContext {
            context,
            documents: results,
        })
    }

    async fn process_hybrid_search(
        &self,
        query_vector: Vec<f32>,
        top_k_chunks: usize,
        chunks_similarity_threshold: f32,
        top_k_descriptions: usize,
        descriptions_similarity_threshold: f32,
        ranking_method: &RankingMethod,
        final_documents_count: usize,
    ) -> Result<Vec<DocumentType>> {
        info!("Starting Hybrid Search...");

        // ============ STEP 1: Searching chunks and descriptions ============
        let (chunks_result, descriptions_result) = tokio::join!(
            self.hybrid_search_retriever.search_chunks(
                query_vector.clone(),
                top_k_chunks,
                chunks_similarity_threshold
            ),
            self.hybrid_search_retriever.search_descriptions(
                query_vector,
                top_k_descriptions,
                descriptions_similarity_threshold
            )
        );

        let chunks = chunks_result?;
        let descriptions = descriptions_result?;

        info!(
            "Found {} chunks and {} descriptions",
            chunks.len(),
            descriptions.len()
        );

        // ============ STEP 2: Processing chunks and descriptions ============
        let mut documents_map: HashMap<String, DocumentAggregation> = HashMap::new();

        // Chunks processing
        for (rank, chunk) in chunks.into_iter().enumerate() {
            documents_map
                .entry(chunk.document_id.clone())
                .and_modify(|document| {
                    let current_max = document.max_chunk_score.unwrap_or(0.0);
                    document.max_chunk_score = Some(current_max.max(chunk.score));

                    document.matched_chunks.push(chunk.clone());
                })
                .or_insert_with(|| DocumentAggregation {
                    document_id: chunk.document_id.clone(),
                    metadata: chunk.metadata.clone(),
                    matched_chunks: vec![chunk.clone()],
                    max_chunk_score: Some(chunk.score),
                    chunk_rank: Some(rank),
                    description_score: None,
                    description_rank: None,
                    final_score: 0.0,
                });
        }

        // Description processing
        for (rank, description) in descriptions.into_iter().enumerate() {
            documents_map
                .entry(description.document_id.clone())
                .and_modify(|document| {
                    document.description_score = Some(description.score);
                    document.description_rank = Some(rank);
                })
                .or_insert_with(|| DocumentAggregation {
                    document_id: description.document_id.clone(),
                    metadata: description.metadata.clone(),
                    matched_chunks: vec![],
                    max_chunk_score: None,
                    chunk_rank: None,
                    description_score: Some(description.score),
                    description_rank: Some(rank),
                    final_score: 0.0,
                });
        }

        // ============ STEP 3: Ranking ============
        let mut aggregated_documents: Vec<DocumentAggregation> =
            documents_map.into_values().collect();

        info!(
            "Aggregated into {} unique documents",
            aggregated_documents.len()
        );

        for document in &mut aggregated_documents {
            document.final_score = match ranking_method {
                RankingMethod::RRF { k } => {
                    let chunk_component = document
                        .chunk_rank
                        .map(|rank| 1.0 / (k + rank as f32))
                        .unwrap_or(0.0);

                    let description_component = document
                        .description_rank
                        .map(|rank| 1.0 / (k + rank as f32))
                        .unwrap_or(0.0);

                    chunk_component + description_component
                }
                RankingMethod::WeightedSum {
                    chunk_weight,
                    description_weight,
                } => {
                    let chunk_score = document.max_chunk_score.unwrap_or(0.0);
                    let desc_score = document.description_score.unwrap_or(0.0);

                    chunk_weight * chunk_score + description_weight * desc_score
                }
            };
        }

        aggregated_documents.sort_by(|a, b| {
            b.final_score
                .partial_cmp(&a.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        info!(
            "Top document after ranking: {} (score: {})",
            aggregated_documents
                .first()
                .map(|l| l.metadata.title.as_str())
                .unwrap_or("none"),
            aggregated_documents
                .first()
                .map(|l| l.final_score)
                .unwrap_or(0.0)
        );

        // ============ Step 4: Picking top-N documents ============
        let top_documents: Vec<DocumentAggregation> = aggregated_documents
            .into_iter()
            .take(final_documents_count)
            .collect();

        info!(
            "Selected {} documents for final context",
            top_documents.len()
        );

        let mut top_doc_counter = 1;

        for document in &top_documents {
            info!(
                "Selected document ({}/{}) for final context:\n{}",
                top_doc_counter,
                top_documents.len(),
                document.metadata.title
            );
            top_doc_counter += 1;
        }

        // ============ Step 5: Retrieving full documents' content ============
        let mut final_documents = Vec::new();

        for document_agg in top_documents {
            let all_chunks = self
                .hybrid_search_retriever
                .load_full_document(&document_agg.document_id)
                .await?;

            if all_chunks.is_empty() {
                warn!(
                    "Warning: No chunks found for document {}",
                    document_agg.document_id
                );
                continue;
            }

            let full_text = all_chunks
                .iter()
                .map(|chunk| chunk.chunk_text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");

            let matched_chunk_indices: Vec<u32> = document_agg
                .matched_chunks
                .iter()
                .map(|chunk| chunk.chunk_index)
                .collect();

            let hybrid_doc = HybridSearchDocument {
                point_id: PointId::Uuid(document_agg.document_id.clone()),
                text: full_text,
                score: document_agg.final_score,
                vector: None,
                document_id: document_agg.document_id,
                metadata: document_agg.metadata.clone(),
                matched_chunk_indices,
            };

            final_documents.push(DocumentType::HybridSearch(hybrid_doc));
        }

        info!(
            "Successfully loaded {} full documents",
            final_documents.len()
        );

        Ok(final_documents)
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

pub fn get_hybrid_search_rag_config() -> RAGConfig {
    RAGConfig::HybridSearch {
        top_k_chunks: 10,
        chunks_similarity_threshold: 0.5,
        top_k_descriptions: 5,
        descriptions_similarity_threshold: 0.5,
        ranking_method: RankingMethod::RRF { k: 60.0 },
        final_documents_count: 3,
    }
}

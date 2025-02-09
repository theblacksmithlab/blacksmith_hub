use crate::rag_system::types::{Document, DocumentMetadata, PointId};
use crate::rag_system::Retriever;
use crate::state::qdrant_client_init_trait::QdrantClientInit;
use anyhow::Result;
use async_trait::async_trait;
use qdrant_client::qdrant::{point_id, vectors_output, SearchParamsBuilder, SearchPointsBuilder};
use qdrant_client::Qdrant;
use std::sync::Arc;
use tracing::error;

pub struct QdrantRetriever<T: QdrantClientInit> {
    app_state: Arc<T>,
    collection_names: Vec<String>,
}

impl<T: QdrantClientInit> QdrantRetriever<T> {
    pub fn new(app_state: Arc<T>, collection_names: Vec<String>) -> Self {
        Self {
            app_state,
            collection_names,
        }
    }

    pub fn get_qdrant_client(&self) -> Arc<Qdrant> {
        self.app_state.get_qdrant_client()
    }
}

#[async_trait]
impl<T: QdrantClientInit + Send + Sync> Retriever for QdrantRetriever<T> {
    async fn search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        similarity_threshold: f32,
    ) -> Result<Vec<Document>> {
        let mut all_documents = Vec::new();

        for collection_name in &self.collection_names {
            let search_request = SearchPointsBuilder::new(
                collection_name.clone(),
                query_vector.clone(),
                limit as u64,
            )
            .with_payload(true)
            .with_vectors(true)
            .score_threshold(similarity_threshold)
            .params(SearchParamsBuilder::default().exact(true))
            .build();

            match self.get_qdrant_client().search_points(search_request).await {
                Ok(response) => {
                    let documents = response.result.into_iter().map(|point| {
                        let point_id = match point.id {
                            Some(id) => match id.point_id_options {
                                Some(point_id::PointIdOptions::Num(num)) => PointId::Num(num),
                                Some(point_id::PointIdOptions::Uuid(uuid)) => PointId::Uuid(uuid),
                                None => PointId::Uuid("unknown".to_string()),
                            },
                            None => PointId::Uuid("unknown".to_string()),
                        };

                        let content = point
                            .payload
                            .get("text")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| String::new());

                        let source = point
                            .payload
                            .get("source")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| collection_name.clone());

                        let timestamp = point.payload.get("timestamp").and_then(|v| v.as_integer());

                        let vector = match &point.vectors {
                            Some(vectors_output) => match &vectors_output.vectors_options {
                                Some(vectors_output::VectorsOptions::Vector(single_vector)) => {
                                    Some(single_vector.data.clone())
                                }
                                Some(vectors_output::VectorsOptions::Vectors(named_vectors)) => {
                                    error!("Named vectors retrieval option is not supported yet!");
                                    named_vectors
                                        .vectors
                                        .values()
                                        .next()
                                        .map(|v| v.data.clone())
                                }
                                None => None,
                            },
                            None => None,
                        };

                        Document {
                            point_id,
                            content,
                            score: Some(point.score),
                            metadata: Some(DocumentMetadata { source, timestamp }),
                            vector,
                        }
                    });

                    all_documents.extend(documents);
                }
                Err(e) => {
                    error!("Error searching in collection {}: {:?}", collection_name, e);
                }
            }
        }

        all_documents.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        all_documents.truncate(limit);

        Ok(all_documents)
    }
}

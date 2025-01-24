use std::sync::Arc;
use crate::rag_system::types::{Document, DocumentMetadata};
use crate::rag_system::Retriever;
use anyhow::Result;
use async_trait::async_trait;
use qdrant_client::qdrant::{SearchParamsBuilder, SearchPointsBuilder};
use qdrant_client::Qdrant;
use tracing::error;

pub struct QdrantRetriever {
    client: Arc<Qdrant>,
    collection_name: String,
}

impl QdrantRetriever {
    pub fn new(client: Arc<Qdrant>, collection_name: String) -> Self {
        Self {
            client,
            collection_name,
        }
    }
}

#[async_trait]
impl Retriever for QdrantRetriever {
    async fn search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        similarity_threshold: f32,
    ) -> Result<Vec<Document>> {
        let search_request =
            SearchPointsBuilder::new(self.collection_name.clone(), query_vector, limit as u64)
                .with_payload(true)
                .with_vectors(true)
                .score_threshold(similarity_threshold)
                .params(SearchParamsBuilder::default().exact(true))
                .build();

        let response = match self.client.search_points(search_request).await {
            Ok(response) => response,
            Err(e) => {
                error!(
                    "Error searching in collection {}: {:?}",
                    self.collection_name, e
                );
                return Err(anyhow::anyhow!(e));
            }
        };

        let documents = response
            .result
            .into_iter()
            .map(|point| {
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
                    .unwrap_or_else(|| String::new());

                let timestamp = point.payload.get("timestamp").and_then(|v| v.as_integer());

                Document {
                    content,
                    metadata: Some(DocumentMetadata { source, timestamp }),
                    score: Some(point.score),
                }
            })
            .collect();

        Ok(documents)
    }
}

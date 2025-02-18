use crate::rag_system::types::{Document, PointId, W3ADocument};
use crate::rag_system::{PayloadKeyBasedRetriever, Retriever};
use crate::state::qdrant_client_init_trait::QdrantClientInit;
use anyhow::Result;
use async_trait::async_trait;
use qdrant_client::qdrant::r#match::MatchValue;
use qdrant_client::qdrant::{
    point_id, vectors_output, Condition, FieldCondition, Filter, Match, ScrollPointsBuilder,
    SearchParamsBuilder, SearchPointsBuilder,
};
use qdrant_client::Qdrant;
use std::sync::Arc;
use tracing::{error, info};

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

                        let text = point
                            .payload
                            .get("text")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| String::new());

                        let score = point.score;

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
                            text,
                            score,
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

#[async_trait]
impl<T: QdrantClientInit + Send + Sync> PayloadKeyBasedRetriever for QdrantRetriever<T> {
    async fn search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        similarity_threshold: f32,
    ) -> Result<Vec<W3ADocument>> {
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

                        let text = point
                            .payload
                            .get("text")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| String::new());

                        let score = point.score;

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

                        let module = point
                            .payload
                            .get("module")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| String::new());

                        let block_title = point
                            .payload
                            .get("block_title")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| String::new());

                        let lesson_title = point
                            .payload
                            .get("lesson_title")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| String::new());

                        let segment_id = point
                            .payload
                            .get("segment_id")
                            .and_then(|v| v.as_integer())
                            .unwrap_or(0);

                        W3ADocument {
                            point_id,
                            text,
                            score: Some(score),
                            vector,
                            module,
                            block_title,
                            lesson_title,
                            segment_id,
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

    async fn search_by_payload_key(
        &self,
        payload_key: &str,
        payload_value: &str,
    ) -> Result<Vec<W3ADocument>> {
        let mut all_documents = Vec::new();

        for collection_name in &self.collection_names {
            let filter = Filter::must(vec![Condition {
                condition_one_of: Some(qdrant_client::qdrant::condition::ConditionOneOf::Field(
                    FieldCondition {
                        key: payload_key.to_string(),
                        r#match: Some(Match {
                            match_value: Some(MatchValue::Keyword(payload_value.to_string())),
                        }),
                        ..Default::default()
                    },
                )),
            }]);

            let mut all_results = Vec::new();

            let mut next_page_offset = None;

            loop {
                info!(
                    "Entering the loop of searching results by payload_key. Next page offset: {:?}",
                    next_page_offset
                );
                let mut scroll_request = ScrollPointsBuilder::new(collection_name.clone())
                    .filter(filter.clone())
                    .with_payload(true)
                    .with_vectors(true);

                if let Some(offset) = next_page_offset {
                    scroll_request = scroll_request.offset(offset);
                }

                let response = self
                    .get_qdrant_client()
                    .scroll(scroll_request.build())
                    .await?;

                if response.result.is_empty() {
                    break;
                }
                all_results.extend(response.result.iter().cloned());

                if response.next_page_offset.is_none() {
                    info!("Next page offset is None, breaking loop.");
                    break;
                }

                next_page_offset = response.next_page_offset.clone();
            }

            info!(
                "TEMP log: results by payload key quantity: {} ",
                all_results.len()
            );

            let documents: Vec<W3ADocument> = all_results
                .into_iter()
                .map(|point| {
                    let point_id = match point.id {
                        Some(id) => match id.point_id_options {
                            Some(point_id::PointIdOptions::Num(num)) => PointId::Num(num),
                            Some(point_id::PointIdOptions::Uuid(uuid)) => PointId::Uuid(uuid),
                            None => PointId::Uuid("unknown".to_string()),
                        },
                        None => PointId::Uuid("unknown".to_string()),
                    };

                    let text = point
                        .payload
                        .get("text")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                        .unwrap_or_else(|| String::new());

                    let module = point
                        .payload
                        .get("module")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                        .unwrap_or_else(|| String::new());

                    let block_title = point
                        .payload
                        .get("block_title")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                        .unwrap_or_else(|| String::new());

                    let lesson_title = point
                        .payload
                        .get("lesson_title")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                        .unwrap_or_else(|| String::new());

                    let segment_id = point
                        .payload
                        .get("segment_id")
                        .and_then(|v| v.as_integer())
                        .unwrap_or(0);

                    W3ADocument {
                        point_id,
                        text,
                        score: None,
                        vector: None,
                        module,
                        block_title,
                        lesson_title,
                        segment_id,
                    }
                })
                .collect();

            all_documents.extend(documents);
        }

        all_documents.sort_by(|a, b| a.segment_id.cmp(&b.segment_id));

        Ok(all_documents)
    }
}

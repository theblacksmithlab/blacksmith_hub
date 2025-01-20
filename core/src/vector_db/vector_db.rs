use crate::ai::utils::vectorize;
use crate::state::request_app::app_state::{RequestAppState, UserSearchResults};
use anyhow::Result;
use qdrant_client::qdrant::condition::ConditionOneOf;
use qdrant_client::qdrant::r#match::MatchValue;
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::{
    Condition, CreateCollection, Distance, FieldCondition, Filter, Match, PointStruct, ScoredPoint,
    SearchParamsBuilder, SearchPointsBuilder, UpsertPointsBuilder, VectorParams, VectorsConfig,
};
use qdrant_client::Payload;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use teloxide::prelude::ChatId;
use tracing::info;

#[derive(Debug)]
pub enum ExecutionMode {
    Full,
    Partial,
}

pub enum QdrantSearchResult {
    Partial(Vec<ScoredPoint>),
    Full(Option<String>),
}

pub async fn qdrant_upsert(
    app_state: Arc<RequestAppState>,
    data_to_upsert: String,
    collection_name: &String,
    user_id: ChatId,
    username: String,
) -> Result<()> {
    let qdrant_client = &app_state.qdrant_client;
    let vector = vectorize(data_to_upsert.clone(), app_state.clone()).await?;

    if !qdrant_client.collection_exists(collection_name).await? {
        let details = CreateCollection {
            collection_name: collection_name.clone(),
            vectors_config: Some(VectorsConfig {
                config: Some(Config::Params(VectorParams {
                    size: vector.len() as u64,
                    distance: Distance::Cosine.into(),
                    ..Default::default()
                })),
            }),
            ..Default::default()
        };
        qdrant_client.create_collection(details).await?;
    }

    let filter = Filter::must(vec![Condition {
        condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
            key: "user_id".to_string(),
            r#match: Some(Match {
                match_value: Some(MatchValue::Integer(user_id.0)),
            }),
            ..Default::default()
        })),
    }]);

    let dummy_vector = vec![0.0; vector.len()];
    let existing_points = qdrant_client
        .search_points(
            SearchPointsBuilder::new(collection_name, dummy_vector, 1)
                .with_payload(true)
                .filter(filter),
        )
        .await?;

    let payload_json = json!({
        "user_id": user_id,
        "username": username,
        "text": data_to_upsert,
    });
    let payload: Payload = serde_json::from_value(payload_json)?;

    if let Some(existing_point) = existing_points.result.get(0) {
        info!("Overwriting existing point's payload!");
        if let Some(point_id) = existing_point.clone().id {
            qdrant_client
                .upsert_points(UpsertPointsBuilder::new(
                    collection_name.clone(),
                    vec![PointStruct::new(point_id, vector, payload)],
                ))
                .await?;
        } else {
            info!("Point has no id!?");
        }
    } else {
        info!("Uploading initial point's payload!");
        let current_point_count = qdrant_client
            .collection_info(collection_name)
            .await?
            .result
            .unwrap_or_default()
            .points_count
            .unwrap_or(0);

        // TODO: potential bug: improve current_point_count determination
        let new_point_id = current_point_count + 1;

        qdrant_client
            .upsert_points(UpsertPointsBuilder::new(
                collection_name,
                vec![PointStruct::new(new_point_id, vector, payload)],
            ))
            .await?;
    }

    Ok(())
}

pub async fn qdrant_search(
    app_state: Arc<RequestAppState>,
    query_vector: Vec<f32>,
    collection_name: &String,
    score: f32,
    limit: u64,
    user_id: ChatId,
    mode: ExecutionMode,
) -> Result<QdrantSearchResult> {
    info!("Fn: qdrant_search | Execution mode: {:?}", mode);

    let qdrant_client = &app_state.qdrant_client;

    let collections_to_search_in = vec![&*collection_name];

    let mut all_results = Vec::new();

    for collection_name in collections_to_search_in {
        match qdrant_client
            .search_points(
                SearchPointsBuilder::new(collection_name, query_vector.clone(), limit)
                    .with_payload(true)
                    .params(SearchParamsBuilder::default().exact(true)),
            )
            .await
        {
            Ok(result) => all_results.extend(result.result),
            Err(_) => {
                info!(
                    "Collection '{}' does not exist. It will be handled during upsert fn.",
                    collection_name
                );
            }
        }
    }

    // info!(
    //     "Fn qdrant_search | Unfiltered search results including user's request: {:?}",
    //     all_results
    // );

    let mut filtered_results: Vec<_> = all_results
        .into_iter()
        .filter(|point| {
            point.score > score
                && point
                    .payload
                    .get("user_id")
                    .map(|id| id.to_string().parse::<i64>().ok())
                    .map_or(true, |id| id != Option::from(user_id.0))
        })
        .collect();

    filtered_results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    info!(
        "Fn: qdrant_search | filtered_results check:\n{:?}",
        filtered_results
    );

    match mode {
        ExecutionMode::Partial => Ok(QdrantSearchResult::Partial(filtered_results)),
        ExecutionMode::Full => {
            let mut combined_payloads = Vec::new();
            let mut payload_authors = Vec::new();

            for found_point in filtered_results.clone() {
                if let Some(payload) = found_point.payload.get("text") {
                    if let Some(text) = payload.as_str() {
                        combined_payloads.push(text.to_string());
                    }
                }
            }

            for found_point in filtered_results {
                if let Some(payload_author) = found_point.payload.get("username") {
                    if let Some(username) = payload_author.as_str() {
                        payload_authors.push(username.to_string());
                    }
                }
            }

            if combined_payloads.is_empty() {
                Ok(QdrantSearchResult::Full(None))
            } else {
                let combined_payload = combined_payloads.join("\n");
                let author_username = payload_authors.join(", ");
                info!(
                    "Combined_payload excluding user's request: {}",
                    combined_payload
                );

                // let new_combined_payload = tokenize_and_truncate(combined_payload)?;
                // info!("new_combined_payload: {:?}", new_combined_payload);
                // Ok(new_combined_payload)

                let mut authors_map = app_state.last_request_result_author.lock().await;
                authors_map.insert(user_id, author_username);

                Ok(QdrantSearchResult::Full(Some(combined_payload)))
            }
        }
    }
}

pub(crate) async fn restore_request_from_qdrant(
    app_state: Arc<RequestAppState>,
    limit: u64,
    user_id: ChatId,
) -> Result<Option<String>> {
    let qdrant_client = &app_state.qdrant_client;

    let collection_name = "outside_app".to_string();

    let collections_to_search_in = vec![collection_name];

    let filter = Filter::must(vec![Condition {
        condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
            key: "user_id".to_string(),
            r#match: Some(Match {
                match_value: Some(MatchValue::Integer(user_id.0)),
            }),
            ..Default::default()
        })),
    }]);

    let dummy_vector = vec![0.0; 3072];

    let mut all_results = Vec::new();

    for collection_name in collections_to_search_in.clone() {
        match qdrant_client
            .search_points(
                SearchPointsBuilder::new(collection_name.clone(), dummy_vector.clone(), limit)
                    .with_payload(true)
                    .filter(filter.clone()),
            )
            .await
        {
            Ok(result) => all_results.extend(result.result),
            Err(err) => {
                eprintln!(
                    "Error searching qdrant collection '{}': {:?}",
                    collection_name, err
                );
            }
        }
    }

    info!(
        "Fn: restore_request_from_qdrant | Restoring user request text from qdrant: {:?}",
        all_results
    );

    let mut combined_payloads = Vec::new();
    for found_point in all_results {
        if let Some(payload) = found_point.payload.get("text") {
            if let Some(text) = payload.as_str() {
                combined_payloads.push(text.to_string());
            }
        }
    }

    if combined_payloads.is_empty() {
        Ok(None)
    } else {
        let combined_payload = combined_payloads.join("\n");

        // let new_combined_payload = tokenize_and_truncate(combined_payload)?;
        // info!("new_combined_payload: {:?}", new_combined_payload);
        // Ok(new_combined_payload)

        let mut requests = app_state.user_request.lock().await;
        requests.insert(user_id, combined_payload.clone());

        Ok(Some(combined_payload))
    }
}

pub async fn store_search_results(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    filtered_results: Vec<ScoredPoint>,
) {
    let mut raw_search_results = BTreeMap::new();
    let mut order = Vec::new();

    for (index, point) in filtered_results.into_iter().enumerate() {
        raw_search_results.insert(index + 1, point);
        order.push(index + 1);
    }

    let user_search_results = UserSearchResults {
        points: raw_search_results,
        order,
    };

    let mut search_results_map = app_state.user_search_results.lock().await;

    search_results_map.insert(user_id, user_search_results);

    // if let Some(user_results) = search_results_map.get(&user_id) {
    //     info!("Stored search results for user_id {}: {:?}", user_id, user_results);
    //
    //     info!(
    //         "Results (points): {:?}",
    //         user_results.points.iter().collect::<Vec<_>>()
    //     );
    //     info!(
    //         "Order of results: {:?}",
    //         user_results.order
    //     );
    // }

    info!(
        "Fn: store_search_results | Search results for user_id {}: {:?} stored in App State",
        user_id,
        search_results_map.get(&user_id)
    );
}

pub async fn prepare_search_results_for_llm(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
) -> String {
    let search_results_map = app_state.user_search_results.lock().await;

    if let Some(user_search_results) = search_results_map.get(&user_id) {
        let mut data_ro_process_via_llm = Vec::new();

        for (index, point) in &user_search_results.points {
            if let Some(text) = point.payload.get("text") {
                data_ro_process_via_llm.push(format!("{}. {}", index, text));
            }
        }

        return data_ro_process_via_llm.join("\n");
    }

    "".to_string()
}

pub async fn get_llm_order_from_response(llm_response: &str) -> Result<Vec<usize>, String> {
    let parsed_json_from_llm: Value =
        serde_json::from_str(llm_response).map_err(|e| format!("Error parsing JSON: {}", e))?;

    if let Some(new_order) = parsed_json_from_llm.get("order") {
        if let Some(new_order_vec) = new_order.as_array() {
            let mut new_order_indices = Vec::new();
            for item in new_order_vec {
                if let Some(index) = item.as_u64() {
                    new_order_indices.push(index as usize);
                }
            }
            return Ok(new_order_indices);
        }
    }

    Err("Error getting new order from LLM".to_string())
}

pub async fn update_search_results_order(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    new_order: Vec<usize>,
) {
    let mut search_results_map = app_state.user_search_results.lock().await;

    if let Some(user_search_results) = search_results_map.get_mut(&user_id) {
        info!(
            "Fn: update_search_results_order | Updating order for user_id {}: current order {:?}, new order {:?}",
            user_id,
            user_search_results.order,
            new_order
        );

        user_search_results.order = new_order.clone();

        let mut sorted_points = Vec::new();
        for &index in &new_order {
            if let Some(point) = user_search_results.points.get(&index) {
                sorted_points.push((index, point.clone()));
            } else {
                info!(
                    "Fn: update_search_results_order | No point found for index {}",
                    index
                );
                continue;
            }
        }

        user_search_results.points = sorted_points.into_iter().collect();
    } else {
        info!(
            "Fn: update_search_results_order | No search results found for user_id {}",
            user_id
        );
    }
}

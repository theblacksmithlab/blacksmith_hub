use anyhow::Result;
use async_openai::config::OpenAIConfig;
use async_openai::types::{CreateEmbeddingRequestArgs, CreateEmbeddingResponse};
use async_openai::Client as LLM_Client;
use core::utils::common::LlmModel;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointStruct, UpsertPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::Qdrant;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
use walkdir::WalkDir;

const INDEX_FILE: &str = "./common_res/w3a/w3a_qdrant_index.json";

pub async fn upsert_data_to_qdrant(
    qdrant_client: Arc<Qdrant>,
    llm_client: LLM_Client<OpenAIConfig>,
) -> Result<()> {
    let input_dir = "./tools/tmp/input_jsons";
    let collection_name = "w3a_main";

    if !qdrant_client.collection_exists(collection_name).await? {
        qdrant_client
            .create_collection(
                CreateCollectionBuilder::new(collection_name)
                    .vectors_config(VectorParamsBuilder::new(3072, Distance::Cosine)),
            )
            .await?;
    }

    let mut index = load_index()?;
    let mut uploaded_count = 0;

    for entry in WalkDir::new(input_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() || path.extension().unwrap_or_default() != "json" {
            continue;
        }

        let file_content = fs::read_to_string(path)?;
        let json_data: Value = serde_json::from_str(&file_content)?;

        let point_id = Uuid::new_v4().to_string();
        let mut payload = json_data.as_object().cloned().unwrap_or_default();

        // Applying to_lowercase to all titles
        for key in ["block_title", "lesson_title", "module"] {
            if let Some(value) = payload.get_mut(key) {
                if let Some(str_value) = value.as_str() {
                    *value = Value::String(str_value.to_lowercase());
                }
            }
        }

        // Changing 'content' key for 'text'
        if let Some(content) = payload.remove("content") {
            payload.insert("text".to_string(), content);
        }

        let point_content = payload
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let vector = vectorize(point_content, llm_client.clone()).await?;

        qdrant_client
            .upsert_points(UpsertPointsBuilder::new(
                collection_name,
                vec![PointStruct::new(point_id.clone(), vector, payload)],
            ))
            .await?;

        index.insert(point_id, path.to_string_lossy().to_string());

        uploaded_count += 1;

        info!(
            "File {} uploaded successfully. Files processed: {}",
            path.to_string_lossy(),
            uploaded_count
        );
    }

    save_index(&index)?;

    info!(
        "All JSON files have been successfully uploaded to Qdrant db! Total files uploaded: {}",
        uploaded_count
    );

    Ok(())
}

async fn vectorize(data: String, llm_client: LLM_Client<OpenAIConfig>) -> Result<Vec<f32>> {
    let request = CreateEmbeddingRequestArgs::default()
        .model(LlmModel::TextEmbedding3Large.as_str())
        .input(data)
        .build()?;

    let response: CreateEmbeddingResponse = llm_client.embeddings().create(request).await?;
    let embedding = response.data.into_iter().next().unwrap().embedding;

    Ok(embedding)
}

fn load_index() -> Result<HashMap<String, String>> {
    if Path::new(INDEX_FILE).exists() {
        let content = fs::read_to_string(INDEX_FILE)?;
        let index: HashMap<String, String> = serde_json::from_str(&content)?;
        Ok(index)
    } else {
        Ok(HashMap::new())
    }
}

fn save_index(index: &HashMap<String, String>) -> Result<()> {
    let json_data = json!(index);
    fs::create_dir_all(Path::new(INDEX_FILE).parent().unwrap())?;
    fs::write(INDEX_FILE, serde_json::to_string_pretty(&json_data)?)?;
    Ok(())
}

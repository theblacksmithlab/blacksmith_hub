use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::state::request_app::app_state::RequestAppState;
use crate::state::request_app::app_state::UserProfile;
use crate::vector_db::vector_db::qdrant_upsert;
use anyhow::{Context, Result};
use async_openai::types::ResponseFormat::JsonObject;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs, CreateEmbeddingRequestArgs, CreateEmbeddingResponse,
};
use std::fs;

use crate::local_db::local_db::save_user_profile;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::RequestAppSystemRoleType;
use crate::utils::common::get_system_role_or_fallback;
use crate::utils::common::LlmModel;
use std::sync::Arc;
use teloxide::prelude::ChatId;
use tiktoken_rs::cl100k_base;
use tracing::info;

pub async fn raw_llm_processing_json<T: OpenAIClientInit + Send + Sync>(
    system_role: &str,
    request: &str,
    app_state: Arc<T>,
    model: LlmModel,
) -> Result<String> {
    let llm_client = app_state.get_llm_client().clone();

    let llm_request = CreateChatCompletionRequestArgs::default()
        .max_tokens(4095u32)
        .model(model.as_str())
        .temperature(0.2)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_role)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(request)
                .build()?
                .into(),
        ])
        .response_format(JsonObject)
        .build()?;

    let response = llm_client.chat().create(llm_request).await?;

    if let Some(choice) = response.choices.get(0) {
        let content = choice
            .message
            .content
            .clone()
            .unwrap_or_else(|| "Error generating response... Please try again".to_string());
        Ok(content)
    } else {
        Ok("Error generating response... Please try again".to_string())
    }
}

pub async fn raw_llm_processing<T: OpenAIClientInit + Send + Sync>(
    system_role: &str,
    request: &str,
    app_state: Arc<T>,
    model: LlmModel,
) -> Result<String> {
    let llm_client = app_state.get_llm_client().clone();

    let llm_request = CreateChatCompletionRequestArgs::default()
        .max_tokens(4095u32)
        .model(model.as_str())
        .temperature(0.2)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_role)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(request)
                .build()?
                .into(),
        ])
        .build()?;

    let response = llm_client.chat().create(llm_request).await?;

    if let Some(choice) = response.choices.get(0) {
        let content = choice
            .message
            .content
            .clone()
            .unwrap_or_else(|| "Error generating response... Please try again".to_string());
        Ok(content)
    } else {
        Ok("Error generating response... Please try again".to_string())
    }
}

pub async fn process_users_self_description(
    user_id: ChatId,
    user_story_for_profile_creation: String,
    app_state: Arc<RequestAppState>,
) -> Result<()> {
    let pool = &app_state.local_db_pool;

    let fallback_system_role =
        "Return the text provided to you without additional remarks or design.".to_string();
    let system_role = get_system_role_or_fallback(
        &AppName::RequestApp,
        RequestAppSystemRoleType::ProcessingUsersBioText,
        Some(&fallback_system_role),
    );

    let users_about_text_str = raw_llm_processing_json(
        &system_role,
        &user_story_for_profile_creation,
        app_state.clone(),
        LlmModel::Complex,
    )
    .await?;
    info!(
        "TEMP log: LLM processed users self description {}",
        users_about_text_str
    );
    let user_profile: UserProfile = match serde_json::from_str(&users_about_text_str) {
        Ok(profile) => profile,
        Err(err) => {
            eprintln!("Failed to parse user profile from LLM response: {}", err);
            return Err(anyhow::Error::new(err));
        }
    };

    info!("TEMP: Fn: process_users_self_description | Trying to save user_profile to local_db");
    save_user_profile(&pool, user_id.0, &user_profile).await?;

    info!("TEMP: Fn: process_users_self_description | Trying to save user_profile to app_state");
    let mut profiles = app_state.user_profile.lock().await;
    profiles.insert(user_id, user_profile);
    info!("Fn: process_users_self_description | User_profile saved to app_state");

    Ok(())
}

pub async fn process_users_request(
    username: String,
    user_id: ChatId,
    user_request_text: String,
    app_state: Arc<RequestAppState>,
) -> Result<()> {
    let _system_role_to_process_users_request = fs::read_to_string(
        "common_res/system_role_for_processing_users_request_text.txt",
    )
        .with_context(|| "Failed to read system role file from file: system_role_for_processing_users_request_text.txt")?;

    // let user_request_text_by_llm = raw_llm_processing(system_role_to_process_users_request, msg.text().unwrap_or_default().to_string(), app_state.clone()).await?;
    // info!("user_request_text_by_llm: {}", user_request_text_by_llm);

    let mut requests = app_state.user_request.lock().await;
    requests.insert(user_id, user_request_text.clone());

    let collection_name = "outside_app".to_string();

    qdrant_upsert(
        app_state.clone(),
        user_request_text,
        &collection_name,
        user_id,
        username,
    )
    .await
    .context("Failed to upsert data in Qdrant")?;

    Ok(())
}

pub async fn vectorize(data: String, app_state: Arc<RequestAppState>) -> Result<Vec<f32>> {
    let llm_client = app_state.llm_client.clone();

    let request = CreateEmbeddingRequestArgs::default()
        .model(LlmModel::TextEmbedding3Large.as_str())
        .input(data)
        .build()?;

    let response: CreateEmbeddingResponse = llm_client.embeddings().create(request).await?;
    let embedding = response.data.into_iter().next().unwrap().embedding;

    Ok(embedding)
}

pub async fn tokenize_and_truncate(data: &str, max_tokens: usize) -> Result<(String, usize)> {
    let bpe = cl100k_base()?;
    let tokens = bpe.encode_ordinary(&*data);
    let token_count = tokens.len();

    info!("Tokenize_and_truncate fn | Input tokens: {:?}", token_count);

    if token_count > max_tokens {
        let truncated_tokens = tokens[..max_tokens].to_vec();
        let truncated_data = bpe.decode(truncated_tokens)?;
        let truncated_text_tokens = bpe.encode_ordinary(&*truncated_data);
        let truncated_count = truncated_text_tokens.len();

        info!("Truncated input tokens: {:?}", truncated_count);

        Ok((truncated_data, truncated_count))
    } else {
        info!(
            "Input tokens {} < max_tokens, no need to truncate",
            token_count
        );
        Ok((data.to_string(), token_count))
    }
}

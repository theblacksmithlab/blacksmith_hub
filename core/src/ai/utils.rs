use crate::state::llm_processing_trait::LlmProcessing;
use crate::state::request_app::app_state::RequestAppState;
use crate::state::request_app::app_state::UserProfile;
use crate::vector_db::vector_db::qdrant_upsert;
use anyhow::{Context, Result};
use async_openai::types::ResponseFormat::JsonObject;
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs, CreateEmbeddingRequestArgs, CreateEmbeddingResponse,
    CreateSpeechRequestArgs, SpeechModel, Voice,
};
// use std::env;
use chrono::{Duration, Utc};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
// use reqwest::Client as ReqwestClient;
// use serde_json::json;
use crate::local_db::local_db::save_user_profile;
use crate::utils::common::{get_system_role_file_path, read_system_role, LlmModel, SystemRoleType};
use teloxide::prelude::ChatId;
use tracing::info;

pub async fn raw_llm_processing_json(
    system_role: String,
    request: String,
    app_state: Arc<RequestAppState>,
) -> Result<String> {
    let llm_client = app_state.llm_client.clone();

    let llm_request = CreateChatCompletionRequestArgs::default()
        .max_tokens(4095u32)
        .model("gpt-4o")
        .temperature(0.2)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_role.as_str())
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

pub async fn raw_llm_processing<T: LlmProcessing + Send + Sync>(
    system_role: String,
    request: String,
    app_state: Arc<T>,
    model: LlmModel,
) -> Result<String> {
    let llm_client = app_state.get_llm_client().clone();

    let llm_request = CreateChatCompletionRequestArgs::default()
        .max_tokens(4045u32)
        .model(model.as_str())
        .temperature(0.2)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_role.as_str())
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

pub async fn text_to_speech<T: LlmProcessing + Send + Sync>(
    text: String,
    user_tmp_dir: String,
    app_state: Arc<T>,
) -> Result<PathBuf> {
    info!("Starting recording podcast...");

    let now = Utc::now();
    let utc_plus_3 = now + Duration::hours(3);
    let date_only = utc_plus_3.date_naive();

    let llm_client = app_state.get_llm_client().clone();

    let request = CreateSpeechRequestArgs::default()
        .input(&text)
        .voice(Voice::Onyx)
        .model(SpeechModel::Tts1Hd)
        .speed(1.3)
        .build()?;

    let response = llm_client.audio().speech(request).await?;

    let file_name = format!("The_Viper_Podcast_({})", date_only);

    let audio_file_path = format!("{}/{}.mp3", user_tmp_dir, file_name);
    response.save(audio_file_path.clone()).await?;

    info!("fn: text_to_speech | Podcast is ready");

    Ok(PathBuf::from(audio_file_path))
}

// async fn generate_speech(text: &str, api_key: &str) -> Result<Vec<u8>> {
//     let client = ReqwestClient::new();
//     let voice_id = "nPczCjzI2devNBz1zQrb";
//
//     let response = client
//         .post(format!(
//             "https://api.elevenlabs.io/v1/text-to-speech/{}/stream",
//             voice_id
//         ))
//         .header("xi-api-key", api_key)
//         .header("Content-Type", "application/json")
//         .json(&json!({
//             "text": text,
//             "model_id": "eleven_multilingual_v2",
//             "voice_settings": {
//                 "stability": 0.9,
//                 "similarity_boost": 0.65,
//                 "speed": 1.3
//             }
//         }))
//         .send()
//         .await?
//         .bytes()
//         .await?
//         .to_vec();
//
//     Ok(response)
// }
//
// pub(crate) async fn text_to_speech_11_labs<T: LlmProcessing + Send + Sync>(
//     text: String,
//     user_tmp_dir: String,
//     app_state: Arc<T>
// ) -> Result<PathBuf> {
//     info!("Starting recording podcast...");
//
//     let api_key = env::var("ELEVEN_LABS_API_TOKEN")
//         .map_err(|_| anyhow::anyhow!("ELEVEN_LABS_API_TOKEN not found in environment"))?;
//
//     let now = Utc::now();
//     let utc_plus_3 = now + Duration::hours(3);
//     let date_only = utc_plus_3.date_naive();
//
//     let podcast_number = get_podcast_counter().await?;
//     let file_name = format!("The_Viper_podcast_#{}_{}",
//                             podcast_number,
//                             date_only
//     );
//
//     let audio_file_path = format!("{}/{}.mp3", user_tmp_dir, file_name);
//
//     let audio_data = generate_speech(&text, &api_key).await?;
//
//     fs::write(&audio_file_path, audio_data)?;
//
//     info!("fn: text_to_speech | Podcast is ready");
//
//     Ok(PathBuf::from(audio_file_path))
// }

pub async fn process_users_self_description(
    user_id: ChatId,
    user_story_for_profile_creation: String,
    app_state: Arc<RequestAppState>,
) -> Result<()> {
    let pool = &app_state.local_db_pool;

    let role_type = SystemRoleType::ProcessingUserStoryForProfile;
    let file_path = get_system_role_file_path(role_type);

    let system_role = read_system_role(file_path).unwrap_or_else(|err| {
        eprintln!("Failed to load system role: {}", err);
        "Return the text provided to you without additional remarks or design.".to_string()
    });

    let users_about_text_str = raw_llm_processing_json(
        system_role,
        user_story_for_profile_creation,
        app_state.clone(),
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
        .model("text-embedding-3-large")
        .input(data)
        .build()?;

    let response: CreateEmbeddingResponse = llm_client.embeddings().create(request).await?;
    let embedding = response.data.into_iter().next().unwrap().embedding;

    Ok(embedding)
}

use crate::ai::common::voice_processing::speech_to_text;
use crate::models::common::app_name::AppName;
use crate::models::common::system_messages::AppsSystemMessages;
use crate::models::request_app::request_app::{AvatarRequest, AvatarResponse};
use crate::state::request_app::app_state::{RequestAppState, UserProfile, UserStates};
use crate::state::the_viper_room::app_state::{AuthStages, TheViperRoomAppState, UserData};
use crate::vector_db::vector_db::restore_request_from_qdrant;
use anyhow::{anyhow, Result};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use pulldown_cmark::{html, Parser};
use std::env;
use std::fs::{read_to_string, remove_file};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use teloxide::prelude::ChatId;
use tracing::{error, info};

pub fn get_mapped_from_app_name_role_directory(app_name: &AppName) -> &str {
    match app_name {
        AppName::ProbiotBot => "probiot",
        AppName::W3ABot => "w3a",
        AppName::W3AWeb => "w3a",
        AppName::RequestAppBot => "request_app",
        AppName::TheViperRoomBot => "the_viper_room",
        _ => app_name.as_str(),
    }
}

pub fn get_system_role_path(app_name: &AppName, role_type: &str) -> String {
    format!(
        "common_res/system_roles/{}/{}.txt",
        get_mapped_from_app_name_role_directory(app_name),
        role_type
    )
}

pub fn get_system_role_or_fallback<T>(
    app_name: &AppName,
    role_type: T,
    fallback: Option<&str>,
) -> String
where
    T: AsRef<str>,
{
    let role_str = role_type.as_ref();

    let file_path = get_system_role_path(app_name, role_str);

    match read_to_string(&file_path) {
        Ok(content) => content,
        Err(err) => {
            error!(
                "Failed to load system role '{}': {}. Using fallback.",
                file_path, err
            );

            error!(
                "Invalid role '{}' used for application '{}'. This role does not exist.",
                role_str,
                app_name.as_str()
            );

            fallback
                .unwrap_or("You are a helpful assistant")
                .to_string()
        }
    }
}

pub enum LlmModel {
    Light,               // OpenAI gpt-4o-mini
    Complex,             // OpenAI gpt-4o
    TextEmbedding3Large, // OpenAI embedding generative model
}

impl LlmModel {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            LlmModel::Light => "gpt-4o-mini",
            LlmModel::Complex => "gpt-4o",
            LlmModel::TextEmbedding3Large => "text-embedding-3-large",
        }
    }
}

pub async fn extract_user_profile_from_app_state(
    app_state: &Arc<RequestAppState>,
    chat_id: ChatId,
) -> Option<UserProfile> {
    let profiles = app_state.user_profile.lock().await;
    profiles.get(&chat_id).cloned()
}

async fn extract_user_request_from_app_state(
    app_state: &Arc<RequestAppState>,
    chat_id: ChatId,
) -> Option<String> {
    let requests = app_state.user_request.lock().await;
    requests.get(&chat_id).cloned()
}

pub async fn update_request_app_user_state<F>(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    update_fn: F,
) where
    F: FnOnce(&mut UserStates),
{
    let mut user_states = app_state.user_states.lock().await;
    let state = user_states
        .entry(user_id)
        .or_insert_with(UserStates::default);
    update_fn(state);
}

pub fn format_user_profile(user_profile: &UserProfile) -> String {
    let first_name = user_profile
        .registration_info
        .first_name
        .as_deref()
        .unwrap_or("Не указано");
    let last_name = user_profile
        .registration_info
        .last_name
        .as_deref()
        .unwrap_or("Не указано");
    let age = user_profile
        .registration_info
        .age
        .map_or("Не указано".to_string(), |a| a.to_string());
    let gender = user_profile
        .registration_info
        .gender
        .as_deref()
        .unwrap_or("Не указано");
    let city = user_profile
        .registration_info
        .city_of_residence
        .as_deref()
        .unwrap_or("Не указано");
    let interests = user_profile
        .additional_info
        .interests
        .as_ref()
        .map(|ints| {
            if ints.is_empty() {
                "Не указаны".to_string()
            } else {
                ints.join(", ")
            }
        })
        .unwrap_or("Не указаны".to_string());

    format!(
        "Имя: {}\nФамилия: {}\nВозраст: {}\nПол: {}\nГород: {}\nИнтересы: {}",
        first_name, last_name, age, gender, city, interests
    )
}

pub async fn determine_user_request(
    user_id: ChatId,
    app_state: Arc<RequestAppState>,
) -> Result<Option<String>> {
    if let Some(request) = extract_user_request_from_app_state(&app_state, user_id).await {
        return Ok(Some(request));
    }

    match restore_request_from_qdrant(app_state.clone(), 1, user_id).await {
        Ok(Some(restored_request)) => {
            app_state
                .user_request
                .lock()
                .await
                .insert(user_id, restored_request.clone());
            Ok(Some(restored_request))
        }
        Ok(None) => Ok(None),
        Err(_) => Err(anyhow::anyhow!(
            "Error restoring user's request from qdrant",
        )),
    }
}

pub async fn update_the_viper_room_user_state<F>(
    the_viper_room_app_state: Arc<TheViperRoomAppState>,
    user_id: u64,
    update_fn: F,
) where
    F: FnOnce(&mut AuthStages),
{
    let mut user_states = the_viper_room_app_state.user_state.lock().await;
    let state = user_states
        .entry(user_id)
        .or_insert_with(AuthStages::default);
    update_fn(state);
}

pub async fn update_the_viper_room_user_data<F>(
    the_viper_room_app_state: Arc<TheViperRoomAppState>,
    user_id: u64,
    update_fn: F,
) where
    F: FnOnce(&mut UserData),
{
    let mut user_data = the_viper_room_app_state.user_data.lock().await;
    let data = user_data.entry(user_id).or_insert_with(UserData::default);
    update_fn(data);
}

pub async fn get_message(message_enum: AppsSystemMessages) -> Result<String> {
    const DEFAULT_FALLBACK_MESSAGE: &str =
        "Извините, произошла техническая ошибка. Пожалуйста, попробуйте позже.";

    let (base_path, message_name): (PathBuf, String) = match message_enum {
        AppsSystemMessages::Common(msg) => (
            Path::new("common_res/messages/common").to_path_buf(),
            msg.as_str().to_string(),
        ),
        AppsSystemMessages::Probiot(msg) => (
            Path::new("common_res/messages/probiot_bot").to_path_buf(),
            msg.as_str().to_string(),
        ),
        AppsSystemMessages::TheViperRoom(msg) => (
            Path::new("common_res/messages/the_viper_room").to_path_buf(),
            msg.as_str().to_string(),
        ),
        AppsSystemMessages::TheViperRoomBot(msg) => (
            Path::new("common_res/messages/the_viper_room_bot").to_path_buf(),
            msg.as_str().to_string(),
        ),
        AppsSystemMessages::RequestApp(msg) => (
            Path::new("common_res/messages/request_app").to_path_buf(),
            msg.as_str().to_string(),
        ),
        AppsSystemMessages::RequestAppBot(msg) => (
            Path::new("common_res/messages/request_app_bot").to_path_buf(),
            msg.as_str().to_string(),
        ),
        AppsSystemMessages::W3ABot(msg) => (
            Path::new("common_res/messages/w3a_bot").to_path_buf(),
            msg.as_str().to_string(),
        ),
    };

    let path = base_path.join(format!("{}.txt", message_name));

    if !path.exists() {
        error!("Message file not found: {}", path.display());
        return Ok(DEFAULT_FALLBACK_MESSAGE.to_string());
    }

    read_to_string(&path)
        .map_err(|e| {
            error!("Failed to read message file {}: {}", path.display(), e);
            anyhow!(
                "Failed to read message '{}' {}: {}",
                message_name,
                path.display(),
                e
            )
        })
        .or_else(|_| Ok(DEFAULT_FALLBACK_MESSAGE.to_string()))
}

pub async fn get_user_avatar(
    Query(params): Query<AvatarRequest>,
) -> Result<Json<AvatarResponse>, StatusCode> {
    let user_id = params.user_id.to_string();

    let bot_token = env::var("TELOXIDE_TOKEN_THE_VIPER_ROOM")
        .expect("TELOXIDE_TOKEN_THE_VIPER_ROOM must be set in the environment");

    let url = format!(
        "https://api.telegram.org/bot{}/getUserProfilePhotos?user_id={}",
        bot_token, user_id
    );

    let response = reqwest::get(&url)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let photos: serde_json::Value = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(photo_array) = photos["result"]["photos"].as_array() {
        if !photo_array.is_empty() {
            if let Some(file_id) = photo_array[0][0]["file_id"].as_str() {
                let file_url = format!(
                    "https://api.telegram.org/bot{}/getFile?file_id={}",
                    bot_token, file_id
                );
                let file_response = reqwest::get(&file_url)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                let file_data: serde_json::Value = file_response
                    .json()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                if let Some(file_path) = file_data["result"]["file_path"].as_str() {
                    let avatar_url = format!(
                        "https://api.telegram.org/file/bot{}/{}",
                        bot_token, file_path
                    );
                    return Ok(Json(AvatarResponse {
                        avatar_url: Some(avatar_url),
                    }));
                }
            }
        }
    }

    Ok(Json(AvatarResponse { avatar_url: None }))
}

pub fn split_text_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_pos = 0;
    let text_len = text.len();

    while current_pos < text_len {
        let max_end_pos = std::cmp::min(current_pos + max_chars, text_len);

        let substring = &text[current_pos..max_end_pos];
        let end_pos =
            if let Some(period_pos) = substring.rfind(|c| c == '.' || c == '!' || c == '?') {
                current_pos + period_pos + 1
            } else {
                max_end_pos
            };

        chunks.push(text[current_pos..end_pos].to_string());
        current_pos = end_pos;
    }

    chunks
}

pub fn convert_to_wav(file_path: &str) -> Result<String, anyhow::Error> {
    let mut path = std::path::PathBuf::from(file_path);
    path.set_extension("wav");

    let wav_path = path.to_str().unwrap();

    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(file_path)
        .arg("-ar")
        .arg("16000")
        .arg(&wav_path)
        .output();

    match output {
        Ok(output) if output.status.success() => Ok(wav_path.to_string()),
        Ok(output) => Err(anyhow::anyhow!(
            "FFmpeg conversion failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )),
        Err(err) => Err(anyhow::anyhow!("Failed to execute FFmpeg: {}", err)),
    }
}

pub fn check_whisper_installed() -> Result<(), anyhow::Error> {
    let output = Command::new("whisper-cli").arg("--help").output();

    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(anyhow::anyhow!(
            "Whisper CLI failed to respond correctly: {}",
            String::from_utf8_lossy(&output.stderr)
        )),
        Err(err) => Err(anyhow::anyhow!("Whisper CLI not found: {}", err)),
    }
}

pub async fn transcribe_voice_message(file_path: &str) -> Result<Option<String>> {
    check_whisper_installed()?;

    let wav_path = convert_to_wav(file_path)?;

    let transcription = speech_to_text(&wav_path).await?;

    remove_file(file_path).ok();
    remove_file(&wav_path).ok();

    if transcription.trim().is_empty() {
        info!("Voice message transcription is empty, looks like user sent message by mistake");
        Ok(None)
    } else {
        Ok(Some(transcription))
    }
}

pub fn markdown_to_html(markdown: &str) -> String {
    let parser = Parser::new(markdown);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

pub fn convert_markdown_to_telegram(markdown: &str) -> String {
    markdown
        .replace("_", "\\_") // Экранируем подчеркивания
        .replace("*", "\\*") // Экранируем звездочки
        .replace("[", "\\[") // Экранируем квадратные скобки
        .replace("]", "\\]")
        .replace("(", "\\(")
        .replace(")", "\\)")
        .replace("~", "\\~") // Экранируем тильду
        .replace("`", "\\`") // Экранируем бектик
        .replace(">", "\\>") // Экранируем ">"
        .replace("#", "\\#") // Экранируем #
        .replace("+", "\\+") // Экранируем +
        .replace("-", "\\-") // Экранируем -
        .replace("=", "\\=") // Экранируем =
        .replace("|", "\\|") // Экранируем |
        .replace("{", "\\{") // Экранируем {
        .replace("}", "\\}") // Экранируем }
        .replace(".", "\\.") // Экранируем .
        .replace("!", "\\!") // Экранируем !
}


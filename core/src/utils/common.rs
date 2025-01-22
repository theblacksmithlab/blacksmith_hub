use crate::models::request_app::request_app::{AvatarRequest, AvatarResponse};
use crate::state::request_app::app_state::{RequestAppState, UserProfile, UserStates};
use crate::state::the_viper_room::app_state::{AuthStages, TheViperRoomAppState, UserData};
use crate::vector_db::vector_db::restore_request_from_qdrant;
use anyhow::Result;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use std::env;
use std::fs::read_to_string;
use std::path::Path;
use std::sync::Arc;
use teloxide::prelude::ChatId;
use tracing::error;

pub enum SystemRoleType {
    ProcessingUserStoryForProfile,
    // ProcessingUserStoryForRequest,
    ReorderingResults,
}

pub fn get_system_role_file_path(role_type: SystemRoleType) -> &'static str {
    match role_type {
        SystemRoleType::ProcessingUserStoryForProfile => {
            "common_res/system_role_for_processing_users_about_text.txt"
        }
        // SystemRoleType::ProcessingUserStoryForRequest => "common_res/system_role_for_processing_users_request_text.txt",
        SystemRoleType::ReorderingResults => "common_res/system_role_for_reordering_via_llm.txt",
    }
}

pub fn read_system_role(file_path: &str) -> Result<String, String> {
    read_to_string(file_path).map_err(|e| format!("Failed to read '{}': {}", file_path, e))
}

pub enum LlmModel {
    Light,   // gpt-4o-mini
    Complex, // gpt-4o
}

impl LlmModel {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            LlmModel::Light => "gpt-4o-mini",
            LlmModel::Complex => "gpt-4o",
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

pub async fn get_message(app_name: &str, message_name: &str) -> Result<String> {
    let path = Path::new("common_res/messages")
        .join(app_name)
        .join(format!("{}.txt", message_name));

    if !path.exists() {
        error!("Message file not found: {}", path.display());
        return Err(anyhow::anyhow!(
            "Message file '{}' for app '{}' does not exist at path: {}",
            message_name,
            app_name,
            path.display()
        ));
    }

    read_to_string(&path).map_err(|e| {
        error!("Failed to read message file {}: {}", path.display(), e);
        anyhow::anyhow!(
            "Failed to read message '{}' for app '{}': {}",
            message_name,
            app_name,
            e
        )
    })
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

use crate::ai::common::voice_processing::speech_to_text;
use crate::models::common::app_name::AppName;
use crate::models::common::avatar_request_response::{AvatarRequest, AvatarResponse};
use crate::models::common::system_messages::AppsSystemMessages;
use crate::state::the_viper_room::app_state::{AuthStages, TheViperRoomAppState, UserData};
use anyhow::{anyhow, Result};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use pulldown_cmark::{html, Event, Parser, Tag, TagEnd};
use std::env;
use std::fs::{read_to_string, remove_file};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info};

pub fn build_resource_file_path(app_name: &AppName, file_name: &str) -> PathBuf {
    PathBuf::from("common_res")
        .join(get_mapped_from_app_name_role_directory(app_name))
        .join(file_name)
}

pub fn get_mapped_from_app_name_role_directory(app_name: &AppName) -> &str {
    match app_name {
        AppName::ProbiotBot => "probiot",
        AppName::W3AWeb => "w3a",
        AppName::TheViperRoomBot => "the_viper_room",
        AppName::BlacksmithWeb => "blacksmith_lab",
        AppName::GrootBot => "groot_bot",
        _ => app_name.as_str(),
    }
}

pub fn get_system_role_path(app_name: &AppName, role_type: &str) -> PathBuf {
    PathBuf::from("common_res")
        .join("system_roles")
        .join(get_mapped_from_app_name_role_directory(app_name))
        .join(format!("{}.txt", role_type))
}

pub fn get_system_role<T>(app_name: &AppName, role_type: T) -> Result<String>
where
    T: AsRef<str>,
{
    let role_str = role_type.as_ref();
    let file_path = get_system_role_path(app_name, role_str);

    read_to_string(&file_path).map_err(|err| {
        anyhow!(
            "System role '{}' not found for app '{}': {}",
            role_str,
            app_name.as_str(),
            err
        )
    })
}

/// Deprecated: use `get_system_role` instead
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
                file_path.display(),
                err
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
        AppsSystemMessages::ProbiotBot(msg) => (
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
        AppsSystemMessages::GrootBot(msg) => (
            Path::new("common_res/messages/groot_bot").to_path_buf(),
            msg.as_str().to_string(),
        ),
        AppsSystemMessages::W3A(msg) => (
            Path::new("common_res/w3a").to_path_buf(),
            msg.as_str().to_string(),
        ),
        AppsSystemMessages::AgentDavon(msg) => (
            Path::new("common_res/messages/agent_davon").to_path_buf(),
            msg.as_str().to_string(),
        ),
        AppsSystemMessages::StatBot(msg) => (
            Path::new("common_res/messages/stat_bot").to_path_buf(),
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

    let bot_token = env::var("TELOXIDE_TOKEN_THE_VIPER_ROOM_BOT")
        .expect("TELOXIDE_TOKEN_THE_VIPER_ROOM_BOT must be set in the environment");

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
    let mut current_chunk = String::new();
    let mut char_count = 0;
    let mut last_boundary = None;
    let search_range = 200;

    for c in text.chars() {
        current_chunk.push(c);
        char_count += 1;

        if c == '.' || c == '!' || c == '?' {
            last_boundary = Some(char_count);
        }

        if char_count >= max_chars {
            if let Some(boundary) = last_boundary {
                if char_count - boundary <= search_range {
                    let valid_chunk: String = current_chunk.chars().take(boundary).collect();
                    chunks.push(valid_chunk);

                    current_chunk = current_chunk.chars().skip(boundary).collect();
                    char_count = current_chunk.chars().count();
                    last_boundary = None;
                    continue;
                }
            }

            let valid_chunk: String = current_chunk.chars().take(max_chars).collect();
            chunks.push(valid_chunk);

            current_chunk = current_chunk.chars().skip(max_chars).collect();
            char_count = current_chunk.chars().count();
            last_boundary = None;
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}

pub async fn transcribe_voice_message(file_path: &Path) -> Result<Option<String>> {
    let transcription = speech_to_text(file_path).await?;

    remove_file(file_path).ok();
    info!("Successfully removed temp file: {:?}", file_path);

    if transcription.trim().is_empty() {
        info!(
            "Voice message transcription is empty, looks like user sent empty message by mistake"
        );
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

pub fn markdown_to_telegram_html(markdown: &str) -> String {
    let parser = Parser::new(markdown);
    let mut html_output = String::new();
    let mut in_code_block = false;
    let mut code_block_content = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    // ignore
                }
                Tag::Heading { .. } => {
                    html_output.push_str("<b>");
                }
                Tag::Strong => {
                    html_output.push_str("<b>");
                }
                Tag::Emphasis => {
                    html_output.push_str("<i>");
                }
                Tag::Strikethrough => {
                    html_output.push_str("<s>");
                }
                Tag::Link { dest_url, .. } => {
                    html_output.push_str(&format!("<a href=\"{}\">", escape_html(&dest_url)));
                }
                Tag::CodeBlock(_) => {
                    in_code_block = true;
                    code_block_content.clear();
                }
                Tag::List(_) => {
                    // ignore
                }
                Tag::Item => {
                    html_output.push_str("• ");
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => {
                    html_output.push_str("\n\n");
                }
                TagEnd::Heading { .. } => {
                    html_output.push_str("</b>\n\n");
                }
                TagEnd::Strong => {
                    html_output.push_str("</b>");
                }
                TagEnd::Emphasis => {
                    html_output.push_str("</i>");
                }
                TagEnd::Strikethrough => {
                    html_output.push_str("</s>");
                }
                TagEnd::Link => {
                    html_output.push_str("</a>");
                }
                TagEnd::CodeBlock => {
                    html_output
                        .push_str(&format!("<pre>{}</pre>", escape_html(&code_block_content)));
                    in_code_block = false;
                }
                TagEnd::Item => {
                    html_output.push_str("\n");
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    code_block_content.push_str(&text);
                } else {
                    html_output.push_str(&escape_html(&text));
                }
            }
            Event::Code(code) => {
                html_output.push_str(&format!("<code>{}</code>", escape_html(&code)));
            }
            Event::SoftBreak => {
                html_output.push(' ');
            }
            Event::HardBreak => {
                html_output.push('\n');
            }
            _ => {}
        }
    }

    html_output.trim().to_string()
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// legacy method
pub fn convert_markdown_to_telegram(markdown: &str) -> String {
    markdown
        .replace("\\", "\\\\")
        .replace("[", "\\[")
        .replace("]", "\\]")
        .replace("(", "\\(")
        .replace(")", "\\)")
        .replace("~", "\\~")
        .replace("`", "\\`")
        .replace(">", "\\>")
        .replace("#", "\\#")
        .replace("+", "\\+")
        .replace("-", "\\-")
        .replace("=", "\\=")
        .replace("|", "\\|")
        .replace("{", "\\{")
        .replace("}", "\\}")
        .replace(".", "\\.")
        .replace("!", "\\!")
}

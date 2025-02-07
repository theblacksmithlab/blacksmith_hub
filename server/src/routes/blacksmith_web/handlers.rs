use anyhow::Result;
use axum::extract::{Query, State};
use axum::Json;
use base64::{engine::general_purpose::STANDARD, Engine};
use core::ai::common::voice_processing::simple_tts;
use core::local_db::local_db::fetch_chat_history_from_db;
use core::message_processing_flow::web::default_message_handler::default_message_handler;
use core::models::blacksmith_web::blacksmith_web::ChatMessage;
use core::models::blacksmith_web::blacksmith_web::{
    BlacksmithWebServerResponse, BlacksmithWebTTSRequest, BlacksmithWebTTSResponse,
    BlacksmithWebUserRequest,
};
use core::models::common::app_name::AppName;
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tracing::log::info;
use tracing::warn;
use uuid::Uuid;

pub(crate) async fn handle_blacksmith_web_user_request(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Json(request): Json<BlacksmithWebUserRequest>,
) -> Json<BlacksmithWebServerResponse> {
    let app_name = match AppName::from_str(&request.app_name) {
        Ok(app) => app,
        Err(_) => {
            warn!("Unsupported app type: {}", request.app_name);
            AppName::BlacksmithWeb
        }
    };

    let user_id = request.user_id;
    let action_text = request.text;

    info!("Got message: {} from user: {}", action_text, user_id);

    let response =
        default_message_handler(&action_text, blacksmith_web_app_state, &user_id, app_name).await;

    Json(BlacksmithWebServerResponse { text: response })
}

pub(crate) async fn handle_blacksmith_web_chat_fetch(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<ChatMessage>> {
    info!("Fetching chat history for web application");
    let user_id = match params.get("user_id") {
        Some(id) => id.clone(),
        None => return Json(vec![]),
    };

    let app_name = match params.get("app_name") {
        Some(name) => match AppName::from_str(name) {
            Ok(app) => app,
            Err(_) => return Json(vec![]),
        },
        None => return Json(vec![]),
    };

    info!(
        "Fetching history for user_id={} with app_name={}",
        user_id, app_name
    );
    match fetch_chat_history_from_db(
        &blacksmith_web_app_state.local_db_pool,
        &user_id,
        app_name.as_str(),
    )
    .await
    {
        Ok(chat_history) => Json(chat_history),
        Err(_) => Json(vec![]),
    }
}

pub(crate) async fn handle_blacksmith_web_tts_request(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Json(request): Json<BlacksmithWebTTSRequest>,
) -> Json<BlacksmithWebTTSResponse> {
    let user_id = request.user_id;
    let request_text = request.text;

    // TODO: prepare action_text for TTS

    let app_name = match AppName::from_str(&request.app_name) {
        Ok(app) => app,
        Err(_) => {
            warn!("Unsupported app type: {}", request.app_name);
            AppName::BlacksmithWeb
        }
    };

    let temp_dir = app_name.temp_dir();
    info!("TEMP log: temp dir: {:?}", temp_dir);

    info!(
        "Got TTS request for text: {} from user: {}",
        request_text, user_id
    );

    match simple_tts(&request_text, blacksmith_web_app_state.clone()).await {
        Ok(audio_response) => {
            let temp_file_id = Uuid::new_v4().to_string();
            let audio_file_path = temp_dir.join(format!("{}.mp3", temp_file_id));

            if let Err(e) = audio_response.save(&audio_file_path).await {
                warn!("Failed to save audio file: {}", e);
                return Json(BlacksmithWebTTSResponse {
                    audio_data: String::new(),
                });
            }

            match read_audio_file_as_base64(&audio_file_path) {
                Ok(audio_data) => {
                    if let Err(e) = fs::remove_file(&audio_file_path) {
                        warn!("Failed to delete temp file {:?}: {}", audio_file_path, e);
                    }

                    info!("TEMP log: input text transcribed successfully");

                    Json(BlacksmithWebTTSResponse { audio_data })
                }
                Err(e) => {
                    warn!("Failed to read audio file: {}", e);
                    Json(BlacksmithWebTTSResponse {
                        audio_data: String::new(),
                    })
                }
            }
        }
        Err(e) => {
            warn!("TTS generation failed: {}", e);
            Json(BlacksmithWebTTSResponse {
                audio_data: String::new(),
            })
        }
    }
}

fn read_audio_file_as_base64(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(STANDARD.encode(&buffer))
}

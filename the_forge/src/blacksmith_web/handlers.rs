use anyhow::Result;
use axum::extract::{Query, State};
use axum::Json;
use base64::{engine::general_purpose::STANDARD, Engine};
use core::ai::common::common::raw_llm_processing;
use core::ai::common::voice_processing::openai_base_tts;
use core::local_db::local_db::fetch_chat_history_from_db;
use core::message_processing_flow::web::default_message_handler::default_message_handler;
use core::models::blacksmith_web::blacksmith_web::ChatMessage;
use core::models::blacksmith_web::blacksmith_web::{
    BlacksmithWebServerResponse, BlacksmithWebTTSRequest, BlacksmithWebTTSResponse,
    BlacksmithWebUserRequest,
};
use core::models::common::ai::LlmModel;
use core::models::common::app_name::AppName;
use core::models::common::system_roles::{AppsSystemRoles, BlacksmithLabRoleType, W3ARoleType};
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;
use core::utils::common::get_system_role_or_fallback;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tracing::log::info;
use tracing::{error, warn};
use uuid::Uuid;

pub(crate) async fn handle_blacksmith_web_user_request(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Json(request): Json<BlacksmithWebUserRequest>,
) -> Json<BlacksmithWebServerResponse> {
    let app_name = match AppName::from_str(&request.app_name) {
        Ok(app) => app,
        Err(_) => {
            warn!("Unsupported app_name received from frontend: {}. Processing request as Blacksmith Web", request.app_name);
            AppName::BlacksmithWeb
        }
    };

    let user_id = request.user_id;
    let request_text = request.text;

    info!(
        "App-Source: {} | Got text message from user: {}: {}",
        app_name, user_id, request_text
    );

    let (response, extra_data) =
        default_message_handler(&request_text, blacksmith_web_app_state, &user_id, &app_name).await;

    info!(
        "\n==============================================================================\n\n\
    User's request: {}\n\n\
    ------------------------------\n\n\
    System's response: {}\n\n\
    ==============================================================================\n",
        request_text, response
    );

    // NEW: extra_data is now HashMap<String, String> directly from RAG system
    let extra_data_map = extra_data;

    Json(BlacksmithWebServerResponse {
        text: response,
        extra_data_parsed: extra_data_map,
    })
}

pub(crate) async fn handle_blacksmith_web_chat_fetch(
    State(blacksmith_web_app_state): State<Arc<BlacksmithWebAppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<ChatMessage>> {
    let app_name = match params.get("app_name") {
        Some(name) => match AppName::from_str(name) {
            Ok(app) => app,
            Err(_) => return Json(vec![]),
        },
        None => return Json(vec![]),
    };

    let user_id = match params.get("user_id") {
        Some(id) => id.clone(),
        None => return Json(vec![]),
    };

    match fetch_chat_history_from_db(
        &blacksmith_web_app_state.local_db_pool,
        &user_id,
        app_name.as_str(),
        Some(20), // Лимит в 20 последних сообщений
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
    let app_name = match AppName::from_str(&request.app_name) {
        Ok(app) => app,
        Err(_) => {
            warn!("Unsupported app type: {}", request.app_name);
            AppName::BlacksmithWeb
        }
    };

    let user_id = request.user_id;
    let request_text = request.text;

    let temp_dir = app_name.temp_dir();

    info!(
        "App-Source: {} | Got TTS request from user: {} for text: {}",
        app_name, user_id, request_text
    );

    let processed_text =
        match prepare_text_for_tts_fn(&app_name, blacksmith_web_app_state.clone(), &request_text)
            .await
        {
            Ok(clean_text) => clean_text,
            Err(err) => {
                warn!(
                    "Failed to pre-process text for TTS: {}. Using original text.",
                    err
                );
                request_text.clone()
            }
        };

    match openai_base_tts(&processed_text, blacksmith_web_app_state.clone(), 1.3).await {
        Ok(audio_response) => {
            let temp_file_id = Uuid::new_v4().to_string();
            let audio_file_path = temp_dir.join(format!("{}.mp3", temp_file_id));

            if let Err(e) = audio_response.save(&audio_file_path).await {
                error!("Failed to save audio file: {}", e);
                return Json(BlacksmithWebTTSResponse {
                    audio_data: String::new(),
                });
            }

            match read_audio_file_as_base64(&audio_file_path) {
                Ok(audio_data) => {
                    if let Err(e) = fs::remove_file(&audio_file_path) {
                        error!(
                            "Failed to delete TTS temp file {:?}: {}",
                            audio_file_path, e
                        );
                    }

                    info!("TTS request processed successfully!");

                    Json(BlacksmithWebTTSResponse { audio_data })
                }
                Err(e) => {
                    error!("Failed to read audio file with TTS result: {}", e);
                    Json(BlacksmithWebTTSResponse {
                        audio_data: String::new(),
                    })
                }
            }
        }
        Err(e) => {
            error!("TTS generation failed: {}", e);
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

async fn prepare_text_for_tts_fn(
    app_name: &AppName,
    blacksmith_web_app_state: Arc<BlacksmithWebAppState>,
    text_to_process: &str,
) -> Result<String> {
    let system_role = match app_name {
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::TTSPreProcessing)),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::TTSPreProcessing,
        )),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "TTSPreProcessing role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    let llm_message = format!("Text to process: {}", text_to_process);

    let processed_text = raw_llm_processing(
        &system_role,
        &llm_message,
        blacksmith_web_app_state,
        LlmModel::Light,
    )
    .await?;

    Ok(processed_text)
}

// REMOVED: load_lesson_urls() - no longer needed, lesson URLs come directly from Qdrant metadata

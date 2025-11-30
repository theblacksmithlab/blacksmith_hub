use anyhow::Result;
use axum::extract::{Query, State};
use axum::Json;
use base64::{engine::general_purpose::STANDARD, Engine};
use core::ai::common::common::raw_llm_processing;
use core::ai::common::voice_processing::simple_tts;
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
use core::utils::common::{build_resource_file_path, get_system_role_or_fallback};
use serde_json::Value;
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

    let extra_data_map = if matches!(app_name, AppName::W3AWeb) && !extra_data.is_empty() {
        let all_lesson_urls = load_lesson_urls(&app_name).await;

        let mut data_map = HashMap::new();
        for item in extra_data {
            let item_lower = item.to_lowercase();
            if let Some(url) = all_lesson_urls.get(&item_lower) {
                data_map.insert(item, url.clone());
            } else {
                warn!("URL for item '{}' not found in database", item);
            }
        }
        data_map
    } else {
        HashMap::new()
    };

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

    // info!(
    //     "App-Source: {} | Fetching chat history for user: {}",
    //     app_name, user_id
    // );

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

    match simple_tts(&processed_text, blacksmith_web_app_state.clone()).await {
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

pub async fn load_lesson_urls(app_name: &AppName) -> HashMap<String, String> {
    let file_path = build_resource_file_path(app_name, "learning_structure_with_urls.json");
    let file_content = match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => content,
        Err(err) => {
            error!(
                "Failed to read lesson URLs file at {:?}: {}",
                file_path, err
            );
            return HashMap::new();
        }
    };

    let structure: Value = match serde_json::from_str(&file_content) {
        Ok(value) => value,
        Err(err) => {
            error!("Failed to parse learning structure urls JSON: {}", err);
            return HashMap::new();
        }
    };

    let mut lesson_urls = HashMap::new();

    if let Some(categories) = structure.as_object() {
        for (_, category_value) in categories {
            if let Some(category) = category_value.as_object() {
                for (_, subcategory_value) in category {
                    if let Some(lessons) = subcategory_value.as_object() {
                        for (lesson_title, url_value) in lessons {
                            if let Some(url) = url_value.as_str() {
                                lesson_urls.insert(lesson_title.to_lowercase(), url.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    lesson_urls
}

use crate::ai::common::openai::raw_openai_processing_json;
use crate::models::common::ai::OpenAIModel;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::{AppsSystemRoles, UniframeStudioRoleType};
use crate::state::uniframe_studio::app_state::UniframeStudioAppState;
use crate::utils::common::get_system_role_or_fallback;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, warn};

pub async fn validate_transcription_keywords(
    raw_transcription_keywords: String,
    app_state: Arc<UniframeStudioAppState>,
) -> String {
    let app_name = AppName::UniframeStudio;
    let system_role = Some(AppsSystemRoles::UniframeStudio(
        UniframeStudioRoleType::ValidateKeywords,
    ));

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "MainProcessing role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    let llm_message = format!("Raw transcription keywords: {}", raw_transcription_keywords);

    let validated_keywords =
        match raw_openai_processing_json(&system_role, &llm_message, app_state, OpenAIModel::GPT4o)
            .await
        {
            Ok(response) => response,
            Err(e) => {
                warn!("LLM processing failed, using original keywords: {}", e);
                return raw_transcription_keywords;
            }
        };

    let json_response: Value = match serde_json::from_str(&validated_keywords) {
        Ok(json) => json,
        Err(e) => {
            warn!(
                "Failed to parse LLM response as JSON, using original keywords: {}",
                e
            );
            return raw_transcription_keywords;
        }
    };

    let keywords_str = match json_response
        .get("transcription_keywords")
        .and_then(|v| v.as_str())
    {
        Some(keywords) => keywords.to_string(),
        None => {
            warn!("Missing 'transcription_keywords' in LLM response, using original keywords");
            return raw_transcription_keywords;
        }
    };

    keywords_str
}

pub fn get_adaptive_interval(check_number: u32) -> Duration {
    match check_number {
        1..=20 => Duration::from_secs(3),

        21..=50 => Duration::from_secs(6),

        51..=100 => Duration::from_secs(12),

        _ => Duration::from_secs(25),
    }
}

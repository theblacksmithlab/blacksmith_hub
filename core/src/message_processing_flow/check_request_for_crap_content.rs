use crate::ai::common::common::raw_llm_processing_json;
use crate::models::common::ai::LlmModel;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::{
    AppsSystemRoles, BlacksmithLabRoleType, ProbiotRoleType, W3ARoleType,
};
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::get_system_role_or_fallback;
use std::sync::Arc;
use tracing::error;

pub async fn check_request_for_common_case<T: OpenAIClientInit + Send + Sync>(
    user_raw_request: &str,
    current_cache: &str,
    app_state: Arc<T>,
    app_name: AppName,
) -> anyhow::Result<bool> {
    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(
            ProbiotRoleType::CommonCaseDetection,
        )),
        AppName::W3ABot => Some(AppsSystemRoles::W3A(W3ARoleType::CommonCaseDetection)),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::CommonCaseDetection)),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::CommonCaseDetection,
        )),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "CommonCaseDetection role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    let llm_message = format!(
        "Текущий запрос пользователя: {}\nИстория чата: {}",
        user_raw_request, current_cache
    );

    let common_case_detection_result =
        raw_llm_processing_json(&system_role, &llm_message, app_state, LlmModel::Light).await?;

    let is_common: bool =
        match serde_json::from_str::<serde_json::Value>(&common_case_detection_result) {
            Ok(json) => json
                .get("is_common")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            Err(err) => {
                error!("Failed to parse JSON: {}", err);
                true
            }
        };

    Ok(is_common)
}

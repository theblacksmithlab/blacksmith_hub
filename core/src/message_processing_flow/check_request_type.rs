use crate::ai::common::common::raw_llm_processing_json;
use crate::models::common::ai::LlmModel;
use crate::models::common::app_name::AppName;
use crate::models::common::request_type::RequestType;
use crate::models::common::system_roles::{
    AppsSystemRoles, BlacksmithLabRoleType, ProbiotRoleType, W3ARoleType,
};
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::get_system_role_or_fallback;
use std::sync::Arc;
use tracing::error;

pub async fn check_request_type<T: OpenAIClientInit + Send + Sync>(
    user_raw_request: &str,
    current_cache: &str,
    app_state: Arc<T>,
    app_name: AppName,
) -> anyhow::Result<RequestType> {
    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(
            ProbiotRoleType::RequestTypeDetection,
        )),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::RequestTypeDetection)),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::RequestTypeDetection,
        )),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "RequestTypeDetection role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    let llm_message = format!(
        "<user_request>{}</user_request>\n\n<chat_history>{}</chat_history>",
        user_raw_request, current_cache
    );

    let request_type_detection_result =
        raw_llm_processing_json(&system_role, &llm_message, app_state, LlmModel::Light).await?;

    let request_type: RequestType =
        match serde_json::from_str::<serde_json::Value>(&request_type_detection_result) {
            Ok(json) => {
                let type_str = json
                    .get("request_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("common");

                match type_str {
                    "common" => RequestType::Common,
                    "special" => RequestType::Special,
                    "invalid" => RequestType::Invalid,
                    _ => {
                        error!("Unknown request_type '{}', defaulting to Common", type_str);
                        RequestType::Common
                    }
                }
            }
            Err(err) => {
                error!("Failed to parse JSON: {}", err);
                RequestType::Common
            }
        };

    Ok(request_type)
}

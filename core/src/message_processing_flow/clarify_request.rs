use crate::ai::common::common::raw_llm_processing;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::{AppsSystemRoles, BlacksmithLabRoleType, ProbiotRoleType, W3ARoleType};
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::{get_system_role_or_fallback, LlmModel};
use std::sync::Arc;
use tracing::{error, info};

pub async fn clarify_request<T: OpenAIClientInit + Send + Sync>(
    user_raw_request: &str,
    current_cache: &str,
    app_state: Arc<T>,
    app_name: AppName,
) -> anyhow::Result<String> {
    let llm_message = format!(
        "User's current query: {}\nChat history: {}",
        user_raw_request, current_cache
    );

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(ProbiotRoleType::ClarifyRequest)),
        AppName::W3ABot => Some(AppsSystemRoles::W3A(W3ARoleType::ClarifyRequest)),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::ClarifyRequest)),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(BlacksmithLabRoleType::ClarifyRequest)),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "ClarifyRequest role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    match raw_llm_processing(&system_role, &llm_message, app_state, LlmModel::Light).await {
        Ok(clarified_request) => {
            info!("User's raw request clarified successfully");
            Ok(clarified_request)
        }
        Err(err) => {
            error!("Error in raw_llm_processing: {}", err);
            Ok(user_raw_request.to_string())
        }
    }
}

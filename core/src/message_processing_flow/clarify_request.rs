use crate::ai::common::google::raw_google_processing;
use crate::ai::common::openai::raw_openai_processing;
use crate::models::common::ai::{GoogleModel, OpenAIModel};
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::{
    AppsSystemRoles, BlacksmithLabRoleType, ProbiotRoleType, W3ARoleType,
};
use crate::state::llm_client_init_trait::{GoogleClientInit, OpenAIClientInit};
use crate::utils::common::get_system_role;
use std::sync::Arc;
use tracing::{error, info, warn};

pub async fn clarify_query<T: OpenAIClientInit + GoogleClientInit + Send + Sync>(
    user_raw_query: &str,
    current_cache: &str,
    app_state: Arc<T>,
    app_name: AppName,
) -> anyhow::Result<String> {
    let chat_history_section = if current_cache.trim().is_empty() {
        "<chat_history>Нет предыдущих сообщений</chat_history>".to_string()
    } else {
        format!("<chat_history>\n{}\n</chat_history>", current_cache)
    };

    let llm_message = format!(
        "{}\n\n<current_query>\n{}\n</current_query>",
        chat_history_section, user_raw_query
    );

    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(ProbiotRoleType::ClarifyQuery)),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::ClarifyQuery)),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::ClarifyQuery,
        )),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role(&app_name, role.as_str())?,
        None => {
            return Err(anyhow::anyhow!(
                "ClarifyQuery system role is not defined for app '{}'",
                app_name.as_str()
            ));
        }
    };

    let clarified_query = match raw_google_processing(
        &system_role,
        &llm_message,
        app_state.clone(),
        GoogleModel::Flash,
    )
    .await
    {
        Ok(result) => {
            info!("User's raw query clarified successfully (Google)");
            result
        }
        Err(e) => {
            warn!(
                "Google clarify processing failed: {}. Falling back to OpenAI.",
                e
            );
            match raw_openai_processing(&system_role, &llm_message, app_state, OpenAIModel::GPT5lr)
                .await
            {
                Ok(result) => {
                    info!("User's raw query clarified successfully (OpenAI fallback)");
                    result
                }
                Err(err) => {
                    error!("Both Google and OpenAI failed to clarify query: {}", err);
                    user_raw_query.to_string()
                }
            }
        }
    };

    Ok(clarified_query)
}

use crate::ai::common::google::raw_google_processing_json;
use crate::models::common::ai::GoogleModel;
// use crate::models::common::ai::OpenAIModel;
use crate::models::common::app_name::AppName;
use crate::models::common::query_type::QueryType;
use crate::models::common::system_roles::{
    AppsSystemRoles, BlacksmithLabRoleType, ProbiotRoleType, W3ARoleType,
};
use crate::state::llm_client_init_trait::{GoogleClientInit, OpenAIClientInit};
use crate::utils::common::get_system_role_or_fallback;
use std::sync::Arc;
use tracing::{error, info};

pub async fn get_query_type<T: OpenAIClientInit + GoogleClientInit + Send + Sync>(
    user_raw_query: &str,
    current_cache: &str,
    app_state: Arc<T>,
    app_name: AppName,
) -> anyhow::Result<QueryType> {
    let system_role = match app_name {
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(
            ProbiotRoleType::QueryTypeDefinition,
        )),
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::QueryTypeDefinition)),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::QueryTypeDefinition,
        )),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "QueryTypeDefinition role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    let chat_history_section = if current_cache.trim().is_empty() {
        "<chat_history>Нет предыдущих сообщений</chat_history>".to_string()
    } else {
        format!("<chat_history>{}</chat_history>", current_cache)
    };

    let llm_message = format!(
        "{}\n\n<current_query>{}</current_query>",
        chat_history_section, user_raw_query
    );

    // Google request processing
    let query_type_definition_result =
        raw_google_processing_json(&system_role, &llm_message, app_state, GoogleModel::Flash).await?;

    info!("Google processing json result: {}", query_type_definition_result);

    // // OpenAI request processing
    // let query_type_definition_result =
    //     raw_openai_processing_json(&system_role, &llm_message, app_state, OpenAIModel::GPT4o).await?;

    let request_type: QueryType =
        match serde_json::from_str::<serde_json::Value>(&query_type_definition_result) {
            Ok(json) => {
                let type_str = json
                    .get("request_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("special");

                match type_str {
                    "common" => QueryType::Common,
                    "special" => QueryType::Special,
                    "invalid" => QueryType::Invalid,
                    _ => {
                        error!("Unknown request_type '{}', defaulting to Special", type_str);
                        QueryType::Special
                    }
                }
            }
            Err(err) => {
                error!("Failed to parse JSON: {}", err);
                QueryType::Special
            }
        };

    Ok(request_type)
}

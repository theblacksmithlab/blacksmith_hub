use crate::ai::common::google::raw_google_processing_json;
use crate::ai::common::openai::raw_openai_processing_json;
use crate::models::common::ai::{GoogleModel, OpenAIModel};
use crate::models::common::app_name::AppName;
use crate::models::common::query_type::QueryType;
use crate::models::common::system_roles::{
    AppsSystemRoles, BlacksmithLabRoleType, ProbiotRoleType, W3ARoleType,
};
use crate::state::llm_client_init_trait::{GoogleClientInit, OpenAIClientInit};
use crate::utils::common::get_system_role;
use std::sync::Arc;
use tracing::{error, info, warn};

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
        Some(role) => get_system_role(&app_name, role.as_str())?,
        None => {
            return Err(anyhow::anyhow!(
                "QueryTypeDefinition system role is not defined for app '{}'",
                app_name.as_str()
            ));
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

    let query_type_definition_result = match raw_google_processing_json(
        &system_role,
        &llm_message,
        app_state.clone(),
        GoogleModel::Flash,
    )
    .await
    {
        Ok(result) => {
            info!("Google query type processing json result: {}", result);
            result
        }
        Err(e) => {
            warn!(
                "Google query type processing failed: {}. Falling back to OpenAI.",
                e
            );
            let openai_query_type_processing_result = raw_openai_processing_json(
                &system_role,
                &llm_message,
                app_state,
                OpenAIModel::GPT5mini,
            )
            .await?;
            info!(
                "OpenAI query type processing json result: {}",
                openai_query_type_processing_result
            );
            openai_query_type_processing_result
        }
    };

    let query_type: QueryType =
        match serde_json::from_str::<serde_json::Value>(&query_type_definition_result) {
            Ok(json) => {
                let type_str = json
                    .get("query_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("special");

                match type_str {
                    "common" => QueryType::Common,
                    "special" => QueryType::Special,
                    "invalid" => QueryType::Invalid,
                    "support" => QueryType::Support,
                    _ => {
                        error!(
                            "Got unknown query type from llm: '{}', defaulting to 'special'",
                            type_str
                        );
                        QueryType::Special
                    }
                }
            }
            Err(err) => {
                error!(
                    "Failed to parse QueryType JSON: {}, defaulting to 'special'",
                    err
                );
                QueryType::Special
            }
        };

    Ok(query_type)
}

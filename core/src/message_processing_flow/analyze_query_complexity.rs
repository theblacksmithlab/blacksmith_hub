use crate::ai::common::common::raw_llm_processing_json;
use crate::models::common::ai::LlmModel;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::{
    AppsSystemRoles, BlacksmithLabRoleType, ProbiotRoleType, W3ARoleType,
};
use crate::rag_system::query_decompression_types::QueryComplexity;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::get_system_role_or_fallback;
use std::sync::Arc;
use tracing::error;

pub async fn analyze_query_complexity<T: OpenAIClientInit + Send + Sync>(
    clarified_request: &str,
    current_cache: &str,
    app_state: Arc<T>,
    app_name: AppName,
) -> anyhow::Result<QueryComplexity> {
    let system_role = match app_name {
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::QueryComplexityAnalysis)),
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(
            ProbiotRoleType::QueryComplexityAnalysis,
        )),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::QueryComplexityAnalysis,
        )),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "QueryComplexityAnalysis role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    let chat_history_section = if current_cache.trim().is_empty() {
        "<chat_history>Нет предыдущих сообщений</chat_history>".to_string()
    } else {
        format!("<chat_history>\n{}\n</chat_history>", current_cache)
    };

    let llm_message = format!(
        "{}\n\n<current_query>\n{}\n</current_query>",
        chat_history_section, clarified_request
    );

    let query_complexity_result =
        raw_llm_processing_json(&system_role, &llm_message, app_state, LlmModel::Light).await?;

    let query_complexity: QueryComplexity =
        match serde_json::from_str::<serde_json::Value>(&query_complexity_result) {
            Ok(json) => {
                let complexity_str = json
                    .get("query_complexity")
                    .and_then(|v| v.as_str())
                    .unwrap_or("base");

                match complexity_str {
                    "base" => QueryComplexity::Base,
                    "complex" => QueryComplexity::Complex,
                    _ => {
                        error!(
                            "Unknown query_complexity '{}', defaulting to Base",
                            complexity_str
                        );
                        QueryComplexity::Base
                    }
                }
            }
            Err(err) => {
                error!("Failed to parse JSON: {}", err);
                QueryComplexity::Base
            }
        };

    Ok(query_complexity)
}

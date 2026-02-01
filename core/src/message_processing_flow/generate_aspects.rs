use crate::ai::common::google::raw_google_processing_json;
use crate::ai::common::openai::raw_openai_processing_json;
use crate::models::common::ai::{GoogleModel, OpenAIModel};
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::{
    AppsSystemRoles, BlacksmithLabRoleType, ProbiotRoleType, W3ARoleType,
};
use crate::rag_system::query_decompression_types::GeneratedAspects;
use crate::state::llm_client_init_trait::{GoogleClientInit, OpenAIClientInit};
use crate::utils::common::get_system_role;
use std::sync::Arc;
use tracing::{error, info, warn};

pub async fn generate_aspects<T: OpenAIClientInit + GoogleClientInit + Send + Sync>(
    clarified_query: &str,
    current_cache: &str,
    app_state: Arc<T>,
    app_name: AppName,
) -> anyhow::Result<Vec<String>> {
    let chat_history_section = if current_cache.trim().is_empty() {
        "<chat_history>Нет предыдущих сообщений</chat_history>".to_string()
    } else {
        format!("<chat_history>\n{}\n</chat_history>", current_cache)
    };

    let llm_message = format!(
        "{}\n\n<current_query>\n{}\n</current_query>",
        chat_history_section, clarified_query
    );

    let system_role = match app_name {
        AppName::W3AWeb => Some(AppsSystemRoles::W3A(W3ARoleType::AspectGeneration)),
        AppName::BlacksmithWeb => Some(AppsSystemRoles::BlacksmithLab(
            BlacksmithLabRoleType::AspectGeneration,
        )),
        AppName::ProbiotBot => Some(AppsSystemRoles::Probiot(ProbiotRoleType::AspectGeneration)),
        _ => None,
    };

    let system_role = match system_role {
        Some(role) => get_system_role(&app_name, role.as_str())?,
        None => {
            return Err(anyhow::anyhow!(
                "AspectGeneration system role is not defined for app '{}'",
                app_name.as_str()
            ));
        }
    };

    let aspects_json = match raw_google_processing_json(
        &system_role,
        &llm_message,
        app_state.clone(),
        GoogleModel::Flash,
    )
    .await
    {
        Ok(result) => {
            info!("Google aspect generation succeeded");
            result
        }
        Err(e) => {
            warn!(
                "Google aspect generation failed: {}, falling back to OpenAI",
                e
            );
            raw_openai_processing_json(&system_role, &llm_message, app_state, OpenAIModel::GPT5)
                .await?
        }
    };

    match serde_json::from_str::<GeneratedAspects>(&aspects_json) {
        Ok(generated_aspects) => {
            let aspects = generated_aspects.aspects;

            if aspects.len() < 3 {
                warn!(
                    "Generated aspects count is less than 3 (got {}), falling back to Base mode",
                    aspects.len()
                );
                return Err(anyhow::anyhow!(
                    "Generated aspects count is less than 3 (got {})",
                    aspects.len()
                ));
            }

            if aspects.len() > 3 {
                warn!(
                    "Generated aspects count is more than 3 (got {}). Truncating to first 3.",
                    aspects.len()
                );
                let truncated_aspects = aspects.into_iter().take(3).collect();
                info!("Aspects generated successfully (truncated to 3)");
                return Ok(truncated_aspects);
            }

            info!("Aspects generated successfully: {} aspects", aspects.len());
            Ok(aspects)
        }
        Err(err) => {
            error!("Error parsing aspects JSON: {}", err);
            Err(anyhow::anyhow!("Failed to parse aspects JSON: {}", err))
        }
    }
}

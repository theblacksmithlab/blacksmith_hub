use crate::ai::common::common::raw_llm_processing_json;
use crate::models::common::ai::LlmModel;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::{
    AppsSystemRoles, BlacksmithLabRoleType, ProbiotRoleType, W3ARoleType,
};
use crate::rag_system::query_decompression_types::GeneratedAspects;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::get_system_role_or_fallback;
use std::sync::Arc;
use tracing::{error, info, warn};

pub async fn generate_aspects<T: OpenAIClientInit + Send + Sync>(
    clarified_request: &str,
    current_cache: &str,
    app_state: Arc<T>,
    app_name: AppName,
) -> anyhow::Result<Vec<String>> {
    let llm_message = format!(
        "<user_request>\n{}\n</user_request>\n\n<chat_history>\n{}\n</chat_history>",
        clarified_request, current_cache
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
        Some(role) => get_system_role_or_fallback(&app_name, role.as_str(), None),
        None => {
            error!(
                "AspectGeneration role is not defined for app '{}'. Using fallback.",
                app_name.as_str()
            );
            "You are a helpful assistant".to_string()
        }
    };

    let aspects_json =
        raw_llm_processing_json(&system_role, &llm_message, app_state, LlmModel::Light).await?;

    match serde_json::from_str::<GeneratedAspects>(&aspects_json) {
        Ok(generated_aspects) => {
            let aspects = generated_aspects.aspects;

            if aspects.len() < 3 {
                warn!(
                    "Generated aspects count is less than 3 (got {}). This should trigger fallback to Base mode.",
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

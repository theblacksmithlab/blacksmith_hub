use crate::ai::common::openai::raw_openai_processing;
use crate::models::common::ai::OpenAIModel;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::TheViperRoomRoleType;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::get_system_role_or_fallback;
use std::sync::Arc;
use tracing::info;

pub async fn generate_user_nickname<T>(
    app_state: Arc<T>,
    username: String,
    first_name: String,
    last_name: String,
) -> Result<String, String>
where
    T: OpenAIClientInit + Send + Sync + 'static,
{
    let system_role = get_system_role_or_fallback(
        &AppName::TheViperRoomBot,
        TheViperRoomRoleType::SystemNicknameGeneration,
        None,
    );

    let user_data = format!(
        "Username: {}, user's firstname: {}, user's lastname: {}",
        username, first_name, last_name
    );

    info!("Generating a nickname from user's data: {}", user_data);

    raw_openai_processing(&system_role, &user_data, app_state, OpenAIModel::GPT5lr)
        .await
        .map_err(|e| format!("Failed to generate nickname: {}", e))
}

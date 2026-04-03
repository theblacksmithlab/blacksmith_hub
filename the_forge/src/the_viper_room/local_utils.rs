use blacksmith_core::ai::common::openai::raw_openai_processing;
use blacksmith_core::models::common::ai::OpenAIModel;
use blacksmith_core::models::common::app_name::AppName;
use blacksmith_core::models::common::system_roles::TheViperRoomRoleType;
use blacksmith_core::state::the_viper_room::app_state::TheViperRoomAppState;
use blacksmith_core::utils::common::get_system_role_or_fallback;
use std::sync::Arc;

pub async fn generate_user_system_nickname(
    the_viper_room_app_state: Arc<TheViperRoomAppState>,
    username: String,
    first_name: String,
    last_name: String,
) -> Result<String, String> {
    let system_role = get_system_role_or_fallback(
        &AppName::TheViperRoom,
        TheViperRoomRoleType::SystemNicknameGeneration,
        None,
    );

    let user_data = format!(
        "Username: {}, user's firstname: {}, user's lastname: {}",
        username, first_name, last_name
    );

    raw_openai_processing(
        &system_role,
        &user_data,
        the_viper_room_app_state,
        OpenAIModel::GPT5lr,
    )
    .await
    .map_err(|e| format!("Failed to generate nickname: {}", e))
}

pub(crate) async fn get_user_system_nickname(
    the_viper_room_app_state: Arc<TheViperRoomAppState>,
    user_id: u64,
) -> Option<String> {
    let user_data = the_viper_room_app_state.user_data.lock().await;
    user_data
        .get(&user_id)
        .map(|data| data.user_system_nickname.clone())
}

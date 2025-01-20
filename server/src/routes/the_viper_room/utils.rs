use core::ai::utils::raw_llm_processing;
use core::state::the_viper_room::app_state::TheViperRoomAppState;
use core::utils::common::LlmModel;
use std::fs::read_to_string;
use std::sync::Arc;

pub async fn generate_user_system_nickname(
    the_viper_room_app_state: Arc<TheViperRoomAppState>,
    username: String,
    first_name: String,
    last_name: String,
) -> Result<String, String> {
    let system_role = read_to_string("common_res/the_viper_room/ai_utils/system_role_nickname.txt")
        .map_err(|e| format!("Failed to read system role: {}", e))?;

    let user_data = format!(
        "Username: {}, user's firstname: {}, user's lastname: {}",
        username, first_name, last_name
    );

    raw_llm_processing(
        system_role,
        user_data,
        the_viper_room_app_state,
        LlmModel::Complex,
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

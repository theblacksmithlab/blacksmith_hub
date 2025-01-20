use crate::state::the_viper_room::app_state::TheViperRoomAppState;
use crate::utils::common::update_the_viper_room_user_state;
use std::sync::Arc;
use tracing::info;

pub async fn reset_user_state(the_viper_room_app_state: Arc<TheViperRoomAppState>, user_id: u64) {
    update_the_viper_room_user_state(the_viper_room_app_state, user_id, |state| {
        state.awaiting_phone_number = false;
        state.awaiting_passcode = false;
        state.awaiting_2fa = false;
        state.phone_number = None;
        state.passcode = None;
        state.two_fa = None;
        state.client = None;
        state.token = None;
        state.password_token = None;
        state.authorized = false;
    })
    .await;

    info!("User state has been reset for user_id: {}", user_id);
}

pub async fn reset_user_state_with_message(
    the_viper_room_app_state: Arc<TheViperRoomAppState>,
    user_id: u64,
) -> String {
    reset_user_state(the_viper_room_app_state, user_id).await;
    ". Please restart the application and try again.".to_string()
}

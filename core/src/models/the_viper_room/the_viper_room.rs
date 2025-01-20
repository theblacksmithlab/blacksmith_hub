use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct TheViperRoomUserAction {
    pub user_id: i64,
    pub username: Option<String>,
    pub user_first_name: Option<String>,
    pub user_last_name: Option<String>,
    pub action: Option<String>,
    pub action_step: Option<ActionStep>,
    pub session_data: Vec<u8>,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionStep {
    MiniAppInitialized,
    LoginStart,
    SignOut,
}

#[derive(Serialize)]
pub struct TheViperRoomServerResponse {
    pub message: String,
    pub buttons: Vec<String>,
    pub action_buttons: Vec<String>,
    pub can_input: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_data: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<AuthStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_data: Option<Vec<u8>>,
}

#[derive(Serialize, Debug)]
pub enum AuthStage {
    PhoneNumerRequest,
    PasscodeCodeRequest,
    TwoFAPassRequest,
    AuthSuccess,
    AuthError,
    SignedOut,
    MiniAppInitConfirmed,
}

#[derive(Deserialize, Debug)]
pub struct AvatarRequest {
    user_id: i64,
}

#[derive(Serialize)]
pub struct AvatarResponse {
    pub avatar_url: Option<String>,
}

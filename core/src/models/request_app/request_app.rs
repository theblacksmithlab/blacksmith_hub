use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct UserAction {
    pub user_id: i64,
    pub action: String,
    pub username: String,
}

#[derive(Serialize)]
pub struct ServerResponse {
    pub message: String,
    pub buttons: Vec<String>,
    pub action_buttons: Vec<String>,
    pub can_input: bool,
}

#[derive(Deserialize, Debug)]
pub struct AvatarRequest {
    pub user_id: i64,
}

#[derive(Serialize)]
pub struct AvatarResponse {
    pub avatar_url: Option<String>,
}

pub enum RequestAppSystemRoleType {
    ProcessingUsersBioText,
    // ProcessingUsersRequestText,
    ReorderingResults,
}

impl Into<&'static str> for RequestAppSystemRoleType {
    fn into(self) -> &'static str {
        match self {
            RequestAppSystemRoleType::ProcessingUsersBioText => "processing_users_bio_text",
            // RequestAppSystemRoleType::ProcessingUsersRequestText => "processing_users_request_text",
            RequestAppSystemRoleType::ReorderingResults => "reordering_results",
        }
    }
}

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

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct BlacksmithWebUserAction {
    pub user_id: String,
    pub text: String,
    pub app_name: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct BlacksmithWebTTSRequest {
    pub user_id: String,
    pub text: String,
    pub app_name: String
}

#[derive(Serialize)]
pub struct BlacksmithWebServerResponse {
    pub text: String,
}

#[derive(Serialize)]
pub struct BlacksmithWebTTSResponse {
    pub audio_data: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatMessage {
    pub id: i64,
    pub user_id: String,
    pub sender: String,
    pub message: String,
    pub app_name: String,
}
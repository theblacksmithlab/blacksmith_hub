use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct BlacksmithWebUserRequest {
    pub user_id: String,
    pub text: String,
    pub app_name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BlacksmithWebTTSRequest {
    pub user_id: String,
    pub text: String,
    pub app_name: String,
}

#[derive(Serialize)]
pub struct BlacksmithWebServerResponse {
    pub text: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub extra_data_parsed: HashMap<String, String>,
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
    pub created_at: String,
}

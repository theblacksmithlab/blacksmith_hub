use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct BlacksmithWebUserAction {
    pub user_id: i64,
    pub text: String,
    pub app_name: String
}

#[derive(Serialize)]
pub struct BlacksmithWebServerResponse {
    pub text: String,
}
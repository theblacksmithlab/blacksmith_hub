use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
pub struct BlacksmithWebUserAction {
    pub user_id: String,
    pub text: String
}

#[derive(Serialize)]
pub struct BlacksmithWebServerResponse {
    pub text: String,
}
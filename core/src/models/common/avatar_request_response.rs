use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct AvatarRequest {
    pub user_id: i64,
}

#[derive(Serialize)]
pub struct AvatarResponse {
    pub avatar_url: Option<String>,
}

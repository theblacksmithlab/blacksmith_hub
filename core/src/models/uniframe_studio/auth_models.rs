use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SendMagicLinkRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct VerifyTokenRequest {
    pub token: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub success: bool,
    pub message: String,
    pub session_token: Option<String>,
}

#[derive(Serialize)]
pub struct SessionCheckResponse {
    pub valid: bool,
    pub user_email: String,
    pub expires_at: i64,
}

#[derive(Serialize)]
pub struct AuthError {
    pub error: String,
}
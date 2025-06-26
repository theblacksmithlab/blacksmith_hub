use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SendMagicLinkRequest {
    pub email: String,
    pub captcha_token: String,
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
}

#[derive(Serialize)]
pub struct AuthError {
    pub error: String,
}

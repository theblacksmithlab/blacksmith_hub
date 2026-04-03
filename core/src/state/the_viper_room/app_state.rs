use crate::ai::anthropic_client::AnthropicClient;
use crate::ai::google_client::GoogleClient;
use async_openai::config::OpenAIConfig;
use async_openai::Client as OpenAIClient;
use grammers_client::types::{LoginToken, PasswordToken};
use grammers_client::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TheViperRoomAppState {
    pub user_state: Mutex<HashMap<u64, AuthStages>>,
    pub openai_client: OpenAIClient<OpenAIConfig>,
    pub anthropic_client: AnthropicClient,
    pub google_client: GoogleClient,
    pub user_data: Mutex<HashMap<u64, UserData>>,
}

impl TheViperRoomAppState {
    pub fn new(
        openai_client: OpenAIClient<OpenAIConfig>,
        anthropic_client: AnthropicClient,
        google_client: GoogleClient,
    ) -> Self {
        Self {
            user_state: Mutex::new(HashMap::new()),
            openai_client,
            anthropic_client,
            google_client,
            user_data: Mutex::new(HashMap::new()),
        }
    }
}

#[derive(Default, Clone)]
pub struct AuthStages {
    pub awaiting_phone_number: bool,
    pub awaiting_passcode: bool,
    pub awaiting_2fa: bool,
    pub phone_number: Option<String>,
    pub passcode: Option<String>,
    pub two_fa: Option<String>,
    pub client: Option<Client>,
    pub token: Option<Arc<LoginToken>>,
    pub password_token: Option<PasswordToken>,
    pub authorized: bool,
    pub unauthorized: bool,
}

#[derive(Default, Clone)]
pub struct UserData {
    pub user_system_nickname: String,
}

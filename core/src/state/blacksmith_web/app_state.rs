use crate::ai::anthropic_client::AnthropicClient;
use crate::ai::google_client::GoogleClient;
use crate::models::common::dialogue_cache::DialogueCache;
use async_openai::config::OpenAIConfig;
use async_openai::Client as OpenAIClient;
use qdrant_client::Qdrant;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct BlacksmithWebAppState {
    pub openai_client: OpenAIClient<OpenAIConfig>,
    pub anthropic_client: AnthropicClient,
    pub google_client: GoogleClient,
    pub qdrant_client: Arc<Qdrant>,
    pub temp_cache: Mutex<HashMap<String, DialogueCache>>,
    pub local_db_pool: Pool<Sqlite>,
}

impl BlacksmithWebAppState {
    pub fn new(
        openai_client: OpenAIClient<OpenAIConfig>,
        anthropic_client: AnthropicClient,
        google_client: GoogleClient,
        qdrant_client: Arc<Qdrant>,
        db_pool: Pool<Sqlite>,
    ) -> Self {
        let temp_cache = Mutex::new(HashMap::new());
        Self {
            openai_client,
            anthropic_client,
            google_client,
            qdrant_client,
            temp_cache,
            local_db_pool: db_pool,
        }
    }

    pub fn get_db_pool(&self) -> &Pool<Sqlite> {
        &self.local_db_pool
    }
}

use std::collections::HashMap;
use std::sync::Arc;
use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;
use qdrant_client::Qdrant;
use sqlx::{Pool, Sqlite};
use tokio::sync::Mutex;
use crate::models::common::dialogue_cache::DialogueCache;

pub struct BlacksmithWebAppState {
    pub llm_client: LLM_Client<OpenAIConfig>,
    pub qdrant_client: Arc<Qdrant>,
    pub temp_cache: Mutex<HashMap<String, DialogueCache>>,
    pub local_db_pool: Pool<Sqlite>,
}

impl BlacksmithWebAppState {
    pub fn new(
        llm_client: LLM_Client<OpenAIConfig>,
        qdrant_client: Arc<Qdrant>,
        db_pool: Pool<Sqlite>,
    ) -> Self {
        let temp_cache = Mutex::new(HashMap::new());
        Self {
            llm_client,
            qdrant_client,
            temp_cache,
            local_db_pool: db_pool,
        }
    }

    pub fn get_db_pool(&self) -> &Pool<Sqlite> {
        &self.local_db_pool
    }
}
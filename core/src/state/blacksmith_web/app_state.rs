use std::collections::HashMap;
use std::sync::Arc;
use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;
use qdrant_client::Qdrant;
use teloxide::prelude::ChatId;
use tokio::sync::Mutex;
use crate::models::common::app_name::AppName;
use crate::models::common::dialogue_cache::DialogueCache;

pub struct BlacksmithWebAppState {
    pub llm_client: LLM_Client<OpenAIConfig>,
    pub qdrant_client: Arc<Qdrant>,
    pub temp_cache: Mutex<HashMap<ChatId, DialogueCache>>,
}

impl BlacksmithWebAppState {
    pub fn new(
        llm_client: LLM_Client<OpenAIConfig>,
        qdrant_client: Arc<Qdrant>,
    ) -> Self {
        let temp_cache = Mutex::new(HashMap::new());
        Self {
            llm_client,
            qdrant_client,
            temp_cache,
        }
    }
}
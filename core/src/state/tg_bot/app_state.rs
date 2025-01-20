use crate::models::tg_bot::tg_bot::PodcastManager;
use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;
use std::sync::Arc;

pub struct BotAppState {
    pub llm_client: LLM_Client<OpenAIConfig>,
    pub podcast_manager: Arc<PodcastManager>,
}

impl BotAppState {
    pub fn new(llm_client: LLM_Client<OpenAIConfig>) -> Self {
        let podcast_manager = Arc::new(PodcastManager::new());
        Self {
            llm_client,
            podcast_manager,
        }
    }
}

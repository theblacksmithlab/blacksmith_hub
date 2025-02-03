use crate::models::common::app_name::AppName;
use crate::models::common::dialogue_cache::DialogueCache;
use crate::models::tg_bot::the_viper_room_bot::podcast_manager::PodcastManager;
use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;
use qdrant_client::Qdrant;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct BotAppState {
    pub llm_client: LLM_Client<OpenAIConfig>,
    pub podcast_manager: Arc<PodcastManager>,
    pub temp_cache: Mutex<HashMap<String, DialogueCache>>,
    pub qdrant_client: Arc<Qdrant>,
    pub app_name: AppName,
}

impl BotAppState {
    pub fn new(
        llm_client: LLM_Client<OpenAIConfig>,
        qdrant_client: Arc<Qdrant>,
        app_name: AppName,
    ) -> Self {
        let podcast_manager = Arc::new(PodcastManager::new());
        let temp_cache = Mutex::new(HashMap::new());
        Self {
            llm_client,
            podcast_manager,
            temp_cache,
            qdrant_client,
            app_name,
        }
    }
}

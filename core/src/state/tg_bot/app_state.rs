use crate::models::common::app_name::AppName;
use crate::models::common::dialogue_cache::DialogueCache;
use crate::models::tg_bot::groot_bot::groot_bot::MessageCounts;
use crate::models::tg_bot::groot_bot::groot_bot::{ChatMessageStats, ResourcesDialogState};
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
    pub dialog_states: Option<Mutex<HashMap<u64, ResourcesDialogState>>>,
    pub message_counts: Option<Arc<Mutex<MessageCounts>>>,
    pub chat_message_stats: Option<Arc<Mutex<ChatMessageStats>>>,
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
            dialog_states: None,
            message_counts: None,
            chat_message_stats: None,
        }
    }

    pub async fn with_groot_bot_options(
        llm_client: LLM_Client<OpenAIConfig>,
        qdrant_client: Arc<Qdrant>,
        app_name: AppName,
    ) -> Self {
        let temp_cache = Mutex::new(HashMap::new());
        let podcast_manager = Arc::new(PodcastManager::new());

        let message_counts = Arc::new(Mutex::new(
            MessageCounts::load_message_counts(&app_name).await.unwrap(),
        ));

        let chat_message_stats = Arc::new(Mutex::new(ChatMessageStats::new()));
        {
            let mut chat_stats = chat_message_stats.lock().await;
            chat_stats
                .fetch_chat_history_for_all_chats(&app_name)
                .await
                .unwrap();
        }

        let dialog_states = Some(Mutex::new(HashMap::new()));

        Self {
            llm_client,
            podcast_manager,
            temp_cache,
            qdrant_client,
            app_name,
            chat_message_stats: Some(chat_message_stats),
            message_counts: Some(message_counts),
            dialog_states,
        }
    }
}

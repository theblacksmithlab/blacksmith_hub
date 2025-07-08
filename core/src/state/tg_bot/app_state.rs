use crate::models::common::app_name::AppName;
use crate::models::common::dialogue_cache::DialogueCache;
use crate::models::tg_bot::groot_bot::groot_bot::{ChatMessageStats, ResourcesDialogState};
use crate::models::tg_bot::groot_bot::groot_bot::{MessageCounts, MessageReports};
use crate::models::tg_bot::the_viper_room_bot::podcast_manager::PodcastManager;
use crate::utils::tg_bot::tg_bot::is_localdb_implemented;
use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;
use qdrant_client::Qdrant;
use std::collections::HashMap;
use std::sync::Arc;
use sqlx::SqlitePool;
use tokio::sync::Mutex;
use crate::local_db::tg_bot::tg_bot_local_db::setup_bot_localdb_pool;
use anyhow::Result;


pub struct BotAppState {
    pub llm_client: LLM_Client<OpenAIConfig>,
    pub podcast_manager: Arc<PodcastManager>,
    pub temp_cache: Mutex<HashMap<String, DialogueCache>>,
    pub qdrant_client: Arc<Qdrant>,
    pub app_name: AppName,
    pub dialog_states: Option<Mutex<HashMap<u64, ResourcesDialogState>>>,
    pub message_counts: Option<Arc<Mutex<MessageCounts>>>,
    pub chat_message_stats: Option<Arc<Mutex<ChatMessageStats>>>,
    pub message_reports: Option<Arc<Mutex<MessageReports>>>,
    pub db_pool: Option<Arc<SqlitePool>>,
}

impl BotAppState {
    pub async fn new(
        llm_client: LLM_Client<OpenAIConfig>,
        qdrant_client: Arc<Qdrant>,
        app_name: AppName,
    ) -> Result<Self> {
        let podcast_manager = Arc::new(PodcastManager::new());
        let temp_cache = Mutex::new(HashMap::new());
        let db_pool = if is_localdb_implemented(&app_name) {
            Some(Arc::new(setup_bot_localdb_pool(&app_name).await?))
        } else {
            None
        };

        Ok(Self {
            llm_client,
            podcast_manager,
            temp_cache,
            qdrant_client,
            app_name,
            dialog_states: None,
            message_counts: None,
            chat_message_stats: None,
            message_reports: None,
            db_pool,
        })
    }

    pub async fn with_groot_bot_options(
        llm_client: LLM_Client<OpenAIConfig>,
        qdrant_client: Arc<Qdrant>,
        app_name: AppName,
    ) -> Result<Self> {
        let temp_cache = Mutex::new(HashMap::new());
        let podcast_manager = Arc::new(PodcastManager::new());

        let message_counts = Arc::new(Mutex::new(
            MessageCounts::load_message_counts(&app_name).await?,
        ));

        let chat_message_stats = Arc::new(Mutex::new(ChatMessageStats::new()));
        {
            let mut chat_stats = chat_message_stats.lock().await;
            chat_stats
                .fetch_chat_history_for_all_chats(&app_name)
                .await?;
        }

        let dialog_states = Some(Mutex::new(HashMap::new()));

        let message_reports = Arc::new(Mutex::new(
            MessageReports::load_message_reports(&app_name)
                .await
                .unwrap_or_else(|_| MessageReports::new()),
        ));

        let db_pool = if is_localdb_implemented(&app_name) {
            Some(Arc::new(setup_bot_localdb_pool(&app_name).await?))
        } else {
            None
        };
        
        Ok(Self {
            llm_client,
            podcast_manager,
            temp_cache,
            qdrant_client,
            app_name,
            chat_message_stats: Some(chat_message_stats),
            message_counts: Some(message_counts),
            dialog_states,
            message_reports: Some(message_reports),
            db_pool,
        })
    }
}

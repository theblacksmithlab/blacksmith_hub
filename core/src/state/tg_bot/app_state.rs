use crate::models::common::app_name::AppName;
use crate::models::common::dialogue_cache::DialogueCache;
use crate::models::tg_bot::groot_bot::groot_bot::{ChatMessageStats, ResourcesDialogState};
use crate::models::tg_bot::groot_bot::groot_bot::{MessageCounts, MessageReports};
use crate::models::tg_bot::the_viper_room_bot::podcast_manager::PodcastManager;
use crate::models::tg_bot::the_viper_room_bot::the_viper_room_bot_user_state::TheViperRoomBotUserState;
use crate::models::the_viper_room::db_models::PendingChannel;
use crate::utils::tg_bot::groot_bot::subscription_utils::PaymentProcess;
use crate::utils::tg_bot::tg_bot::is_localdb_implemented;
use crate::utils::uniframe_studio::heleket_client::{HeleketClient, HeleketConfig};
use anyhow::Result;
use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;
use qdrant_client::Qdrant;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::local_db::local_db::setup_app_db_pool;

pub struct BotAppState {
    pub llm_client: LLM_Client<OpenAIConfig>,
    pub podcast_manager: Arc<PodcastManager>,
    pub temp_cache: Mutex<HashMap<String, DialogueCache>>,
    pub qdrant_client: Arc<Qdrant>,
    pub app_name: AppName,
    pub dialog_states: Option<Mutex<HashMap<u64, ResourcesDialogState>>>,
    pub payment_states: Option<Mutex<HashMap<u64, PaymentProcess>>>,
    pub message_counts: Option<Arc<Mutex<MessageCounts>>>,
    pub chat_message_stats: Option<Arc<Mutex<ChatMessageStats>>>,
    pub message_reports: Option<Arc<Mutex<MessageReports>>>,
    pub db_pool: Option<Arc<SqlitePool>>,
    pub heleket_client: Option<HeleketClient>,
    pub the_viper_room_bot_user_states: Option<Mutex<HashMap<u64, TheViperRoomBotUserState>>>,
    pub the_viper_room_bot_pending_channels: Option<Mutex<HashMap<u64, Vec<PendingChannel>>>>,
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
            Some(Arc::new(setup_app_db_pool(&app_name).await?))
        } else {
            None
        };

        // Initialize The Viper Room Bot user states if this is The Viper Room Bot
        let the_viper_room_bot_user_states = if matches!(app_name, AppName::TheViperRoomBot) {
            Some(Mutex::new(HashMap::new()))
        } else {
            None
        };

        // Initialize The Viper Room Bot pending channels storage if this is The Viper Room Bot
        let the_viper_room_bot_pending_channels = if matches!(app_name, AppName::TheViperRoomBot) {
            Some(Mutex::new(HashMap::new()))
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
            payment_states: None,
            message_counts: None,
            chat_message_stats: None,
            message_reports: None,
            db_pool,
            heleket_client: None,
            the_viper_room_bot_user_states,
            the_viper_room_bot_pending_channels,
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

        let payment_states = Some(Mutex::new(HashMap::new()));

        let message_reports = Arc::new(Mutex::new(
            MessageReports::load_message_reports(&app_name)
                .await
                .unwrap_or_else(|_| MessageReports::new()),
        ));

        let db_pool = if is_localdb_implemented(&app_name) {
            Some(Arc::new(setup_app_db_pool(&app_name).await?))
        } else {
            None
        };

        let heleket_client = Some(HeleketClient::new(HeleketConfig::groot_bot()));

        Ok(Self {
            llm_client,
            podcast_manager,
            temp_cache,
            qdrant_client,
            app_name,
            chat_message_stats: Some(chat_message_stats),
            message_counts: Some(message_counts),
            dialog_states,
            payment_states,
            message_reports: Some(message_reports),
            db_pool,
            heleket_client,
            the_viper_room_bot_user_states: None,
            the_viper_room_bot_pending_channels: None,
        })
    }
}

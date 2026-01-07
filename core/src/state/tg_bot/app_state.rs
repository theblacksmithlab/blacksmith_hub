use crate::local_db::local_db::setup_app_db_pool;
use crate::models::common::app_name::AppName;
use crate::models::common::dialogue_cache::DialogueCache;
use crate::models::tg_bot::groot_bot::groot_bot::{ChatMessageStats, ResourcesDialogState};
use crate::models::tg_bot::groot_bot::groot_bot::{MessageCounts, MessageReports};
use crate::models::tg_bot::the_viper_room_bot::podcast_manager::PodcastManager;
use crate::models::tg_bot::the_viper_room_bot::the_viper_room_bot_user_state::TheViperRoomBotUserState;
use crate::models::the_viper_room::db_models::PendingChannel;
use crate::telegram_client::grammers_functionality::initialize_grammers_client;
use crate::telegram_client::telegram_client::TelegramAgent;
use crate::utils::tg_bot::groot_bot::subscription_utils::PaymentProcess;
use crate::utils::tg_bot::tg_bot::is_localdb_implemented;
use crate::utils::uniframe_studio::heleket_client::{HeleketClient, HeleketConfig};
use anyhow::Result;
use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;
use qdrant_client::Qdrant;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

pub trait AppNameProvider {
    fn app_name(&self) -> &AppName;
}

pub struct CoreBotState {
    pub llm_client: LLM_Client<OpenAIConfig>,
    pub temp_cache: Mutex<HashMap<String, DialogueCache>>,
    pub qdrant_client: Arc<Qdrant>,
    pub app_name: AppName,
    pub db_pool: Option<Arc<SqlitePool>>,
}

impl CoreBotState {
    pub async fn new(
        llm_client: LLM_Client<OpenAIConfig>,
        qdrant_client: Arc<Qdrant>,
        app_name: AppName,
    ) -> Result<Self> {
        let temp_cache = Mutex::new(HashMap::new());
        let db_pool = if is_localdb_implemented(&app_name) {
            Some(Arc::new(setup_app_db_pool(&app_name).await?))
        } else {
            None
        };

        Ok(Self {
            llm_client,
            temp_cache,
            qdrant_client,
            app_name,
            db_pool,
        })
    }
}

pub struct ProbiotBotState {
    pub core: Arc<CoreBotState>,
}

impl ProbiotBotState {
    pub async fn new(core: Arc<CoreBotState>) -> Result<Self> {
        Ok(Self { core })
    }
}

impl AppNameProvider for ProbiotBotState {
    fn app_name(&self) -> &AppName {
        &self.core.app_name
    }
}

pub struct GrootBotState {
    pub core: Arc<CoreBotState>,
    pub dialog_states: Mutex<HashMap<u64, ResourcesDialogState>>,
    pub payment_states: Mutex<HashMap<u64, PaymentProcess>>,
    pub message_counts: Arc<Mutex<MessageCounts>>,
    pub chat_message_stats: Arc<Mutex<ChatMessageStats>>,
    pub message_reports: Arc<Mutex<MessageReports>>,
    pub heleket_client: HeleketClient,
}

impl GrootBotState {
    pub async fn new(core: Arc<CoreBotState>) -> Result<Self> {
        let message_counts = Arc::new(Mutex::new(
            MessageCounts::load_message_counts(&core.app_name).await?,
        ));

        let chat_message_stats = Arc::new(Mutex::new(ChatMessageStats::new()));
        {
            let mut chat_stats = chat_message_stats.lock().await;
            chat_stats
                .fetch_chat_history_for_all_chats(&core.app_name)
                .await?;
        }

        let dialog_states = Mutex::new(HashMap::new());
        let payment_states = Mutex::new(HashMap::new());

        let message_reports = Arc::new(Mutex::new(
            MessageReports::load_message_reports(&core.app_name)
                .await
                .unwrap_or_else(|_| MessageReports::new()),
        ));

        let heleket_client = HeleketClient::new(HeleketConfig::groot_bot());

        Ok(Self {
            core,
            dialog_states,
            payment_states,
            message_counts,
            chat_message_stats,
            message_reports,
            heleket_client,
        })
    }
}

impl AppNameProvider for GrootBotState {
    fn app_name(&self) -> &AppName {
        &self.core.app_name
    }
}

pub struct TheViperRoomBotState {
    pub core: Arc<CoreBotState>,
    pub podcast_manager: Arc<PodcastManager>,
    pub user_states: Mutex<HashMap<u64, TheViperRoomBotUserState>>,
    pub pending_channels: Mutex<HashMap<u64, Vec<PendingChannel>>>,
    pub telegram_agent: Arc<TelegramAgent>,
}

impl TheViperRoomBotState {
    pub async fn new(core: Arc<CoreBotState>) -> Result<Self> {
        let podcast_manager = Arc::new(PodcastManager::new());
        let user_states = Mutex::new(HashMap::new());
        let pending_channels = Mutex::new(HashMap::new());

        let tg_agent_id = env::var("TG_AGENT_ID")
            .map_err(|_| anyhow::anyhow!("TG_AGENT_ID must be set in environment"))?;

        let session_path = format!(
            "common_res/the_viper_room/grammers_system_session/{}.session",
            tg_agent_id
        );

        if !Path::new(&session_path).exists() {
            return Err(anyhow::anyhow!(
                "Telegram agent session file not found: {}. Please ensure the session file exists",
                session_path
            ));
        }

        let session_data = fs::read(Path::new(&session_path))
            .map_err(|e| anyhow::anyhow!("Failed to read session file {}: {}", session_path, e))?;

        let g_client = initialize_grammers_client(session_data).await?;

        let client_clone = g_client.clone();
        tokio::spawn(async move {
            info!("Telegram keep-alive task started (ping every 10 minutes)");

            loop {
                tokio::time::sleep(Duration::from_secs(600)).await;

                match client_clone.is_authorized().await {
                    Ok(true) => {
                        debug!("Telegram keep-alive: session is active");
                    }
                    Ok(false) => {
                        warn!("Telegram keep-alive: session is NOT authorized!");
                    }
                    Err(e) => {
                        warn!("Telegram keep-alive ping failed: {}", e);
                    }
                }
            }
        });

        let client_for_updates = g_client.clone();
        tokio::spawn(async move {
            info!("Telegram updates consumer task started");

            loop {
                match client_for_updates.next_update().await {
                    Ok(_update) => {
                        // Consume the update silently - we don't need to process it
                        // This prevents the update queue from overflowing
                    }
                    Err(e) => {
                        warn!("Error consuming Telegram update: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });

        let telegram_agent = Arc::new(TelegramAgent { client: g_client });

        Ok(Self {
            core,
            podcast_manager,
            user_states,
            pending_channels,
            telegram_agent,
        })
    }
}

impl AppNameProvider for TheViperRoomBotState {
    fn app_name(&self) -> &AppName {
        &self.core.app_name
    }
}

pub struct StatBotState {
    pub core: Arc<CoreBotState>,
}

impl StatBotState {
    pub async fn new(core: Arc<CoreBotState>) -> Result<Self> {
        Ok(Self { core })
    }
}

impl AppNameProvider for StatBotState {
    fn app_name(&self) -> &AppName {
        &self.core.app_name
    }
}

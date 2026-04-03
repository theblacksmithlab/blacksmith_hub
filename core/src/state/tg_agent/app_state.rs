use crate::ai::anthropic_client::AnthropicClient;
use crate::ai::google_client::GoogleClient;
use crate::local_db::local_db::setup_app_db_pool;
use crate::models::common::app_name::AppName;
use crate::models::tg_agent::agent_davon::ChatMessageStats;
use anyhow::Result;
use async_openai::config::OpenAIConfig;
use async_openai::Client as OpenAIClient;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AgentAppState {
    pub openai_client: OpenAIClient<OpenAIConfig>,
    pub anthropic_client: AnthropicClient,
    pub google_client: GoogleClient,
    pub db_pool: Arc<SqlitePool>,
    pub app_name: AppName,
    pub chat_message_stats: Arc<Mutex<ChatMessageStats>>,
}

impl AgentAppState {
    pub async fn new(
        openai_client: OpenAIClient<OpenAIConfig>,
        anthropic_client: AnthropicClient,
        google_client: GoogleClient,
        app_name: AppName,
    ) -> Result<Self> {
        let db_pool = Arc::new(setup_app_db_pool(&app_name).await?);

        Ok(Self {
            openai_client,
            anthropic_client,
            google_client,
            db_pool,
            app_name,
            chat_message_stats: Arc::new(Mutex::new(ChatMessageStats::new())),
        })
    }
}

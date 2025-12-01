use crate::models::common::app_name::AppName;
use crate::models::tg_agent::agent_davon::ChatMessageStats;
use anyhow::Result;
use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::local_db::local_db::setup_app_db_pool;

pub struct AgentAppState {
    pub llm_client: LLM_Client<OpenAIConfig>,
    pub db_pool: Arc<SqlitePool>,
    pub app_name: AppName,
    pub chat_message_stats: Arc<Mutex<ChatMessageStats>>,
}

impl AgentAppState {
    pub async fn new(llm_client: LLM_Client<OpenAIConfig>, app_name: AppName) -> Result<Self> {
        let db_pool = Arc::new(setup_app_db_pool(&app_name).await?);

        Ok(Self {
            llm_client,
            db_pool,
            app_name,
            chat_message_stats: Arc::new(Mutex::new(ChatMessageStats::new())),
        })
    }
}

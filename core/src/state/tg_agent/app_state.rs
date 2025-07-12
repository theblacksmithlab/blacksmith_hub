use crate::local_db::tg_bot::tg_bot_local_db::setup_localdb_pool;
use crate::models::common::app_name::AppName;
use anyhow::Result;
use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;
use sqlx::SqlitePool;
use std::sync::Arc;

pub struct AgentAppState {
    pub llm_client: LLM_Client<OpenAIConfig>,
    pub db_pool: Arc<SqlitePool>,
    pub app_name: AppName,
}

impl AgentAppState {
    pub async fn new(llm_client: LLM_Client<OpenAIConfig>, app_name: AppName) -> Result<Self> {
        let db_pool = Arc::new(setup_localdb_pool(&app_name).await?);

        Ok(Self {
            llm_client,
            db_pool,
            app_name,
        })
    }
}

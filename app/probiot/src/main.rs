mod handlers;
mod user_message_processing;
mod probiot_utils;

use crate::handlers::{callback_query_handler, command_handler, message_handler, ProbiotBotCommands};
use async_openai::Client as LLM_Client;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::tg_bot::tg_bot::run_bot_dispatcher;
use dotenv::dotenv;
use qdrant_client::Qdrant;
use std::env;
use std::sync::Arc;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::Update;
use teloxide::{dptree, Bot};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    info!("Starting Probiot...");

    let qdrant_client = Arc::new(
        Qdrant::from_url(&env::var("QDRANT_URL")?)
            .api_key(env::var("QDRANT_API_KEY")?)
            .build()?,
    );

    let llm_client = LLM_Client::new();

    let bot_app_state = Arc::new(BotAppState::new(llm_client, qdrant_client));

    start_probiot(bot_app_state).await?;

    Ok(())
}

pub async fn start_probiot(app_state: Arc<BotAppState>) -> anyhow::Result<()> {
    let bot = Bot::new(env::var("TELOXIDE_TOKEN_PROBIOT")?);

    let cmd_handler = Update::filter_message()
        .filter_command::<ProbiotBotCommands>()
        .endpoint(command_handler);

    let chat_handler = Update::filter_message().endpoint(message_handler);

    let callback_handler = Update::filter_callback_query().endpoint(callback_query_handler);

    let main_handler = dptree::entry()
        .branch(cmd_handler)
        .branch(chat_handler);

    run_bot_dispatcher(bot, main_handler, app_state, Some(callback_handler)).await?;

    Ok(())
}

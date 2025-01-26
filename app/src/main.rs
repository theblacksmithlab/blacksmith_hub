use crate::probiot::start_probiot::start_probiot;
use crate::request_app_bot::start_request_app_bot::start_request_app_bot;
use crate::the_viper_room_bot::start_the_viper_room_bot::start_the_viper_room_bot;
use async_openai::Client as LLM_Client;
use core::models::common::app_name::AppName;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::tg_bot::tg_bot::run_bot_dispatcher;
use dotenv::dotenv;
use qdrant_client::Qdrant;
use std::env;
use std::sync::Arc;
use teloxide::dispatching::UpdateHandler;
use teloxide::{dptree, Bot};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod probiot;
mod request_app_bot;
mod the_viper_room_bot;
mod tester;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    info!("Determining APP_NAME of the bot being launched...");

    let app_name_str = env::var("APP_NAME").unwrap_or_else(|_| "tester_bot".to_string());
    let app_name = match app_name_str.as_str() {
        "probiot" => AppName::Probiot,
        "the_viper_room" => AppName::TheViperRoom,
        "the_viper_room_bot" => AppName::TheViperRoomBot,
        "request_app" => AppName::RequestApp,
        "request_app_bot" => AppName::RequestAppBot,
        "tester_bot" => AppName::TesterBot,
        _ => return Err(anyhow::anyhow!("Unknown app name: {}", app_name_str)),
    };

    let qdrant_client = Arc::new(
        Qdrant::from_url(&env::var("QDRANT_URL")?)
            .api_key(env::var("QDRANT_API_KEY")?)
            .build()?,
    );

    let llm_client = LLM_Client::new();

    let app_state = Arc::new(BotAppState::new(
        llm_client,
        qdrant_client,
        app_name.clone(),
    ));

    match app_name {
        AppName::Probiot => start_probiot(app_state).await?,
        AppName::TheViperRoomBot => start_the_viper_room_bot(app_state).await?,
        AppName::RequestAppBot => start_request_app_bot(app_state).await?,
        _ => {
            return Err(anyhow::anyhow!(
                "Bot not implemented for app: {}",
                app_name.as_str()
            ))
        }
    };

    Ok(())
}

pub async fn start_bot(
    app_name: AppName,
    app_state: Arc<BotAppState>,
    command_handler: UpdateHandler<anyhow::Error>,
    message_handler: UpdateHandler<anyhow::Error>,
    callback_query_handler: Option<UpdateHandler<anyhow::Error>>,
) -> anyhow::Result<()> {
    let bot = match app_name {
        AppName::Probiot => Bot::new(env::var("TELOXIDE_TOKEN_PROBIOT")?),
        AppName::TheViperRoomBot => Bot::new(env::var("TELOXIDE_TOKEN_THE_VIPER_ROOM")?),
        AppName::RequestAppBot => Bot::new(env::var("TELOXIDE_TOKEN_REQUEST_APP")?),
        AppName::TesterBot => Bot::new(env::var("TELOXIDE_TOKEN_TESTER")?),
        _ => return Err(anyhow::anyhow!("Unsupported app name")),
    };

    info!("Starting | {} | Telegram bot", app_state.app_name.as_str());

    let main_handler = dptree::entry()
        .branch(command_handler)
        .branch(message_handler);

    run_bot_dispatcher(bot, main_handler, app_state, callback_query_handler).await?;

    Ok(())
}

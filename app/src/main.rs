use crate::groot_bot::groot_bot_handlers::{groot_bot_command_handler, groot_bot_message_handler};
use crate::probiot_bot::probiot_bot_handlers::{
    probiot_callback_query_handler, probiot_command_handler,
};
use crate::request_app_bot::request_app_bot_handlers::{
    request_app_command_handler, request_app_message_handler,
};
use crate::tester_bot::tester_bot_handlers::{
    tester_bot_command_handler, tester_bot_message_handler,
};
use crate::the_viper_room_bot::the_viper_room_bot_handlers::{
    the_viper_room_command_handler, the_viper_room_message_handler,
};
use crate::w3a_bot::w3a_bot_handlers::w3a_bot_command_handler;
use anyhow::{anyhow, Result};
use async_openai::Client as LLM_Client;
use core::message_processing_flow::tg_bot::default_message_handler::default_message_handler;
use core::models::common::app_name::AppName;
use core::models::tg_bot::groot_bot::groot_bot::GrootBotCommands;
use core::models::tg_bot::probiot_bot::probiot_bot_commands::ProbiotBotCommands;
use core::models::tg_bot::request_app_bot::request_app_bot_commands::RequestAppBotCommands;
use core::models::tg_bot::tester_bot::tester_bot_commands::TesterBotCommands;
use core::models::tg_bot::the_viper_room_bot::the_viper_room_bot_commands::TheViperRoomBotCommands;
use core::models::tg_bot::w3a_bot::w3a_bot_commands::W3ABotCommands;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::tg_bot::tg_bot::reset_tmp_dir;
use core::utils::tg_bot::tg_bot::run_bot_dispatcher;
use dotenv::dotenv;
use qdrant_client::Qdrant;
use std::env;
use std::sync::Arc;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt, UpdateHandler};
use teloxide::prelude::Update;
use teloxide::{dptree, Bot};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

pub mod groot_bot;
pub mod probiot_bot;
pub mod request_app_bot;
pub mod tester_bot;
pub mod the_viper_room_bot;
pub mod w3a_bot;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    info!("Determining APP_NAME of the Telegram bot being launched...");

    let app_name_str = env::var("APP_NAME").unwrap_or_else(|_| "tester_bot".to_string());
    let app_name = match app_name_str.as_str() {
        "probiot_bot" => AppName::ProbiotBot,
        "the_viper_room_bot" => AppName::TheViperRoomBot,
        "request_app_bot" => AppName::RequestAppBot,
        "tester_bot" => AppName::TesterBot,
        "w3a_bot" => AppName::W3ABot,
        "groot_bot" => AppName::GrootBot,
        "the_viper_room" | "request_app" | "w3a_web" | "blacksmith_web" => {
            info!(
                "No Telegram bot system implementation for app: {}",
                app_name_str
            );
            return Ok(());
        }
        _ => return Err(anyhow::anyhow!("Unknown APP_NAME: {}", app_name_str)),
    };

    if let Err(e) = reset_tmp_dir(&app_name) {
        error!("Failed to reset tmp directory: {}", e);
    }

    let qdrant_client = Arc::new(
        Qdrant::from_url(&env::var("QDRANT_URL")?)
            .api_key(env::var("QDRANT_API_KEY")?)
            .build()?,
    );

    let llm_client = LLM_Client::new();

    let app_state = if app_name == AppName::GrootBot {
        Arc::new(
            BotAppState::with_groot_bot_options(llm_client, qdrant_client, app_name.clone()).await,
        )
    } else {
        Arc::new(BotAppState::new(
            llm_client,
            qdrant_client,
            app_name.clone(),
        ))
    };

    let handlers = match get_handlers(&app_name) {
        Ok(handlers) => handlers,
        Err(err) => {
            info!("{}", err);
            return Ok(());
        }
    };

    if let Err(err) = start_bot_with_handlers(app_state, handlers).await {
        error!(
            "Failed to start bot with app_name '{}': {}",
            app_name.as_str(),
            err
        );
    }

    Ok(())
}

async fn start_bot_with_handlers(
    app_state: Arc<BotAppState>,
    handlers: (
        UpdateHandler<anyhow::Error>,
        UpdateHandler<anyhow::Error>,
        Option<UpdateHandler<anyhow::Error>>,
    ),
) -> Result<()> {
    let (command_handler, message_handler, callback_query_handler) = handlers;
    let bot = match app_state.app_name {
        AppName::ProbiotBot => Bot::new(env::var("TELOXIDE_TOKEN_PROBIOT")?),
        AppName::TheViperRoomBot => Bot::new(env::var("TELOXIDE_TOKEN_THE_VIPER_ROOM")?),
        AppName::RequestAppBot => Bot::new(env::var("TELOXIDE_TOKEN_REQUEST_APP")?),
        AppName::TesterBot => Bot::new(env::var("TELOXIDE_TOKEN_TESTER")?),
        AppName::W3ABot => Bot::new(env::var("TELOXIDE_TOKEN_W3A")?),
        AppName::GrootBot => Bot::new(env::var("TELOXIDE_TOKEN_GROOT")?),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported app type of the app: {}",
                app_state.app_name
            ))
        }
    };

    info!(
        "Starting | {} | Telegram bot...",
        app_state.app_name.as_str()
    );

    let main_handler = dptree::entry()
        .branch(command_handler)
        .branch(message_handler);

    run_bot_dispatcher(bot, main_handler, app_state.clone(), callback_query_handler).await?;

    Ok(())
}

fn get_handlers(
    app_name: &AppName,
) -> Result<(
    UpdateHandler<anyhow::Error>,
    UpdateHandler<anyhow::Error>,
    Option<UpdateHandler<anyhow::Error>>,
)> {
    match app_name {
        AppName::ProbiotBot => Ok((
            Update::filter_message()
                .filter_command::<ProbiotBotCommands>()
                .endpoint(probiot_command_handler),
            Update::filter_message().endpoint(default_message_handler),
            Some(Update::filter_callback_query().endpoint(probiot_callback_query_handler)),
        )),
        AppName::TheViperRoomBot => Ok((
            Update::filter_message()
                .filter_command::<TheViperRoomBotCommands>()
                .endpoint(the_viper_room_command_handler),
            Update::filter_message().endpoint(the_viper_room_message_handler),
            None,
        )),
        AppName::RequestAppBot => Ok((
            Update::filter_message()
                .filter_command::<RequestAppBotCommands>()
                .endpoint(request_app_command_handler),
            Update::filter_message().endpoint(request_app_message_handler),
            None,
        )),
        AppName::TesterBot => Ok((
            Update::filter_message()
                .filter_command::<TesterBotCommands>()
                .endpoint(tester_bot_command_handler),
            Update::filter_message().endpoint(tester_bot_message_handler),
            None,
        )),
        AppName::W3ABot => Ok((
            Update::filter_message()
                .filter_command::<W3ABotCommands>()
                .endpoint(w3a_bot_command_handler),
            Update::filter_message().endpoint(default_message_handler),
            None,
        )),
        AppName::GrootBot => Ok((
            Update::filter_message()
                .filter_command::<GrootBotCommands>()
                .endpoint(groot_bot_command_handler),
            Update::filter_message().endpoint(groot_bot_message_handler),
            None,
        )),
        AppName::TheViperRoom | AppName::RequestApp | AppName::W3AWeb | AppName::BlacksmithWeb => {
            Err(anyhow!(
                "No Telegram bot implementation for app: {}",
                app_name.as_str()
            ))
        }
    }
}

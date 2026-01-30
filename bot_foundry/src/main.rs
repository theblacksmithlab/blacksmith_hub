use crate::groot_bot::groot_bot_callback_query_handler::groot_bot_callback_query_handler;
use crate::groot_bot::groot_bot_handlers::{groot_bot_command_handler, groot_bot_message_handler};
use crate::probiot_bot::probiot_bot_handlers::{
    probiot_callback_query_handler, probiot_command_handler,
};
use crate::stat_bot::stat_bot_handlers::{
    stat_bot_callback_query_handler, stat_bot_command_handler, stat_bot_message_handler,
};
use crate::the_viper_room_bot::the_viper_room_bot_callback_query_handler::the_viper_room_bor_callback_query_handler;
use crate::the_viper_room_bot::the_viper_room_bot_command_handler::the_viper_room_command_handler;
use crate::the_viper_room_bot::the_viper_room_bot_message_handler::the_viper_room_message_handler;
use anyhow::{anyhow, Result};
use async_openai::Client as OpenAIClient;
use core::ai::anthropic_client::AnthropicClient;
use core::ai::google_client::GoogleClient;
use core::message_processing_flow::tg_bot::default_message_handler::default_message_handler;
use core::models::common::app_name::AppName;
use core::models::tg_bot::groot_bot::groot_bot::GrootBotCommands;
use core::models::tg_bot::probiot_bot::probiot_bot_commands::ProbiotBotCommands;
use core::models::tg_bot::stat_bot::StatBotCommands;
use core::models::tg_bot::the_viper_room_bot::the_viper_room_bot_commands::TheViperRoomBotCommands;
use core::state::tg_bot::{
    CoreBotState, GrootBotState, ProbiotBotState, StatBotState, TheViperRoomBotState,
};
use core::utils::tg_bot::tg_bot::create_app_tmp_dir;
use core::utils::tg_bot::tg_bot::run_bot_dispatcher;
use dotenv::dotenv;
use qdrant_client::Qdrant;
use rustls::crypto::{aws_lc_rs, CryptoProvider};
use std::env;
use std::sync::Arc;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt, UpdateHandler};
use teloxide::prelude::Update;
use teloxide::{dptree, Bot};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

pub mod groot_bot;
pub mod probiot_bot;
pub mod stat_bot;
pub mod the_viper_room_bot;

enum BotState {
    Probiot(Arc<ProbiotBotState>),
    Groot(Arc<GrootBotState>),
    TheViperRoom(Arc<TheViperRoomBotState>),
    Stat(Arc<StatBotState>),
}

impl BotState {
    fn app_name(&self) -> &AppName {
        match self {
            BotState::Probiot(state) => &state.core.app_name,
            BotState::Groot(state) => &state.core.app_name,
            BotState::TheViperRoom(state) => &state.core.app_name,
            BotState::Stat(state) => &state.core.app_name,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = CryptoProvider::install_default(aws_lc_rs::default_provider()) {
        error!("Failed to install CryptoProvider: {:?}", e);
    }

    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::new("info").add_directive("grammers_session::message_box=warn".parse()?),
        )
        .init();

    info!("Determining AppName of the Telegram bot being launched...");

    let app_name_str = env::var("APP_NAME")
        .map_err(|_| anyhow::anyhow!("APP_NAME environment variable is required"))?;

    let app_name = match app_name_str.as_str() {
        "probiot_bot" => AppName::ProbiotBot,
        "the_viper_room_bot" => AppName::TheViperRoomBot,
        "groot_bot" => AppName::GrootBot,
        "stat_bot" => AppName::StatBot,
        "the_viper_room" | "w3a_web" | "blacksmith_web" => {
            info!("No Telegram bot system implementation for {}", app_name_str);
            return Ok(());
        }
        _ => return Err(anyhow::anyhow!("Unknown APP_NAME: {}", app_name_str)),
    };

    if let Err(e) = create_app_tmp_dir(&app_name) {
        error!("Failed to create app tmp directory: {}", e);
    }

    let qdrant_client = Arc::new(
        Qdrant::from_url(&env::var("QDRANT_CLOUD_URL")?)
            .api_key(env::var("QDRANT_API_KEY")?)
            .build()?,
    );

    let openai_client = OpenAIClient::new();
    let anthropic_client = AnthropicClient::new()?;
    let google_client = GoogleClient::new()?;

    let core = Arc::new(
        CoreBotState::new(
            openai_client,
            anthropic_client,
            google_client,
            qdrant_client,
            app_name.clone(),
        )
        .await?,
    );

    let bot_state = match app_name {
        AppName::ProbiotBot => BotState::Probiot(Arc::new(ProbiotBotState::new(core).await?)),
        AppName::GrootBot => BotState::Groot(Arc::new(GrootBotState::new(core).await?)),
        AppName::TheViperRoomBot => {
            let state = Arc::new(TheViperRoomBotState::new(core).await?);

            if let Err(e) =
                the_viper_room_bot::the_viper_room_bot_utils::schedule_daily_cleanup().await
            {
                error!("Failed to schedule daily cleanup: {}", e);
            }

            BotState::TheViperRoom(state)
        }
        AppName::StatBot => BotState::Stat(Arc::new(StatBotState::new(core).await?)),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported bot app_name: {}",
                app_name.as_str()
            ))
        }
    };

    let handlers = match get_handlers(&app_name) {
        Ok(handlers) => handlers,
        Err(err) => {
            info!("{}", err);
            return Ok(());
        }
    };

    if let Err(err) = start_bot_with_handlers(bot_state, handlers).await {
        error!(
            "Failed to start bot with app_name '{}': {}",
            app_name.as_str(),
            err
        );
    }

    Ok(())
}

async fn start_bot_with_handlers(
    bot_state: BotState,
    handlers: (
        UpdateHandler<anyhow::Error>,
        UpdateHandler<anyhow::Error>,
        Option<UpdateHandler<anyhow::Error>>,
        Option<UpdateHandler<anyhow::Error>>,
    ),
) -> Result<()> {
    let (command_handler, message_handler, callback_query_handler, edited_handler) = handlers;
    let bot = match bot_state.app_name() {
        AppName::ProbiotBot => Bot::new(env::var("TELOXIDE_TOKEN_PROBIOT_BOT")?),
        AppName::TheViperRoomBot => Bot::new(env::var("TELOXIDE_TOKEN_THE_VIPER_ROOM_BOT")?),
        AppName::GrootBot => Bot::new(env::var("TELOXIDE_TOKEN_GROOT_BOT")?),
        AppName::StatBot => Bot::new(env::var("TELOXIDE_TOKEN_STAT_BOT")?),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported app type of the app: {}",
                bot_state.app_name()
            ))
        }
    };

    info!(
        "Starting | {} | Telegram bot...",
        bot_state.app_name().as_str()
    );

    let mut main_handler = dptree::entry()
        .branch(command_handler)
        .branch(message_handler);

    if let Some(edited) = edited_handler {
        main_handler = main_handler.branch(edited);
    }

    match bot_state {
        BotState::Probiot(state) => {
            run_bot_dispatcher(bot, main_handler, state, callback_query_handler).await?
        }
        BotState::Groot(state) => {
            run_bot_dispatcher(bot, main_handler, state, callback_query_handler).await?
        }
        BotState::TheViperRoom(state) => {
            run_bot_dispatcher(bot, main_handler, state, callback_query_handler).await?
        }
        BotState::Stat(state) => {
            run_bot_dispatcher(bot, main_handler, state, callback_query_handler).await?
        }
    }

    Ok(())
}

fn get_handlers(
    app_name: &AppName,
) -> Result<(
    UpdateHandler<anyhow::Error>,
    UpdateHandler<anyhow::Error>,
    Option<UpdateHandler<anyhow::Error>>,
    Option<UpdateHandler<anyhow::Error>>,
)> {
    match app_name {
        AppName::ProbiotBot => Ok((
            Update::filter_message()
                .filter_command::<ProbiotBotCommands>()
                .endpoint(probiot_command_handler),
            Update::filter_message().endpoint(default_message_handler::<ProbiotBotState>),
            Some(Update::filter_callback_query().endpoint(probiot_callback_query_handler)),
            None,
        )),
        AppName::TheViperRoomBot => Ok((
            Update::filter_message()
                .filter_command::<TheViperRoomBotCommands>()
                .endpoint(the_viper_room_command_handler),
            Update::filter_message().endpoint(the_viper_room_message_handler),
            Some(
                Update::filter_callback_query().endpoint(the_viper_room_bor_callback_query_handler),
            ),
            None,
        )),
        AppName::GrootBot => Ok((
            Update::filter_message()
                .filter_command::<GrootBotCommands>()
                .endpoint(groot_bot_command_handler),
            Update::filter_message().endpoint(groot_bot_message_handler),
            Some(Update::filter_callback_query().endpoint(groot_bot_callback_query_handler)),
            Some(Update::filter_edited_message().endpoint(groot_bot_message_handler)),
        )),
        AppName::StatBot => Ok((
            Update::filter_message()
                .filter_command::<StatBotCommands>()
                .endpoint(stat_bot_command_handler),
            Update::filter_message().endpoint(stat_bot_message_handler),
            Some(Update::filter_callback_query().endpoint(stat_bot_callback_query_handler)),
            None,
        )),
        AppName::TheViperRoom
        | AppName::W3AWeb
        | AppName::BlacksmithWeb
        | AppName::UniframeStudio
        | AppName::AgentDavon => Err(anyhow!(
            "No Telegram bot implementation for app: {}",
            app_name.as_str()
        )),
    }
}

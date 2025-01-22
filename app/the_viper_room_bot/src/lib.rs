pub mod handlers;
pub mod local_utils;

use crate::handlers::{command_handler, message_handler, BotCommands};
use anyhow::Result;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::tg_bot::tg_bot::run_bot_dispatcher;
use std::env;
use std::sync::Arc;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::Update;
use teloxide::{dptree, Bot};

pub async fn start_the_viper_room_bot(app_state: Arc<BotAppState>) -> Result<()> {
    let bot = Bot::new(env::var("TELOXIDE_TOKEN_THE_VIPER_ROOM")?);

    let cmd_handler = Update::filter_message()
        .filter_command::<BotCommands>()
        .endpoint(command_handler);

    let chat_handler = Update::filter_message().endpoint(message_handler);

    let handler = dptree::entry().branch(cmd_handler).branch(chat_handler);

    run_bot_dispatcher(bot, handler, app_state).await?;

    Ok(())
}

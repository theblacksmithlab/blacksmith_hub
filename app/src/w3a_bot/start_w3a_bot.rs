use crate::w3a_bot::w3a_bot_handlers::{
    w3a_bot_command_handler,
    w3a_bot_message_handler,
    w3a_bot_callback_query_handler
};
use crate::start_bot;
use core::models::common::app_name::AppName;
use core::models::tg_bot::w3a_bot::w3a_bot_commands::W3ABotCommands;
use core::state::tg_bot::app_state::BotAppState;
use std::sync::Arc;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::Update;

pub async fn start_w3a_bot(app_state: Arc<BotAppState>) -> anyhow::Result<()> {
    let command_handler = Update::filter_message()
        .filter_command::<W3ABotCommands>()
        .endpoint(w3a_bot_command_handler);

    let message_handler = Update::filter_message().endpoint(w3a_bot_message_handler);

    let callback_query_handler =
        Some(Update::filter_callback_query().endpoint(w3a_bot_callback_query_handler));

    start_bot(
        AppName::W3ABot,
        app_state,
        command_handler,
        message_handler,
        callback_query_handler,
    )
        .await
}
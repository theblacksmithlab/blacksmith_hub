use crate::probiot::probiot_handlers::{
    probiot_callback_query_handler, probiot_command_handler, probiot_message_handler,
};
use crate::start_bot;
use core::models::common::app_name::AppName;
use core::models::tg_bot::probiot::probiot_bot_commands::ProbiotBotCommands;
use core::state::tg_bot::app_state::BotAppState;
use std::sync::Arc;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::Update;

pub async fn start_probiot(app_state: Arc<BotAppState>) -> anyhow::Result<()> {
    let command_handler = Update::filter_message()
        .filter_command::<ProbiotBotCommands>()
        .endpoint(probiot_command_handler);

    let message_handler = Update::filter_message().endpoint(probiot_message_handler);

    let callback_query_handler =
        Some(Update::filter_callback_query().endpoint(probiot_callback_query_handler));

    start_bot(
        AppName::Probiot,
        app_state,
        command_handler,
        message_handler,
        callback_query_handler,
    )
    .await
}

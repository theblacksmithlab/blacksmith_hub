use crate::start_bot;
use crate::tester_bot::tester_bot_handlers::{
    tester_bot_command_handler, tester_bot_message_handler, TesterBotCommands,
};
use core::models::common::app_name::AppName;
use core::state::tg_bot::app_state::BotAppState;
use std::sync::Arc;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::Update;

pub async fn start_tester_bot(app_state: Arc<BotAppState>) -> anyhow::Result<()> {
    let command_handler = Update::filter_message()
        .filter_command::<TesterBotCommands>()
        .endpoint(tester_bot_command_handler);

    let message_handler = Update::filter_message().endpoint(tester_bot_message_handler);

    let callback_query_handler = None;

    start_bot(
        AppName::TesterBot,
        app_state,
        command_handler,
        message_handler,
        callback_query_handler,
    )
    .await
}

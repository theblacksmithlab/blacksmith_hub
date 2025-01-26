use crate::request_app_bot::request_app_bot_handlers::{
    request_app_command_handler, request_app_message_handler, RequestAppBotCommands,
};
use crate::start_bot;
use anyhow::Result;
use core::models::common::app_name::AppName;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::tg_bot::tg_bot::check_username;
use std::sync::Arc;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::Update;

pub async fn start_request_app_bot(app_state: Arc<BotAppState>) -> Result<()> {
    let command_handler = Update::filter_message()
        .filter_command::<RequestAppBotCommands>()
        .filter_async(check_username)
        .endpoint(request_app_command_handler);

    let message_handler = Update::filter_message()
        .filter_async(check_username)
        .endpoint(request_app_message_handler);

    start_bot(
        AppName::RequestAppBot,
        app_state,
        command_handler,
        message_handler,
        None,
    )
    .await
}

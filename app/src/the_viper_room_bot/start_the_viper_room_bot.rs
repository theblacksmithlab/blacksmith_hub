use crate::start_bot;
use crate::the_viper_room_bot::the_viper_room_bot_handlers::{
    the_viper_room_command_handler, the_viper_room_message_handler, TheViperRoomBotCommands,
};
use anyhow::Result;
use core::models::common::app_name::AppName;
use core::state::tg_bot::app_state::BotAppState;
use std::sync::Arc;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::Update;

pub async fn start_the_viper_room_bot(app_state: Arc<BotAppState>) -> Result<()> {
    let command_handler = Update::filter_message()
        .filter_command::<TheViperRoomBotCommands>()
        .endpoint(the_viper_room_command_handler);

    let message_handler = Update::filter_message().endpoint(the_viper_room_message_handler);

    start_bot(
        AppName::TheViperRoomBot,
        app_state,
        command_handler,
        message_handler,
        None,
    )
    .await
}

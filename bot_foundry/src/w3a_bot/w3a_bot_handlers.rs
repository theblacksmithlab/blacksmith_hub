// use std::sync::Arc;
use anyhow::Result;
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;
// use tracing::info;
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::W3ABotMessages;
use core::models::tg_bot::w3a_bot::w3a_bot_commands::W3ABotCommands;
use core::utils::common::get_message;

pub async fn w3a_bot_command_handler(bot: Bot, msg: Message, cmd: W3ABotCommands) -> Result<()> {
    let user_id = msg.chat.id;

    match cmd {
        W3ABotCommands::Start => {
            let bot_msg =
                get_message(AppsSystemMessages::W3ABot(W3ABotMessages::StartMessage)).await?;
            bot.send_message(user_id, bot_msg).await?;
        }
    }

    Ok(())
}

// pub async fn w3a_bot_callback_query_handler() -> Result<()> {
//     info!("w3a_bot_callback_query_handler >>>");
//     Ok(())
// }

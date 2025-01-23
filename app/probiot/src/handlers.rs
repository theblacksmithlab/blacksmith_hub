use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use std::sync::Arc;
use teloxide::macros::BotCommands;
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum ProbiotBotCommands {
    Start,
}

pub(crate) async fn message_handler(bot: Bot, msg: Message) -> anyhow::Result<()> {
    let user_id = msg.chat.id;

    // TODO: Переписать логику обработки сообщений пользователя

    let bot_msg = get_message("probiot", "auto_reply", false).await?;
    bot.send_message(user_id, bot_msg).await?;

    Ok(())
}

pub(crate) async fn command_handler(
    bot: Bot,
    msg: Message,
    cmd: ProbiotBotCommands,
    app_state: Arc<BotAppState>,
) -> anyhow::Result<()> {
    // TODO: Прописать логику обработки команд
    let user_id = msg.chat.id;

    match cmd {
        ProbiotBotCommands::Start => {
            let bot_msg = get_message("probiot", "start_message", false).await?;
            bot.send_message(user_id, bot_msg).await?;
        }
    }

    Ok(())
}

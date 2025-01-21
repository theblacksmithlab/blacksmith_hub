use core::utils::common::get_message;
use teloxide::macros::BotCommands;
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum BotCommands {
    Start,
}

pub(crate) async fn command_handler(
    bot: Bot,
    msg: Message,
    cmd: BotCommands,
) -> anyhow::Result<()> {
    let BotCommands::Start = cmd;

    let bot_msg = get_message("request_app", "start_message").await?;
    bot.send_message(msg.chat.id, bot_msg).await?;

    Ok(())
}

pub(crate) async fn message_handler(bot: Bot, msg: Message) -> anyhow::Result<()> {
    let user_id = msg.chat.id;
    let bot_msg = get_message("request_app", "auto_reply").await?;
    bot.send_message(user_id, bot_msg).await?;

    Ok(())
}

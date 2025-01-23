use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use std::sync::Arc;
use teloxide::macros::BotCommands;
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;
use anyhow::Result;
use teloxide::payloads::SendMessageSetters;
use teloxide::types::ReplyParameters;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum ProbiotBotCommands {
    Start,
}

pub(crate) async fn message_handler(bot: Bot, msg: Message) -> Result<()> {
    let chat_id = msg.chat.id;

    let bot_user = bot.get_me().await?.user;
    
    if msg.chat.is_private() {
        let user_raw_request = msg.text().unwrap_or("Empty request").to_string();
        
        let bot_msg = get_message(Some("probiot"), "auto_reply", false).await?;
        bot.send_message(chat_id, bot_msg).await?;
    } else {
        if msg.text().unwrap_or("").contains(&format!("@{}", bot_user.username.unwrap_or_default()))
            || (msg.reply_to_message().is_some()
            && msg
            .reply_to_message()
            .and_then(|reply| reply.from.as_ref())
            .map(|user| user.id == bot_user.id)
            .unwrap_or(false))
        {
            bot.send_message(chat_id, "Пожалуйста, напишите мне в приватный чат.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
            
        }
    }

    Ok(())
}

pub(crate) async fn command_handler(
    bot: Bot,
    msg: Message,
    cmd: ProbiotBotCommands,
    _app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = msg.chat.id;

    match cmd {
        ProbiotBotCommands::Start => {
            let bot_msg = get_message(Some("probiot"), "start_message", false).await?;
            bot.send_message(user_id, bot_msg).await?;
        }
    }

    Ok(())
}

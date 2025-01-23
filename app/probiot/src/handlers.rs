use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use std::sync::Arc;
use teloxide::macros::BotCommands;
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;
use anyhow::Result;
use teloxide::payloads::SendMessageSetters;
use teloxide::types::ReplyParameters;
use tracing::info;
use crate::user_message_processing::process_user_message;
use core::utils::tg_bot::tg_bot::download_voice;
use core::ai::ai::speech_to_text;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum ProbiotBotCommands {
    Start,
}

pub(crate) async fn message_handler(bot: Bot, msg: Message, app_state: Arc<BotAppState>) -> Result<()> {
    let chat_id = msg.chat.id;
    let bot_data = bot.get_me().await?;
    let user_raw_request = msg.text().unwrap_or("Empty request").to_string();
    
    if msg.chat.is_private() {
        if let Some(voice) = msg.voice() {
            let file_path = download_voice(&bot, &voice.file.id, &format!("tmp/{}.ogg", voice.file.id)).await?;
            info!("Passing file path to speech_to_text: {}", file_path);
            let user_voice_transcribed = speech_to_text(&file_path).await?;
            process_user_message(bot.clone(), chat_id, user_voice_transcribed, msg, app_state).await?;
        } else if let Some(text) = msg.text() {
            process_user_message(bot.clone(), chat_id, text.to_string(), msg, app_state).await?;
        } else {
            bot.send_message(chat_id, "Извините, я могу работать только с текстом или голосовыми сообщениями.")
                .await?;
        }
    } else {
        if user_raw_request.contains(&format!("@{}", bot_data.user.clone().username.unwrap_or_default()))
            || (msg.reply_to_message().is_some()
            && msg
            .reply_to_message()
            .and_then(|reply| reply.from.as_ref())
            .map(|user| user.id == bot_data.id)
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

use std::sync::Arc;
use core::utils::common::get_message;
use teloxide::macros::BotCommands;
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;
use teloxide::payloads::SendMessageSetters;
use core::utils::tg_bot::tg_bot::{add_user_message_to_cache, get_cache_as_string, add_llm_response_to_cache};
use core::state::tg_bot::app_state::BotAppState;

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

pub(crate) async fn message_handler(bot: Bot, msg: Message, app_state: Arc<BotAppState>) -> anyhow::Result<()> {
    let user_id = msg.chat.id;

    let user_message = msg.text().unwrap_or_default();
    
    let llm_response = "This is an LLM response".to_string();
    
    add_user_message_to_cache(app_state.clone(), user_id, String::from(user_message)).await;

    let current_cache = get_cache_as_string(app_state.clone(), user_id).await;

    let bot_msg = format!("Текущий кэш:\n{}", current_cache);

    bot.send_message(user_id, bot_msg)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    
    add_llm_response_to_cache(app_state.clone(), user_id, llm_response).await;

    let current_cache_2 = get_cache_as_string(app_state.clone(), user_id).await;

    let bot_msg_2 = format!("Текущий кэш после LLM response:\n{}", current_cache_2);

    bot.send_message(user_id, bot_msg_2)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    
    // let bot_msg = get_message("request_app", "auto_reply").await?;
    // bot.send_message(user_id, bot_msg).await?;

    Ok(())
}

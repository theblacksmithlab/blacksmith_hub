use std::sync::Arc;
use teloxide::types::{ChatId, Message};
use anyhow::Result;
use teloxide::Bot;
use teloxide::prelude::Requester;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::{add_user_message_to_cache, get_cache_as_string};
use core::state::tg_bot::app_state::BotAppState;

pub async fn process_user_message(bot: Bot, chat_id: ChatId, user_request: String, msg: Message, app_state: Arc<BotAppState>) -> Result<()> {
    add_user_message_to_cache(app_state.clone(), chat_id, user_request).await;
    
    let current_cache = get_cache_as_string(app_state, chat_id).await;
    
    let bot_msg = get_message(Some("probiot"), "auto_reply", false).await?;
    
    bot.send_message(chat_id, current_cache).await?;
    
    Ok(())
}
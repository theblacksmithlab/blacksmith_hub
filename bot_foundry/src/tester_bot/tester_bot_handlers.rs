use core::models::common::app_name::AppName;
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::CommonMessages;
use core::models::tg_bot::tester_bot::tester_bot_commands::TesterBotCommands;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::{
    add_llm_response_to_cache, add_user_message_to_cache, get_cache_as_string,
};
use std::sync::Arc;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;

pub(crate) async fn tester_bot_command_handler(
    bot: Bot,
    msg: Message,
    cmd: TesterBotCommands,
) -> anyhow::Result<()> {
    let TesterBotCommands::Start = cmd;

    let bot_msg = get_message(AppsSystemMessages::Common(CommonMessages::StartMessage)).await?;
    bot.send_message(msg.chat.id, bot_msg).await?;

    Ok(())
}

pub(crate) async fn tester_bot_message_handler(
    bot: Bot,
    msg: Message,
    app_state: Arc<BotAppState>,
) -> anyhow::Result<()> {
    let user_id = msg.chat.id;
    let user_id_as_integer = user_id.0;
    let user_id_as_str = user_id_as_integer.to_string();
    let _initiator_app_name = AppName::TesterBot.as_str().to_string();

    let user_message = msg.text().unwrap_or_default();

    let llm_response = "This is an LLM response".to_string();

    add_user_message_to_cache(app_state.clone(), &user_id_as_str, user_message).await;

    let current_cache = get_cache_as_string(app_state.clone(), &user_id_as_str).await;

    let bot_msg = format!("Текущий кэш:\n{}", current_cache);

    bot.send_message(user_id, bot_msg)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    add_llm_response_to_cache(app_state.clone(), &user_id_as_str, &llm_response).await;

    let current_cache_2 = get_cache_as_string(app_state.clone(), &user_id_as_str).await;

    let bot_msg_2 = format!("Текущий кэш после LLM response:\n{}", current_cache_2);

    bot.send_message(user_id, bot_msg_2)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    let bot_msg = get_message(AppsSystemMessages::Common(CommonMessages::AutoReply)).await?;
    bot.send_message(user_id, bot_msg).await?;

    Ok(())
}

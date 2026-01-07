use crate::stat_bot::stat_bot_utils::{check_admin_access, create_stats_keyboard};
use crate::stat_bot::statistics_processing::{handle_export_requests, handle_stats_request};
use anyhow::Result;
use core::models::common::app_name::AppName;
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::{CommonMessages, StatBotMessages};
use core::models::tg_bot::stat_bot::StatBotCommands;
use core::state::tg_bot::StatBotState;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::{
    check_username_from_user, get_chat_title, get_username_from_user,
};
use std::sync::Arc;
use teloxide::prelude::{Message, Requester};
use teloxide::types::CallbackQuery;
use teloxide::Bot;
use teloxide_core::payloads::SendMessageSetters;
use teloxide_core::types::ParseMode;
use tracing::info;

pub async fn stat_bot_command_handler(
    bot: Bot,
    msg: Message,
    cmd: StatBotCommands,
    _app_state: Arc<StatBotState>,
) -> Result<()> {
    let chat_id = msg.chat.id;
    let user = msg
        .from
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let user_id = user.id.0;
    let username = get_username_from_user(&user);

    if !msg.chat.is_private() {
        let chat_title = get_chat_title(&msg);

        info!(
            "User: {} [{}] tried to execute {:?} cmd in public chat: {} [{}]",
            username, user_id, cmd, chat_title, chat_id
        );
        let bot_msg = get_message(AppsSystemMessages::Common(
            CommonMessages::PrivateCmdUsedInPublicChat,
        ))
        .await?;
        bot.send_message(chat_id, bot_msg).await?;
        return Ok(());
    }

    if check_username_from_user(&bot, &user, chat_id).await == false {
        return Ok(());
    }

    let accessible_apps = check_admin_access(user_id);

    if accessible_apps.is_empty() {
        info!(
            "User: {} [{}] tried to use StatBot without admin rights",
            username, user_id
        );

        let bot_system_message = get_message(AppsSystemMessages::Common(
            CommonMessages::NoRightsForAppUse,
        ))
        .await?;

        bot.send_message(chat_id, bot_system_message).await?;

        return Ok(());
    }

    match cmd {
        StatBotCommands::Start => {
            let app_names: Vec<String> = accessible_apps
                .iter()
                .map(|app| match app {
                    AppName::BlacksmithWeb => "Blacksmith Web".to_string(),
                    AppName::W3AWeb => "W3A Web".to_string(),
                    _ => String::new(),
                })
                .filter(|s| !s.is_empty())
                .collect();

            let apps = app_names
                .iter()
                .map(|name| format!("• {}", name))
                .collect::<Vec<_>>()
                .join("\n");

            let welcome_message_template =
                get_message(AppsSystemMessages::StatBot(StatBotMessages::StartMessage)).await?;
            let welcome_message = welcome_message_template.replace("{apps}", &apps);

            let keyboard = create_stats_keyboard(&accessible_apps);

            bot.send_message(chat_id, welcome_message)
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
        }
    }

    Ok(())
}

pub async fn stat_bot_message_handler(
    bot: Bot,
    msg: Message,
    _app_state: Arc<StatBotState>,
) -> Result<()> {
    let chat_id = msg.chat.id;

    if let Some(text) = msg.text() {
        info!("Received message from user: {}", text);

        let bot_system_message = get_message(AppsSystemMessages::StatBot(
            StatBotMessages::TextMessageHandling,
        ))
        .await?;

        bot.send_message(chat_id, bot_system_message).await?;
    }

    Ok(())
}

pub async fn stat_bot_callback_query_handler(
    bot: Bot,
    q: CallbackQuery,
    app_state: Arc<StatBotState>,
) -> Result<()> {
    let user = &q.from;
    let user_id = user.id.0;
    let username = get_username_from_user(user);

    let chat_id = match &q.message {
        Some(msg) => msg.chat().id,
        None => {
            info!("Callback query without message");
            return Ok(());
        }
    };

    let accessible_apps = check_admin_access(user_id);

    if accessible_apps.is_empty() {
        bot.answer_callback_query(&q.id).await?;

        let bot_system_message = get_message(AppsSystemMessages::Common(
            CommonMessages::NoRightsForAppUse,
        ))
        .await?;

        bot.send_message(chat_id, bot_system_message).await?;

        return Ok(());
    }

    if let Some(data) = q.data.clone() {
        info!(
            "User: {} [{}] triggered callback: {}",
            username, user_id, data
        );

        let parts: Vec<&str> = data.split(':').collect();

        match parts.as_slice() {
            ["stats", app_code, period] => {
                handle_stats_request(&bot, &q, app_state, *app_code, *period, &accessible_apps)
                    .await?;
            }
            ["export", app_code, "requests"] => {
                handle_export_requests(&bot, &q, app_state, *app_code, &accessible_apps).await?;
            }
            _ => {
                bot.answer_callback_query(&q.id).await?;
                bot.send_message(chat_id, "❌ Неизвестная команда").await?;
            }
        }
    }

    Ok(())
}

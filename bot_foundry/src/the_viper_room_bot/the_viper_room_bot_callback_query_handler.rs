use crate::the_viper_room_bot::the_viper_room_bot_utils::{
    send_add_channel_prompt, send_channels_menu, send_delete_channel_prompt, send_main_menu,
    send_settings_menu, show_user_channels, MainMenuMessageType,
};
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::TheViperRoomBotMessages;
use core::models::tg_bot::the_viper_room_bot::the_viper_room_bot_user_state::TheViperRoomBotUserState;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::{check_username_from_user, get_username_from_user};
use std::sync::Arc;
use teloxide::prelude::Requester;
use teloxide::Bot;
use teloxide_core::payloads::SendMessageSetters;
use teloxide_core::types::{CallbackQuery, ParseMode, UserId};
use tracing::info;
use tracing::log::warn;

pub(crate) async fn the_viper_room_bor_callback_query_handler(
    bot: Bot,
    q: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> anyhow::Result<()> {
    let user = &q.from;
    let chat_id = match &q.message {
        Some(msg) => msg.chat().id,
        None => {
            warn!("Callback query without message");
            return Ok(());
        }
    };
    if check_username_from_user(&bot, user, chat_id).await == false {
        return Ok(());
    }
    let username = get_username_from_user(user);
    let user_id = UserId(user.id.0);
    let callback_query_message = q.message.as_ref().unwrap().id();

    if let Some(data) = q.data.clone() {
        info!(
            "User: {} [{}] executed callback query with inline query: '{}'",
            username, user_id, data
        );
    } else {
        warn!(
            "User: {} [{}] executed callback query without inline query",
            username, user_id
        );
    }

    match q.data.as_deref() {
        Some("back_to_main_menu") => {
            send_main_menu(
                &bot,
                user_id,
                chat_id,
                &app_state,
                MainMenuMessageType::Minimal,
            )
            .await?;

            if let Err(e) = bot.delete_message(chat_id, callback_query_message).await {
                warn!("Failed to delete query origin message: {}", e);
            }

            bot.answer_callback_query(q.id).await?;
        }
        Some("settings_my_channels") => {
            send_channels_menu(&bot, user_id, chat_id, &app_state).await?;

            if let Err(e) = bot.delete_message(chat_id, callback_query_message).await {
                warn!("Failed to delete query origin message: {}", e);
            }

            bot.answer_callback_query(q.id).await?;
        }
        Some("channels_show_list") => {
            show_user_channels(&bot, user_id, chat_id, &app_state).await?;

            if let Err(e) = bot.delete_message(chat_id, callback_query_message).await {
                warn!("Failed to delete query origin message: {}", e);
            }

            bot.answer_callback_query(q.id).await?;
        }
        Some("back_to_channels_menu") => {
            send_channels_menu(&bot, user_id, chat_id, &app_state).await?;

            if let Err(e) = bot.delete_message(chat_id, callback_query_message).await {
                warn!("Failed to delete query origin message: {}", e);
            }

            bot.answer_callback_query(q.id).await?;
        }
        Some("back_to_settings") => {
            send_settings_menu(&bot, user_id, chat_id, &app_state).await?;

            if let Err(e) = bot.delete_message(chat_id, callback_query_message).await {
                warn!("Failed to delete query origin message: {}", e);
            }

            bot.answer_callback_query(q.id).await?;
        }
        Some("channels_add") => {
            send_add_channel_prompt(&bot, user_id, chat_id, &app_state).await?;

            if let Err(e) = bot.delete_message(chat_id, callback_query_message).await {
                warn!("Failed to delete query origin message: {}", e);
            }

            bot.answer_callback_query(q.id).await?;
        }
        Some("channels_delete") => {
            send_delete_channel_prompt(&bot, user_id, chat_id, &app_state).await?;

            if let Err(e) = bot.delete_message(chat_id, callback_query_message).await {
                warn!("Failed to delete query origin message: {}", e);
            }

            bot.answer_callback_query(q.id).await?;
        }
        Some("settings_podcast_time") => {
            if let Some(states) = &app_state.the_viper_room_bot_user_states {
                let mut states_lock = states.lock().await;
                states_lock.insert(user_id.0, TheViperRoomBotUserState::PodcastTimeMenuView);
            }

            // TODO: Implement podcast time configuration
            let temp_msg = "⏰ Время отправки подкаста\n\nНастройка времени отправки в разработке.";
            bot.send_message(chat_id, temp_msg).await?;

            if let Err(e) = bot.delete_message(chat_id, callback_query_message).await {
                warn!("Failed to delete query origin message: {}", e);
            }

            bot.answer_callback_query(q.id).await?;
        }
        Some("FAQ") => {
            let faq_text = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::FAQ,
            ))
            .await?;
            bot.send_message(chat_id, faq_text)
                .parse_mode(ParseMode::Html)
                .await?;

            if let Err(e) = bot.delete_message(chat_id, callback_query_message).await {
                warn!("Failed to delete query origin message: {}", e);
            }

            bot.answer_callback_query(q.id).await?;

            send_main_menu(
                &bot,
                user_id,
                chat_id,
                &app_state,
                MainMenuMessageType::Minimal,
            )
            .await?;
        }
        _ => {}
    }

    Ok(())
}

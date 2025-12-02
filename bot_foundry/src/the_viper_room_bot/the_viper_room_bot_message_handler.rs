use crate::the_viper_room_bot::the_viper_room_bot_utils::{
    parse_channel_input, send_actual_daily_public_podcast, send_channels_menu, send_main_menu,
    send_settings_menu, ChannelInput, MainMenuMessageType,
};
use anyhow::Result;
use core::local_db::the_viper_room::channel_management;
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::TheViperRoomBotMessages;
use core::models::tg_bot::the_viper_room_bot::the_viper_room_bot_user_state::TheViperRoomBotUserState;
use core::models::the_viper_room::db_models::PendingChannel;
use core::state::tg_bot::app_state::BotAppState;
use core::telegram_client::grammers_functionality::initialize_grammers_client;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::auto_delete_messages_batch;
use core::utils::tg_bot::tg_bot::{
    check_username_from_message, get_chat_title, get_username_from_message, is_bot_addressed,
};
use grammers_client::types::Chat;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};
use teloxide::prelude::{Message, Requester};
use teloxide::sugar::request::RequestReplyExt;
use teloxide::Bot;
use teloxide_core::payloads::SendMessageSetters;
use teloxide_core::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup, UserId,
};
use tracing::info;
use tracing::log::warn;

const MAX_CHANNELS_PER_USER: usize = 10;

pub(crate) async fn the_viper_room_message_handler(
    bot: Bot,
    msg: Message,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    if check_username_from_message(&bot, &msg).await == false {
        return Ok(());
    }
    let username = get_username_from_message(&msg);
    let chat_id = msg.chat.id;
    let user = msg
        .from
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let user_id = UserId(user.id.0);
    let chat_title = get_chat_title(&msg);

    if !msg.chat.is_private() {
        let is_bot_mentioned = is_bot_addressed(&bot, &msg).await?;

        if is_bot_mentioned {
            info!(
                "User: {} [{}] addressed bot in public chat: {} [{}]",
                username, user_id, chat_title, chat_id
            );

            let bot_msg = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::PublicChatMessageCommunication,
            ))
            .await?;

            bot.send_message(chat_id, bot_msg).reply_to(msg.id).await?;
            return Ok(());
        }
        return Ok(());
    }

    let current_state = if let Some(states) = &app_state.the_viper_room_bot_user_states {
        let states_lock = states.lock().await;
        states_lock
            .get(&user_id.0)
            .cloned()
            .unwrap_or(TheViperRoomBotUserState::Idle)
    } else {
        TheViperRoomBotUserState::Idle
    };

    if current_state.is_in_settings() && !current_state.expects_text_input() {
        if msg.text().is_some() {
            let warning_msg = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::SettingsMenuUnexpectedMessage,
            ))
            .await?;
            bot.send_message(chat_id, warning_msg).await?;
            return Ok(());
        }
    }

    // Handle channel adding state (supports messages without text, e.g. forwarded media)
    if matches!(current_state, TheViperRoomBotUserState::ChannelsAdding) {
        // Check if this is a button press (Save or Exit)
        if let Some(text) = msg.text() {
            if text == "💾 Сохранить" || text == "🏠 Главное меню" {
                // Let it fall through to the main match statement below
            } else {
                // Try to parse as channel input (text username or forwarded message)
                match parse_channel_input(&msg) {
                    Ok(ChannelInput::Forwarded(channel_id, channel_title)) => {
                        if let Some(pending) = &app_state.the_viper_room_bot_pending_channels {
                            let mut pending_lock = pending.lock().await;
                            let user_channels =
                                pending_lock.entry(user_id.0).or_insert_with(Vec::new);
                            user_channels.push(PendingChannel {
                                channel_id,
                                channel_title: channel_title.clone(),
                            });
                        }

                        bot.send_message(
                            chat_id,
                            format!("✅ Канал \"{}\" добавлен", channel_title),
                        )
                        .await?;

                        return Ok(());
                    }
                    Ok(ChannelInput::Usernames(usernames)) => {
                        let tg_agent_id = Arc::new(
                            env::var("TG_AGENT_ID")
                                .expect("TG_AGENT_ID must be set in environment"),
                        );
                        let session_path = format!(
                            "common_res/the_viper_room/grammers_system_session/{}.session",
                            tg_agent_id
                        );

                        if !Path::new(&session_path).exists() {
                            bot.send_message(chat_id, "❌ Ошибка: сессия Telegram не найдена")
                                .await?;
                            return Ok(());
                        }

                        let session_data = fs::read(Path::new(&session_path))?;
                        let g_client = initialize_grammers_client(session_data).await?;

                        let mut added_count = 0;
                        let mut error_count = 0;

                        for username in usernames {
                            match g_client.resolve_username(&username).await {
                                Ok(Some(chat)) => {
                                    if let Chat::Channel(channel) = chat {
                                        let channel_id = channel.id();
                                        let channel_title = channel.title().to_string();

                                        // Add to pending storage
                                        if let Some(pending) =
                                            &app_state.the_viper_room_bot_pending_channels
                                        {
                                            let mut pending_lock = pending.lock().await;
                                            let user_channels = pending_lock
                                                .entry(user_id.0)
                                                .or_insert_with(Vec::new);
                                            user_channels.push(PendingChannel {
                                                channel_id,
                                                channel_title,
                                            });
                                        }

                                        added_count += 1;
                                    } else {
                                        warn!("Username {} is not a channel", username);
                                        error_count += 1;
                                    }
                                }
                                Ok(None) => {
                                    warn!("Username {} not found", username);
                                    error_count += 1;
                                }
                                Err(e) => {
                                    warn!("Failed to resolve username {}: {}", username, e);
                                    error_count += 1;
                                }
                            }
                        }

                        let result_msg = if error_count == 0 {
                            format!("✅ Добавлено каналов: {}", added_count)
                        } else if added_count == 0 {
                            "❌ Не удалось добавить каналы. Проверьте правильность имён."
                                .to_string()
                        } else {
                            format!("✅ Добавлено: {}\n❌ Ошибок: {}", added_count, error_count)
                        };

                        bot.send_message(chat_id, result_msg).await?;

                        return Ok(());
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("❌ {}", e)).await?;
                        return Ok(());
                    }
                }
            }
        } else {
            // No text - try to parse forwarded message (e.g., forwarded media)
            match parse_channel_input(&msg) {
                Ok(ChannelInput::Forwarded(channel_id, channel_title)) => {
                    if let Some(pending) = &app_state.the_viper_room_bot_pending_channels {
                        let mut pending_lock = pending.lock().await;
                        let user_channels = pending_lock.entry(user_id.0).or_insert_with(Vec::new);
                        user_channels.push(PendingChannel {
                            channel_id,
                            channel_title: channel_title.clone(),
                        });
                    }

                    bot.send_message(chat_id, format!("✅ Канал \"{}\" добавлен", channel_title))
                        .await?;

                    return Ok(());
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("❌ {}", e)).await?;
                    return Ok(());
                }
                _ => {
                    // Unexpected case
                    bot.send_message(chat_id, "❌ Неподдерживаемый тип сообщения")
                        .await?;
                    return Ok(());
                }
            }
        }
    }

    let msg_text = match msg.text() {
        Some(t) => t,
        None => return Ok(()),
    };

    match msg_text {
        "🏠 Главное меню" => {
            if matches!(current_state, TheViperRoomBotUserState::ChannelsAdding) {
                if let Some(pending) = &app_state.the_viper_room_bot_pending_channels {
                    let mut pending_lock = pending.lock().await;
                    pending_lock.remove(&user_id.0);
                }
            }
            send_main_menu(
                &bot,
                user_id,
                chat_id,
                &app_state,
                MainMenuMessageType::Full,
            )
            .await?;
            Ok(())
        }
        "💾 Сохранить" if matches!(current_state, TheViperRoomBotUserState::ChannelsAdding) =>
        {
            let channels_to_add =
                if let Some(pending) = &app_state.the_viper_room_bot_pending_channels {
                    let pending_lock = pending.lock().await;
                    pending_lock.get(&user_id.0).cloned().unwrap_or_default()
                } else {
                    Vec::new()
                };

            if channels_to_add.is_empty() {
                bot.send_message(chat_id, "❌ Нет каналов для сохранения")
                    .await?;

                let keyboard = KeyboardMarkup::new(vec![
                    vec![KeyboardButton::new("💾 Сохранить")],
                    vec![KeyboardButton::new("🏠 Главное меню")],
                ])
                .resize_keyboard()
                .one_time_keyboard();

                bot.send_message(
                    chat_id,
                    "Отправьте username канала или перешлите пост из канала",
                )
                .reply_markup(keyboard)
                .await?;

                return Ok(());
            }

            let db_pool = match &app_state.db_pool {
                Some(pool) => pool,
                None => {
                    bot.send_message(chat_id, "Ошибка: база данных недоступна")
                        .await?;
                    return Ok(());
                }
            };

            let user_id_i64 = user_id.0 as i64;

            let current_channels =
                channel_management::get_user_channels(db_pool.as_ref(), user_id_i64).await?;
            let current_count = current_channels.len();

            let available_slots = MAX_CHANNELS_PER_USER.saturating_sub(current_count);

            if available_slots == 0 {
                bot.send_message(
                    chat_id,
                    format!("❌ Достигнут лимит каналов ({}).\n\nУдалите старые каналы, чтобы добавить новые.", MAX_CHANNELS_PER_USER)
                ).await?;

                if let Some(pending) = &app_state.the_viper_room_bot_pending_channels {
                    let mut pending_lock = pending.lock().await;
                    pending_lock.remove(&user_id.0);
                }

                send_channels_menu(&bot, user_id, chat_id, &app_state).await?;
                return Ok(());
            }

            let channels_to_save: Vec<_> = channels_to_add.iter().take(available_slots).collect();
            let skipped_count = channels_to_add.len() - channels_to_save.len();

            let mut saved_count = 0;
            let mut error_count = 0;

            for channel in channels_to_save {
                match channel_management::add_channel(
                    db_pool.as_ref(),
                    user_id_i64,
                    channel.channel_id,
                    &channel.channel_title,
                )
                .await
                {
                    Ok(_) => saved_count += 1,
                    Err(e) => {
                        warn!("Failed to add channel {}: {}", channel.channel_id, e);
                        error_count += 1;
                    }
                }
            }

            if let Some(pending) = &app_state.the_viper_room_bot_pending_channels {
                let mut pending_lock = pending.lock().await;
                pending_lock.remove(&user_id.0);
            }

            let result_msg = if skipped_count > 0 {
                format!(
                    "⚠️ Достигнут лимит каналов (максимум {}).\n\n✅ Добавлено каналов: {}\n❌ Не добавлено: {}\n\nДля освобождения слотов воспользуйтесь пунктом '➖ Удалить канал' в меню управления каналами.",
                    MAX_CHANNELS_PER_USER, saved_count, skipped_count
                )
            } else if error_count == 0 {
                format!("✅ Сохранено каналов: {}", saved_count)
            } else {
                format!("✅ Сохранено: {}\n❌ Ошибок: {}", saved_count, error_count)
            };

            bot.send_message(chat_id, result_msg).await?;

            send_channels_menu(&bot, user_id, chat_id, &app_state).await?;
            Ok(())
        }
        "❓ Задать вопрос" => {
            let support_chat = env::var("SUPPORT_CHAT_URL")
                .unwrap_or_else(|_| "https://t.me/the_viper_room_chat".to_string());

            let keyboard = InlineKeyboardMarkup::new(vec![
                vec![InlineKeyboardButton::url(
                    "💬 Чат поддержки",
                    support_chat.parse()?,
                )],
                vec![InlineKeyboardButton::callback("📖 FAQ", "FAQ")],
                vec![InlineKeyboardButton::callback(
                    "« Назад в Главное меню",
                    "back_to_main_menu",
                )],
            ]);

            let bot_system_message = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::SupportMessage,
            ))
            .await?;

            bot.send_message(chat_id, bot_system_message)
                .reply_markup(keyboard)
                .await?;
            Ok(())
        }
        "🎙 Сегодняшний подкаст" => {
            let bot_system_message = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::PublicPodcastSendingIntroMessage,
            ))
            .await?;
            bot.send_message(chat_id, bot_system_message).await?;
            send_actual_daily_public_podcast(bot.clone(), chat_id).await?;
            send_main_menu(
                &bot,
                user_id,
                chat_id,
                &app_state,
                MainMenuMessageType::Minimal,
            )
            .await?;
            Ok(())
        }
        "🎧 Персональный подкаст" => {
            let temp_message = "Этот функционал пока в разработке".to_string();
            bot.send_message(chat_id, temp_message).await?;
            send_main_menu(
                &bot,
                user_id,
                chat_id,
                &app_state,
                MainMenuMessageType::Minimal,
            )
            .await?;
            Ok(())
        }
        "⚙️ Настройки" => {
            send_settings_menu(&bot, user_id, chat_id, &app_state).await?;
            Ok(())
        }
        _ => {
            let bot_system_message = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::UnexpectedMessage,
            ))
            .await?;

            let sent_system_message = bot
                .send_message(chat_id, bot_system_message)
                .reply_to(msg.id)
                .await?;

            let messages_to_delete = vec![(chat_id, msg.id), (chat_id, sent_system_message.id)];

            auto_delete_messages_batch(
                bot.clone(),
                messages_to_delete,
                Some(Duration::from_secs(10)),
            )
            .await;

            Ok(())
        }
    }
}

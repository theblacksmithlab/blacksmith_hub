use crate::the_viper_room_bot::the_viper_room_bot_utils::{
    parse_channel_input, send_channels_menu, send_daily_podcast, send_main_menu,
    send_settings_menu, ChannelInput,
};
use anyhow::Result;
use blacksmith_core::local_db::the_viper_room::channel_management;
use blacksmith_core::local_db::the_viper_room::channel_management::{
    clear_user_channels, get_channel, get_user_channels, remove_channel,
};
use blacksmith_core::models::common::system_messages::AppsSystemMessages;
use blacksmith_core::models::common::system_messages::TheViperRoomBotMessages;
use blacksmith_core::models::tg_bot::the_viper_room_bot::the_viper_room_bot_user_state::TheViperRoomBotUserState;
use blacksmith_core::models::the_viper_room::db_models::PendingChannel;
use blacksmith_core::models::the_viper_room::db_models::Recipient;
use blacksmith_core::models::the_viper_room::the_viper_room_bot::{normalize_channel_id, MainMenuMessageType};
use blacksmith_core::state::tg_bot::TheViperRoomBotState;
use blacksmith_core::telegram_client::grammers_functionality::initialize_grammers_client;
use blacksmith_core::utils::common::get_message;
use blacksmith_core::utils::tg_bot::tg_bot::auto_delete_messages_batch;
use blacksmith_core::utils::tg_bot::tg_bot::{
    check_username_from_user, get_chat_title, get_username_from_message, is_bot_addressed,
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
    InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup, ParseMode,
};
use tracing::info;
use tracing::log::warn;

const MAX_CHANNELS_PER_USER: usize = 10;

pub(crate) async fn the_viper_room_message_handler(
    bot: Bot,
    msg: Message,
    app_state: Arc<TheViperRoomBotState>,
) -> Result<()> {
    let username = get_username_from_message(&msg);
    let chat_id = msg.chat.id;
    let user = msg
        .from
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let user_id = user.id.0;
    let chat_title = get_chat_title(&msg);

    if check_username_from_user(&bot, &user, chat_id).await == false {
        return Ok(());
    }

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

    let current_state = {
        let states_lock = app_state.user_states.lock().await;
        states_lock
            .get(&user_id)
            .cloned()
            .unwrap_or(TheViperRoomBotUserState::Idle)
    };

    if current_state.is_in_settings() && !current_state.expects_text_input() {
        if msg.text().is_some() {
            let warning_msg = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::SettingsMenuUnexpectedMessage,
            ))
            .await?;
            let sent_system_message = bot.send_message(chat_id, warning_msg).await?;
            let messages_to_delete = vec![(chat_id, msg.id), (chat_id, sent_system_message.id)];
            auto_delete_messages_batch(
                bot.clone(),
                messages_to_delete,
                Some(Duration::from_secs(20)),
            )
            .await;

            return Ok(());
        }
    }

    if matches!(current_state, TheViperRoomBotUserState::ChannelsAdding) {
        if let Some(text) = msg.text() {
            if text == "💾 Сохранить" || text == "🏠 Главное меню" || text == "⚙️ Настройки"
            {
                // Let it fall through to the main match statement below
            } else {
                match parse_channel_input(&msg) {
                    Ok(ChannelInput::Forwarded(channel_id, channel_title, channel_username)) => {
                        {
                            let mut pending_lock = app_state.pending_channels.lock().await;
                            let user_channels =
                                pending_lock.entry(user_id).or_insert_with(Vec::new);
                            user_channels.push(PendingChannel {
                                channel_id,
                                channel_title: channel_title.clone(),
                                channel_username,
                            });
                        }

                        bot.send_message(
                            chat_id,
                            format!(
                                "✅ Принят канал: \"{}\"
                            \nНО ПОКА НЕ СОХРАНЁН!\
                            \n\nНажми кнопку \"Сохранить\" в нижнем меню\
                            \nили добавь ещё интересующие каналы",
                                channel_title
                            ),
                        )
                        .await?;

                        return Ok(());
                    }
                    Ok(ChannelInput::Usernames(usernames, invalid_inputs)) => {
                        let tg_agent_id = Arc::new(
                            env::var("TG_AGENT_ID")
                                .expect("TG_AGENT_ID must be set in environment"),
                        );
                        let session_path = format!(
                            "common_res/the_viper_room/grammers_system_session/{}.session",
                            tg_agent_id
                        );

                        if !Path::new(&session_path).exists() {
                            bot.send_message(chat_id, "❌ Ошибка: Наш Telegram-агент в данный момент спит и не готов обрабатывать данные\n\n\
                            Попробуй повторить попытку позднее и спасибо за понимание 🙏")
                                .await?;

                            send_main_menu(
                                &bot,
                                user_id,
                                chat_id,
                                &app_state,
                                MainMenuMessageType::Full,
                            )
                            .await?;

                            return Ok(());
                        }

                        let session_data = fs::read(Path::new(&session_path))?;
                        let g_client = initialize_grammers_client(session_data).await?;

                        let mut accepted_channels = Vec::new();
                        let mut errors: Vec<String> = Vec::new();

                        for username in usernames {
                            match g_client.resolve_username(&username).await {
                                Ok(Some(chat)) => {
                                    if let Chat::Channel(channel) = chat {
                                        let channel_id = normalize_channel_id(channel.id());
                                        let channel_title = channel.title().to_string();

                                        {
                                            let mut pending_lock =
                                                app_state.pending_channels.lock().await;
                                            let user_channels = pending_lock
                                                .entry(user_id)
                                                .or_insert_with(Vec::new);
                                            user_channels.push(PendingChannel {
                                                channel_id,
                                                channel_title: channel_title.clone(),
                                                channel_username: username.clone(),
                                            });
                                        }

                                        accepted_channels.push((channel_title, username.clone()));
                                    } else if let Chat::Group(_) = chat {
                                        warn!("Username '@{}' is a group, not a channel", username);
                                        errors.push(format!(
                                            "⛔️ Извини, но кажется @{} - это группа, а не канал",
                                            username
                                        ));
                                    } else {
                                        warn!("Username '@{}' is not a channel (some person's username provided)", username);
                                        errors.push(format!(
                                            "⛔️ Извини, но @{} не является каналом, похоже, что это пользователь",
                                            username
                                        ));
                                    }
                                }
                                Ok(None) => {
                                    warn!("Username '@{}' not found", username);
                                    errors.push(format!(
                                        "❌ Канал не найден по юзернейму: @{}",
                                        username
                                    ));
                                }
                                Err(e) => {
                                    warn!("Failed to resolve username '@{}': {}", username, e);
                                    errors.push(format!(
                                        "❌ Ошибка при проверке юзернейма: @{}",
                                        username
                                    ));
                                }
                            }
                        }

                        let mut result_parts = Vec::new();

                        if !accepted_channels.is_empty() {
                            let channels_list = accepted_channels
                                .iter()
                                .map(|(title, username)| {
                                    format!("  • \"{}\" (@{})", title, username)
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            result_parts.push(format!(
                                "✅ Принято каналов ({}):\n{}",
                                accepted_channels.len(),
                                channels_list
                            ));
                        }

                        if !errors.is_empty() {
                            result_parts.push(format!("❌ Не принято:\n{}", errors.join("\n")));
                        }

                        if !invalid_inputs.is_empty() {
                            let invalid_list = invalid_inputs
                                .iter()
                                .map(|s| format!("{} (отсутствует @)", s))
                                .collect::<Vec<_>>()
                                .join(", ");
                            result_parts.push(format!("⚠️ Пропущены:\n{}", invalid_list));
                        }

                        let result_msg = if result_parts.is_empty() {
                            "❌ Не удалось обработать каналы\n\nПопробуй повторить попытку, следуя инструкции по загрузке каналов, которые я предоставил ранее".to_string()
                        } else {
                            let result_footer =
                                "Нажми кнопку \"Сохранить\" в нижнем меню чтобы добавить принятые каналы в твою базу данных"
                                    .to_string();
                            result_parts.push(result_footer);
                            result_parts.join("\n\n")
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
            match parse_channel_input(&msg) {
                Ok(ChannelInput::Forwarded(channel_id, channel_title, channel_username)) => {
                    {
                        let mut pending_lock = app_state.pending_channels.lock().await;
                        let user_channels = pending_lock.entry(user_id).or_insert_with(Vec::new);
                        user_channels.push(PendingChannel {
                            channel_id,
                            channel_title: channel_title.clone(),
                            channel_username,
                        });
                    }

                    bot.send_message(
                        chat_id,
                        format!(
                            "✅ Принят канал: \"{}\"
                            \nНО ПОКА НЕ СОХРАНЁН!\
                            \n\nНажми кнопку \"Сохранить\" в нижнем меню\
                            \nили добавь ещё интересующие каналы",
                            channel_title
                        ),
                    )
                    .await?;

                    return Ok(());
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("❌ {}", e)).await?;
                    return Ok(());
                }
                _ => {
                    bot.send_message(chat_id, "❌ Неподдерживаемый тип сообщения")
                        .await?;
                    return Ok(());
                }
            }
        }
    }

    if matches!(current_state, TheViperRoomBotUserState::ChannelsDeleting) {
        if let Some(text) = msg.text() {
            if text == "🏠 Главное меню" || text == "⚙️ Настройки" {
                // Let it fall through to the main match statement below
            } else if text == "🗑 Удалить все каналы" {
                let db_pool = match &app_state.core.db_pool {
                    Some(pool) => pool,
                    None => {
                        let local_db_unavailable_message =
                            get_message(AppsSystemMessages::TheViperRoomBot(
                                TheViperRoomBotMessages::LocalDBUnavailable,
                            ))
                            .await?;

                        bot.send_message(chat_id, local_db_unavailable_message)
                            .await?;

                        send_channels_menu(&bot, user_id, chat_id, &app_state).await?;

                        return Ok(());
                    }
                };

                let channels = get_user_channels(db_pool.as_ref(), user_id).await?;
                let channels_count = channels.len();

                if channels_count == 0 {
                    bot.send_message(chat_id, "Твой список каналов пуст, нечего удалять 🤷‍♂️")
                        .await?;
                } else {
                    clear_user_channels(db_pool.as_ref(), user_id).await?;

                    bot.send_message(
                        chat_id,
                        format!("✅ Все каналы ({}) успешно удалены", channels_count),
                    )
                    .await?;
                }
                send_channels_menu(&bot, user_id, chat_id, &app_state).await?;
                return Ok(());
            } else {
                let channel_id = match text.trim().parse::<i64>() {
                    Ok(id) => id,
                    Err(_) => {
                        bot.send_message(
                            chat_id,
                            "❌ Неверный формат ID канала. ID должен быть числом.\n\nПопробуй снова или нажми \"🏠 Главное меню\"",
                        )
                        .await?;
                        return Ok(());
                    }
                };

                let db_pool = match &app_state.core.db_pool {
                    Some(pool) => pool,
                    None => {
                        let local_db_unavailable_message =
                            get_message(AppsSystemMessages::TheViperRoomBot(
                                TheViperRoomBotMessages::LocalDBUnavailable,
                            ))
                            .await?;

                        bot.send_message(chat_id, local_db_unavailable_message)
                            .await?;

                        send_channels_menu(&bot, user_id, chat_id, &app_state).await?;
                        return Ok(());
                    }
                };

                match get_channel(db_pool.as_ref(), user_id, channel_id).await? {
                    Some(channel) => {
                        remove_channel(db_pool.as_ref(), user_id, channel_id).await?;

                        bot.send_message(
                            chat_id,
                            format!(
                                "🗑️ Канал \"{}\" (ID: {}) успешно удалён",
                                channel.channel_title, channel_id
                            ),
                        )
                        .await?;

                        send_channels_menu(&bot, user_id, chat_id, &app_state).await?;
                        return Ok(());
                    }
                    None => {
                        bot.send_message(
                            chat_id,
                            format!(
                                "❌ Канал с ID {} не найден в твоём списке\n\nПроверь ID и попробуй снова",
                                channel_id
                            ),
                        )
                        .await?;

                        send_channels_menu(&bot, user_id, chat_id, &app_state).await?;
                        return Ok(());
                    }
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
                {
                    let mut pending_lock = app_state.pending_channels.lock().await;
                    pending_lock.remove(&user_id);
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
            let channels_to_add = {
                let pending_lock = app_state.pending_channels.lock().await;
                pending_lock.get(&user_id).cloned().unwrap_or_default()
            };

            if channels_to_add.is_empty() {
                bot.send_message(chat_id, "❌ Похоже ты не предоставил ни одного валидного канала для сохранения\n\
                Попробуй повторить попытку, следуя инструкции по загрузке каналов, которые я предоставил ранее")
                    .await?;

                let keyboard = KeyboardMarkup::new(vec![
                    vec![KeyboardButton::new("💾 Сохранить")],
                    vec![KeyboardButton::new("⚙️ Настройки")],
                    vec![KeyboardButton::new("🏠 Главное меню")],
                ])
                .resize_keyboard()
                .one_time_keyboard();

                bot.send_message(
                    chat_id,
                    "Отправь username канала (@channelname), ссылку (https://t.me/channelname) или перешли пост из канала",
                )
                .reply_markup(keyboard)
                .await?;

                return Ok(());
            }

            let db_pool = match &app_state.core.db_pool {
                Some(pool) => pool,
                None => {
                    let local_db_unavailable_message =
                        get_message(AppsSystemMessages::TheViperRoomBot(
                            TheViperRoomBotMessages::LocalDBUnavailable,
                        ))
                        .await?;

                    bot.send_message(chat_id, local_db_unavailable_message)
                        .await?;

                    send_main_menu(
                        &bot,
                        user_id,
                        chat_id,
                        &app_state,
                        MainMenuMessageType::Full,
                    )
                    .await?;

                    return Ok(());
                }
            };

            let current_channels = get_user_channels(db_pool.as_ref(), user_id).await?;
            let current_count = current_channels.len();

            let available_slots = MAX_CHANNELS_PER_USER.saturating_sub(current_count);

            if available_slots == 0 {
                bot.send_message(
                    chat_id,
                    format!("❌ Достигнут лимит каналов ({}).\n\nДля освобождения слотов воспользуйся пунктом \"➖ Удалить канал\" в меню управления каналами", MAX_CHANNELS_PER_USER)
                ).await?;

                {
                    let mut pending_lock = app_state.pending_channels.lock().await;
                    pending_lock.remove(&user_id);
                }

                send_channels_menu(&bot, user_id, chat_id, &app_state).await?;
                return Ok(());
            }

            // Filter out private channels (without username) - only public channels allowed
            let (public_channels, private_channels): (Vec<_>, Vec<_>) = channels_to_add
                .iter()
                .partition(|ch| !ch.channel_username.trim().is_empty());

            let private_count = private_channels.len();

            let channels_to_save: Vec<_> = public_channels.iter().take(available_slots).collect();
            let skipped_count = public_channels.len() - channels_to_save.len();

            let mut saved_channels = Vec::new();
            let mut failed_channels = Vec::new();

            for channel in channels_to_save {
                match channel_management::add_channel(
                    db_pool.as_ref(),
                    user_id,
                    channel.channel_id,
                    &channel.channel_title,
                    &channel.channel_username,
                )
                .await
                {
                    Ok(_) => {
                        saved_channels.push((
                            channel.channel_title.clone(),
                            channel.channel_username.clone(),
                        ));
                    }
                    Err(e) => {
                        warn!("Failed to add channel {}: {}", channel.channel_id, e);
                        failed_channels.push((
                            channel.channel_title.clone(),
                            channel.channel_username.clone(),
                        ));
                    }
                }
            }

            {
                let mut pending_lock = app_state.pending_channels.lock().await;
                pending_lock.remove(&user_id);
            }

            let mut result_parts = Vec::new();

            if !saved_channels.is_empty() {
                let channels_list = saved_channels
                    .iter()
                    .map(|(title, username)| format!("  • \"{}\" (@{})", title, username))
                    .collect::<Vec<_>>()
                    .join("\n");
                result_parts.push(format!(
                    "✅ Успешно сохранены каналы ({}):\n\n{}",
                    saved_channels.len(),
                    channels_list
                ));
            }

            if private_count > 0 {
                result_parts.push(format!(
                    "⚠️ Приватных каналов пропущено: {}\n(Поддерживаются только публичные каналы с username)",
                    private_count
                ));
            }

            if skipped_count > 0 {
                result_parts.push(format!(
                    "⚠️ Достигнут лимит (макс. {}), не добавлено: {}",
                    MAX_CHANNELS_PER_USER, skipped_count
                ));
            }

            if !failed_channels.is_empty() {
                let failed_list = failed_channels
                    .iter()
                    .map(|(title, username)| format!("  • \"{}\" (@{})", title, username))
                    .collect::<Vec<_>>()
                    .join("\n");
                result_parts.push(format!(
                    "❌ Ошибки при сохранении ({}):\n{}",
                    failed_channels.len(),
                    failed_list
                ));
            }

            let result_msg = if result_parts.is_empty() {
                "❌ Похоже ты не предоставил ни одного валидного канала для сохранения".to_string()
            } else {
                result_parts.join("\n\n")
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
                    "« Выйти в Главное меню",
                    "back_to_main_menu",
                )],
            ]);

            let bot_system_message = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::SupportMessage,
            ))
            .await?;

            bot.send_message(chat_id, bot_system_message)
                .reply_markup(keyboard)
                .parse_mode(ParseMode::Html)
                .await?;
            Ok(())
        }
        "🎙 Сегодняшний подкаст" => {
            let bot_system_message = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::PleaseWaitForPublicPodcastSearch,
            ))
            .await?;
            bot.send_message(chat_id, bot_system_message).await?;

            tokio::time::sleep(Duration::from_secs(2)).await;

            send_daily_podcast(
                &bot,
                user_id,
                chat_id,
                username,
                app_state.clone(),
                Recipient::Public,
            )
            .await?;

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
            let bot_system_message = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::PleaseWaitForPersonalPodcastSearch,
            ))
            .await?;
            bot.send_message(chat_id, bot_system_message).await?;

            tokio::time::sleep(Duration::from_secs(2)).await;

            send_daily_podcast(
                &bot,
                user_id,
                chat_id,
                username,
                app_state.clone(),
                Recipient::Private(user_id),
            )
            .await?;

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

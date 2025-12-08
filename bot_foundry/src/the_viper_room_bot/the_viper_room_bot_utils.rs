use anyhow::anyhow;
use anyhow::Result;
use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Utc};
use core::local_db::the_viper_room::channel_management;
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::TheViperRoomBotMessages;
use core::models::tg_bot::the_viper_room_bot::the_viper_room_bot_user_state::TheViperRoomBotUserState;
use core::models::the_viper_room::db_models::Recipient;
use core::models::the_viper_room::the_viper_room_bot;
use core::models::the_viper_room::the_viper_room_bot::MainMenuMessageType;
use core::state::tg_bot::TheViperRoomBotState;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::{start_bots_chat_action, stop_bots_chat_action};
use core::utils::the_viper_room::news_block_creation::news_block_creation;
use core::utils::the_viper_room::news_block_creation_utils::{
    generate_waveform, save_daily_podcast,
};
use grammers_client::types::{attributes::Attribute, InputMessage};
use grammers_client::Client as g_Client;
use reqwest::multipart;
use std::fs::{read_dir, read_to_string, remove_file};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::{ChatId, Requester};
use teloxide::types::{ChatAction, MessageOrigin};
use teloxide::types::{ChatKind, PublicChatKind};
use teloxide::Bot;
use teloxide_core::payloads::SendMessageSetters;
use teloxide_core::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup, Message, ParseMode,
    UserId,
};
use tokio::sync::Mutex;
use tokio::time;
use tokio::time::Instant;
use tracing::log::info;
use tracing::{error, warn};

fn extract_channel_username_from_link(input: &str) -> Option<String> {
    // Try to strip common prefixes
    let after_tme = input
        .strip_prefix("https://t.me/")
        .or_else(|| input.strip_prefix("http://t.me/"))
        .or_else(|| input.strip_prefix("t.me/"))?;

    // Username is everything before first slash, space or special char
    let username = after_tme.split(&['/', ' ', '?', '#'][..]).next()?.trim();

    if username.is_empty() {
        None
    } else {
        Some(username.to_string())
    }
}

pub async fn generate_podcast(
    app_state: Arc<TheViperRoomBotState>,
    user_id: i64, // make u64
    recipient: Recipient,
) -> Result<PathBuf> {
    info!("Starting podcast generation for recipient: {:?}", recipient);

    let telegram_client = &app_state.telegram_agent.client;

    if !telegram_client.is_authorized().await? {
        return Err(anyhow::anyhow!("Telegram agent is not authorized!"));
    }

    let podcast_path = news_block_creation(
        &telegram_client,
        &user_id.to_string(),
        app_state.clone(),
        recipient,
        true,
        app_state.core.db_pool.as_ref().map(|v| v.as_ref()),
    )
    .await?;

    info!("Podcast generated successfully: {:?}", podcast_path);

    Ok(podcast_path)
}

pub async fn send_generated_podcast_via_telegram_agent(
    podcast_file_path: PathBuf,
    telegram_client: &g_Client,
    chat_username: &str,
) -> Result<()> {
    info!("Sending podcast via Telegram agent to @{}", chat_username);

    let uploaded_file = telegram_client.upload_file(&podcast_file_path).await?;

    let podcast_caption_file = podcast_file_path.with_extension("txt");

    let podcast_caption = read_to_string(&podcast_caption_file)
        .map_err(|e| anyhow::anyhow!("Failed to read podcast caption from file: {}", e))?;

    let waveform = generate_waveform(&podcast_file_path).await?;

    let input_message_default = InputMessage::default();

    let input_message = input_message_default
        .document(uploaded_file)
        .attribute(Attribute::Voice {
            duration: Duration::from_secs(0),
            waveform: Option::from(waveform),
        });

    let chat = telegram_client
        .resolve_username(chat_username)
        .await?
        .ok_or_else(|| anyhow!("Chat for broadcasting not found"))?;

    telegram_client.send_message(&chat, input_message).await?;
    telegram_client.send_message(&chat, podcast_caption).await?;

    info!("Podcast sent successfully to @{}", chat_username);

    if let Err(e) =
        save_daily_podcast(&podcast_file_path, &podcast_caption_file, Recipient::Public).await
    {
        error!("Failed to save daily public podcast: {}", e);
    }

    for file in [&podcast_file_path, &podcast_caption_file] {
        match remove_file(file) {
            Ok(_) => info!("File {} deleted after broadcast!", file.display()),
            Err(e) => info!("Could not delete {}: {}", file.display(), e),
        }
    }

    Ok(())
}

async fn send_voice_with_waveform(
    bot_token: &str,
    chat_id: ChatId,
    audio_path: &PathBuf,
    caption: &str,
    waveform: Vec<u8>,
) -> Result<()> {
    let api_url = format!("https://api.telegram.org/bot{}/sendVoice", bot_token);

    let file_bytes = tokio::fs::read(audio_path).await?;
    let file_name = audio_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("podcast.mp3");

    let form = multipart::Form::new()
        .text("chat_id", chat_id.to_string())
        .text("caption", caption.to_string())
        .text("waveform", serde_json::to_string(&waveform)?)
        .part(
            "voice",
            multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string())
                .mime_str("audio/mpeg")?,
        );

    let client = reqwest::Client::new();
    let response = client.post(&api_url).multipart(form).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!(
            "Failed to send voice with waveform: {}",
            error_text
        ));
    }

    Ok(())
}

pub async fn send_generated_podcast_via_bot(
    _bot: &Bot,
    chat_id: ChatId,
    recipient: Recipient,
    username: &str,
) -> Result<()> {
    let podcast_dir = match &recipient {
        Recipient::Public => "common_res/the_viper_room/daily_public_podcast".to_string(),
        Recipient::Private(user_id) => {
            format!(
                "common_res/the_viper_room/{}/daily_private_podcast",
                user_id
            )
        }
    };

    info!("Looking for podcast in: {}", podcast_dir);

    let mut podcast_file: Option<PathBuf> = None;
    let mut caption_file: Option<PathBuf> = None;

    if let Ok(entries) = read_dir(&podcast_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    match extension.to_str() {
                        Some("mp3") => podcast_file = Some(path),
                        Some("txt") => caption_file = Some(path),
                        _ => {}
                    }
                }
            }
        }
    }

    let podcast_path = match podcast_file {
        Some(path) => path,
        None => {
            return Err(anyhow::anyhow!("Podcast file not found in {}", podcast_dir));
        }
    };

    info!("Found podcast: {:?}", podcast_path);

    let caption = if let Some(caption_path) = caption_file {
        info!("Found caption: {:?}", caption_path);
        read_to_string(&caption_path).unwrap_or_else(|e| {
            error!("Failed to read caption file: {}", e);
            match recipient {
                Recipient::Public => "Сегодняшний подкаст".to_string(),
                Recipient::Private(_) => "Твой персональный подкаст".to_string(),
            }
        })
    } else {
        match recipient {
            Recipient::Public => "Сегодняшний подкаст".to_string(),
            Recipient::Private(_) => "Твой персональный подкаст".to_string(),
        }
    };

    info!("Sending podcast via bot to user {} [{}]", username, chat_id);

    let waveform = generate_waveform(&podcast_path).await?;

    let bot_token = std::env::var("TELOXIDE_TOKEN_THE_VIPER_ROOM_BOT")?;

    send_voice_with_waveform(&bot_token, chat_id, &podcast_path, &caption, waveform).await?;

    // bot.send_voice(chat_id, InputFile::file(&podcast_path))
    //     .caption(&caption)
    //     .await?;

    info!(
        "Podcast sent successfully to user {} [{}]",
        username, chat_id
    );

    Ok(())
}

pub async fn schedule_podcast(
    _bot: Bot,
    _user_id: ChatId,
    app_state: Arc<TheViperRoomBotState>,
    app_tg_account_id: i64,
) -> Result<()> {
    info!("Starting podcast scheduling task by /schedule cmd...");
    {
        let mut is_running = app_state.podcast_manager.state.is_running.lock().await;
        if *is_running {
            return Err(anyhow::anyhow!(
                "Podcast generation task is already running"
            ));
        }
        *is_running = true;
    }

    let chat_username = "the_viper_room";

    let offset = FixedOffset::east_opt(3 * 3600).unwrap();
    let now: DateTime<FixedOffset> = Utc::now().with_timezone(&offset);
    let podcast_time = offset
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 9, 00, 00)
        .unwrap();

    let duration_until_podcast_time = if now > podcast_time {
        podcast_time + chrono::Duration::days(1) - now
    } else {
        podcast_time - now
    };

    let start_at =
        Instant::now() + Duration::from_secs(duration_until_podcast_time.num_seconds() as u64);
    let mut interval = time::interval_at(start_at, Duration::from_secs(24 * 60 * 60));

    info!("Current time (UTC+3): {}", now);
    info!("Scheduled podcast time (UTC+3): {}", podcast_time);

    let hours = duration_until_podcast_time.num_hours();
    let minutes = duration_until_podcast_time.num_minutes() % 60;
    let seconds = duration_until_podcast_time.num_seconds() % 60;

    let mut stop_rx = app_state.podcast_manager.stop_rx.clone();

    tokio::spawn({
        let app_state = app_state.clone();

        async move {
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let podcast_path = match generate_podcast(
                            app_state.clone(),
                            app_tg_account_id,
                            Recipient::Public,
                        ).await {
                            Ok(path) => path,
                            Err(e) => {
                                error!("Error in podcast generation: {:?}", e);
                                continue;
                            }
                        };

                        if let Err(e) = send_generated_podcast_via_telegram_agent(
                            podcast_path,
                            &app_state.telegram_agent.client,
                            chat_username,
                        ).await {
                            error!("Error sending podcast: {:?}", e);
                        }
                    }
                    Ok(_) = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            let mut is_running = app_state.podcast_manager.state.is_running.lock().await;
                            *is_running = false;
                            info!("Stopping daily podcast generation task");
                            break;
                        }
                    }
                }
            }
        }
    });

    info!("Daily podcast generation task successfully scheduled! Next run in {} hours, {} minutes, {} seconds",
       hours, minutes, seconds);

    Ok(())
}

pub async fn stop_daily_podcasts(app_state: Arc<TheViperRoomBotState>) -> Result<()> {
    info!("Stopping podcast scheduling task by /stop cmd...");
    let is_running = {
        let running = app_state.podcast_manager.state.is_running.lock().await;
        *running
    };

    if !is_running {
        return Err(anyhow::anyhow!(
            "No podcast generation task is currently running"
        ));
    }

    app_state.podcast_manager.state.stop_sender.send(true)?;
    info!("Stop signal sent to daily podcast generation task");
    Ok(())
}

pub async fn send_main_menu(
    bot: &Bot,
    user_id: UserId,
    chat_id: ChatId,
    app_state: &Arc<TheViperRoomBotState>,
    message_type: MainMenuMessageType,
) -> Result<()> {
    {
        let mut states_lock = app_state.user_states.lock().await;
        states_lock.insert(user_id.0, TheViperRoomBotUserState::Idle);
    }

    let main_menu_text = match message_type {
        MainMenuMessageType::Full => {
            get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::MainMenu,
            ))
            .await?
        }
        MainMenuMessageType::Minimal => {
            get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::MainMenuMinimal,
            ))
            .await?
        }
    };

    let keyboard = KeyboardMarkup::new(vec![
        vec![
            KeyboardButton::new("🎧 Персональный подкаст"),
            KeyboardButton::new("🎙 Сегодняшний подкаст"),
        ],
        vec![
            KeyboardButton::new("❓ Задать вопрос"),
            KeyboardButton::new("⚙️ Настройки"),
        ],
    ])
    .resize_keyboard()
    .one_time_keyboard();

    bot.send_message(chat_id, main_menu_text)
        .reply_markup(keyboard)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

pub async fn send_settings_menu(
    bot: &Bot,
    user_id: UserId,
    chat_id: ChatId,
    app_state: &Arc<TheViperRoomBotState>,
) -> Result<()> {
    {
        let mut states_lock = app_state.user_states.lock().await;
        states_lock.insert(user_id.0, TheViperRoomBotUserState::InSettingsMenu);
    }

    let settings_text = "⚙️ Настройки:";

    let inline_keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "📋 Мои каналы",
            "settings_my_channels",
        )],
        vec![InlineKeyboardButton::callback(
            "⏰ Время подкаста",
            "settings_podcast_time",
        )],
        vec![InlineKeyboardButton::callback(
            "« Выйти в Главное меню",
            "back_to_main_menu",
        )],
    ]);

    bot.send_message(chat_id, settings_text)
        .reply_markup(inline_keyboard)
        .await?;

    Ok(())
}

pub async fn send_channels_menu(
    bot: &Bot,
    user_id: UserId,
    chat_id: ChatId,
    app_state: &Arc<TheViperRoomBotState>,
) -> Result<()> {
    {
        let mut states_lock = app_state.user_states.lock().await;
        states_lock.insert(user_id.0, TheViperRoomBotUserState::ChannelsMenuView);
    }

    let channels_text = "📋 Управление каналами:";

    let inline_keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "👁 Показать мои каналы",
            "channels_show_list",
        )],
        vec![
            InlineKeyboardButton::callback("➖ Удалить канал", "channels_delete"),
            InlineKeyboardButton::callback("➕ Добавить канал", "channels_add"),
        ],
        vec![InlineKeyboardButton::callback(
            "« Назад в Настройки",
            "back_to_settings",
        )],
        vec![InlineKeyboardButton::callback(
            "« Выйти в главное меню",
            "back_to_main_menu",
        )],
    ]);

    bot.send_message(chat_id, channels_text)
        .reply_markup(inline_keyboard)
        .await?;

    Ok(())
}

pub async fn show_user_channels(
    bot: &Bot,
    user_id: UserId,
    chat_id: ChatId,
    app_state: &Arc<TheViperRoomBotState>,
) -> Result<()> {
    let db_pool = match &app_state.core.db_pool {
        Some(pool) => pool,
        None => {
            bot.send_message(chat_id, "Ошибка: база данных недоступна")
                .await?;
            return Ok(());
        }
    };

    let user_id_i64 = user_id.0 as i64;
    let channels = channel_management::get_user_channels(db_pool.as_ref(), user_id_i64).await?;

    let message = if channels.is_empty() {
        "📋 Твой список каналов пуст\n\nСначала добавь каналы для персонального подкаста"
    } else {
        let channels_list = channels
            .iter()
            .enumerate()
            .map(|(i, ch)| {
                format!(
                    "{}. {} @{} (ID: {})",
                    i + 1,
                    ch.channel_title,
                    ch.channel_username,
                    ch.channel_id
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        &format!("📋 Твои каналы:\n\n{}", channels_list)
    };

    let inline_keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "« Назад к меню управления каналами",
        "back_to_channels_menu",
    )]]);

    bot.send_message(chat_id, message)
        .reply_markup(inline_keyboard)
        .await?;

    Ok(())
}

pub async fn send_add_channel_prompt(
    bot: &Bot,
    user_id: UserId,
    chat_id: ChatId,
    app_state: &Arc<TheViperRoomBotState>,
) -> Result<()> {
    {
        let mut states_lock = app_state.user_states.lock().await;
        states_lock.insert(user_id.0, TheViperRoomBotUserState::ChannelsAdding);
    }

    {
        let mut pending_lock = app_state.pending_channels.lock().await;
        pending_lock.insert(user_id.0, Vec::new());
    }

    let instruction_text = get_message(AppsSystemMessages::TheViperRoomBot(
        TheViperRoomBotMessages::ChannelAddingInstruction,
    ))
    .await?;

    let keyboard = KeyboardMarkup::new(vec![
        vec![KeyboardButton::new("💾 Сохранить")],
        vec![KeyboardButton::new("🏠 Главное меню")],
    ])
    .resize_keyboard()
    .one_time_keyboard();

    bot.send_message(chat_id, instruction_text)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

pub async fn send_delete_channel_prompt(
    bot: &Bot,
    user_id: UserId,
    chat_id: ChatId,
    app_state: &Arc<TheViperRoomBotState>,
) -> Result<()> {
    {
        let mut states_lock = app_state.user_states.lock().await;
        states_lock.insert(user_id.0, TheViperRoomBotUserState::ChannelsDeleting);
    }

    let instruction_text = "📋 Удаление канала\n\nОтправь ID канала, который хочешь удалить.\n\nID можно найти в списке твоих каналов (пункт \"👁 Показать мои каналы\")";

    let keyboard = KeyboardMarkup::new(vec![
        vec![KeyboardButton::new("🗑 Удалить все каналы")],
        vec![KeyboardButton::new("🏠 Главное меню")],
    ])
    .resize_keyboard()
    .one_time_keyboard();

    bot.send_message(chat_id, instruction_text)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

pub enum ChannelInput {
    Forwarded(i64, String, String), // channel_id, title, username (NOT NULL)
    Usernames(Vec<String>, Vec<String>),
}

pub fn parse_channel_input(msg: &Message) -> Result<ChannelInput> {
    if let Some(origin) = msg.forward_origin() {
        match origin {
            MessageOrigin::Channel { chat, .. } => match &chat.kind {
                ChatKind::Public(public_chat) => match &public_chat.kind {
                    PublicChatKind::Channel(channel_info) => {
                        let channel_id = the_viper_room_bot::normalize_channel_id(chat.id.0);
                        let channel_title = chat.title().unwrap_or("Без названия").to_string();

                        let channel_username = match channel_info.username.clone() {
                            Some(username) => username,
                            None => {
                                warn!(
                                    "Public channel '{}' (ID: {}) has no username - unexpected API behavior",
                                    channel_title, channel_id
                                );
                                return Err(anyhow!(
                                    "❌ Неожиданная ошибка Telegram API: публичный канал не имеет username.\n\
                                    Попробуй добавить канал по @username или ссылке https://t.me/..."
                                ));
                            }
                        };

                        info!(
                            "Forwarded from channel: ID={}, title='{}', username=@{}",
                            channel_id, channel_title, channel_username
                        );

                        return Ok(ChannelInput::Forwarded(
                            channel_id,
                            channel_title,
                            channel_username,
                        ));
                    }
                    _ => {
                        return Err(anyhow!(
                            "Источник не является каналом. Перешли сообщение именно из канала."
                        ));
                    }
                },
                _ => {
                    return Err(anyhow!("Источник не является публичным каналом."));
                }
            },
            MessageOrigin::Chat { .. } => {
                return Err(anyhow!(
                    "Сообщение переслано из чата. Пожалуйста, перешли пост непосредственно из канала."
                ));
            }
            _ => {
                return Err(anyhow!(
                    "Сообщение переслано от пользователя, а не из канала."
                ));
            }
        }
    }

    if msg.forward_date().is_some() {
        return Err(anyhow!(
            "Не удалось определить источник пересланного сообщения. Попробуй добавить канал по username."
        ));
    }

    if let Some(text) = msg.text() {
        let text = text.trim();
        if text.is_empty() {
            return Err(anyhow!("Я получил пустое сообщение. Отправь username канала (@channelname) или ссылку (https://t.me/channelname)"));
        }

        let parts: Vec<&str> = text
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut valid_usernames: Vec<String> = Vec::new();
        let mut invalid_inputs: Vec<String> = Vec::new();

        for part in parts {
            // Split each part by whitespace to handle cases like "https://t.me/username srgs"
            let tokens: Vec<&str> = part.split_whitespace().collect();

            for token in tokens {
                // Try to extract from t.me link first
                if let Some(username) = extract_channel_username_from_link(token) {
                    valid_usernames.push(username);
                } else if token.starts_with('@') {
                    // Username with @ prefix
                    valid_usernames.push(token[1..].to_string());
                } else {
                    // Invalid input (no @ prefix, not a link)
                    invalid_inputs.push(token.to_string());
                }
            }
        }

        if valid_usernames.is_empty() {
            return Err(anyhow!(
                "Не найдено ни одного валидного username.\n\nUsername канала должен начинаться с '@' или быть ссылкой https://t.me/channelname\n\nПримеры:\n• @channelname\n• https://t.me/channelname\n• Несколько: @channel1, https://t.me/channel2"
            ));
        }

        return Ok(ChannelInput::Usernames(valid_usernames, invalid_inputs));
    }

    Err(anyhow!(
        "Неподдерживаемый тип сообщения. Отправь username канала (@channelname), ссылку (https://t.me/channelname) или перешли пост из канала."
    ))
}

pub async fn send_daily_podcast(
    bot: &Bot,
    user_id: UserId,
    chat_id: ChatId,
    username: String,
    app_state: Arc<TheViperRoomBotState>,
    recipient: Recipient,
) -> Result<()> {
    let podcast_dir = match &recipient {
        Recipient::Public => "common_res/the_viper_room/daily_public_podcast".to_string(),
        Recipient::Private(uid) => {
            format!("common_res/the_viper_room/{}/daily_private_podcast", uid)
        }
    };

    info!("Looking for daily podcast in: {}", podcast_dir);

    let podcast_exists = if let Ok(entries) = read_dir(&podcast_dir) {
        entries
            .flatten()
            .any(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("mp3"))
    } else {
        false
    };

    if !podcast_exists {
        match &recipient {
            Recipient::Public => {
                let no_podcast_msg =
                    "К сожалению, сегодняшний подкаст ещё не готов. Попробуй зайти позже!";
                bot.send_message(chat_id, no_podcast_msg).await?;
                return Ok(());
            }
            Recipient::Private(uid) => {
                if let Some(db_pool) = app_state.core.db_pool.as_ref() {
                    let user_channels =
                        channel_management::get_user_channels(db_pool, *uid).await?;

                    if user_channels.len() < 5 {
                        let msg = format!(
                            "📋 У тебя всего {} каналов для персонального подкаста.\n\n\
                            Для генерации подкаста нужно минимум 5 каналов, чтобы было из чего делать контент.\n\n\
                            Добавь больше каналов через меню \"⚙️ Настройки\" → \"📝 Управление каналами\"",
                            user_channels.len()
                        );
                        bot.send_message(chat_id, msg).await?;

                        send_main_menu(
                            bot,
                            user_id,
                            chat_id,
                            &app_state,
                            MainMenuMessageType::Minimal,
                        )
                        .await?;

                        return Ok(());
                    }
                }

                let bot_system_message = get_message(AppsSystemMessages::TheViperRoomBot(
                    TheViperRoomBotMessages::PleaseWaitForPersonalPodcastRecord,
                ))
                .await?;
                bot.send_message(chat_id, bot_system_message).await?;

                let action_flag = Arc::new(Mutex::new(true));
                start_bots_chat_action(
                    bot.clone(),
                    chat_id,
                    ChatAction::RecordVoice,
                    Arc::clone(&action_flag),
                )
                .await;

                let generated_podcast =
                    generate_podcast(app_state.clone(), user_id.0 as i64, recipient.clone())
                        .await?;

                if let Err(e) = save_daily_podcast(
                    &generated_podcast,
                    &generated_podcast.with_extension("txt"),
                    recipient.clone(),
                )
                .await
                {
                    error!("Failed to save daily private podcast: {}", e);
                }

                for file in [&generated_podcast, &generated_podcast.with_extension("txt")] {
                    if file.exists() {
                        match remove_file(file) {
                            Ok(_) => {
                                info!("Temporary file {} deleted after saving!", file.display())
                            }
                            Err(e) => info!("Could not delete temporary {}: {}", file.display(), e),
                        }
                    }
                }

                stop_bots_chat_action(action_flag).await;
            }
        }
    } else {
        let message = match &recipient {
            Recipient::Public => {
                get_message(AppsSystemMessages::TheViperRoomBot(
                    TheViperRoomBotMessages::GrabAFreshPublicPodcast,
                ))
                .await?
            }
            Recipient::Private(_) => {
                get_message(AppsSystemMessages::TheViperRoomBot(
                    TheViperRoomBotMessages::GrabAFreshPersonalPodcast,
                ))
                .await?
            }
        };
        bot.send_message(chat_id, message).await?;
    }

    send_generated_podcast_via_bot(bot, chat_id, recipient, &username).await?;

    info!(
        "Daily podcast sent successfully for user: {} [{}]",
        username, user_id
    );

    Ok(())
}

async fn cleanup_daily_podcasts() -> Result<()> {
    info!("Starting daily podcasts cleanup...");

    let public_podcast_dir = "common_res/the_viper_room/daily_public_podcast";
    if let Ok(entries) = read_dir(public_podcast_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                match remove_file(&path) {
                    Ok(_) => info!("Removed public podcast file: {:?}", path),
                    Err(e) => error!("Failed to remove public podcast file {:?}: {}", path, e),
                }
            }
        }
    }

    let base_dir = "common_res/the_viper_room";
    if let Ok(entries) = read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                    if folder_name.parse::<i64>().is_ok() {
                        // Это папка пользователя, очищаем daily_private_podcast
                        let private_podcast_dir = path.join("daily_private_podcast");
                        if private_podcast_dir.exists() {
                            if let Ok(podcast_entries) = read_dir(&private_podcast_dir) {
                                for podcast_entry in podcast_entries.flatten() {
                                    let podcast_path = podcast_entry.path();
                                    if podcast_path.is_file() {
                                        match remove_file(&podcast_path) {
                                            Ok(_) => info!(
                                                "Removed private podcast file for user {}: {:?}",
                                                folder_name, podcast_path
                                            ),
                                            Err(e) => error!(
                                                "Failed to remove private podcast file {:?}: {}",
                                                podcast_path, e
                                            ),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    info!("Daily podcasts cleanup completed!");
    Ok(())
}

pub async fn schedule_daily_cleanup() -> Result<()> {
    info!("Starting daily cleanup scheduler...");

    let offset = FixedOffset::east_opt(3 * 3600).unwrap();
    let now: DateTime<FixedOffset> = Utc::now().with_timezone(&offset);
    let midnight = offset
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .unwrap();

    let duration_until_midnight = if now > midnight {
        midnight + chrono::Duration::days(1) - now
    } else {
        midnight - now
    };

    let start_at =
        Instant::now() + Duration::from_secs(duration_until_midnight.num_seconds() as u64);
    let mut interval = time::interval_at(start_at, Duration::from_secs(24 * 60 * 60));

    let hours = duration_until_midnight.num_hours();
    let minutes = duration_until_midnight.num_minutes() % 60;
    let seconds = duration_until_midnight.num_seconds() % 60;

    info!(
        "Daily cleanup scheduled! Next run in {} hours, {} minutes, {} seconds",
        hours, minutes, seconds
    );

    tokio::spawn(async move {
        loop {
            interval.tick().await;
            if let Err(e) = cleanup_daily_podcasts().await {
                error!("Error during daily cleanup: {:?}", e);
            }
        }
    });

    Ok(())
}

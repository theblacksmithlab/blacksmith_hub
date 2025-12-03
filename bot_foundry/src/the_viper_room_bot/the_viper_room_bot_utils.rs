use anyhow::anyhow;
use anyhow::Result;
use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Utc};
use core::local_db::the_viper_room::channel_management;
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::TheViperRoomBotMessages;
use core::models::tg_bot::the_viper_room_bot::the_viper_room_bot_user_state::TheViperRoomBotUserState;
use core::state::tg_bot::app_state::BotAppState;
use core::telegram_client::grammers_functionality::initialize_grammers_client;
use core::utils::common::get_message;
use core::utils::the_viper_room::news_block_creation::news_block_creation;
use core::utils::the_viper_room::news_block_creation_utils::{
    generate_waveform, save_daily_public_podcast,
};
use grammers_client::types::{attributes::Attribute, InputMessage};
use grammers_client::Client as g_Client;
use std::fs::{read_dir, read_to_string, remove_file};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::{ChatId, Requester};
use teloxide::types::{ChatKind, PublicChatKind};
use teloxide::Bot;
use teloxide_core::payloads::{SendAudioSetters, SendMessageSetters};
use teloxide_core::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, InputFile, KeyboardButton, KeyboardMarkup,
    ParseMode, UserId,
};
use tokio::time;
use tokio::time::Instant;
use tracing::error;
use tracing::log::info;
use teloxide::types::MessageOrigin;

/// Normalizes Telegram channel ID to raw positive format.
///
/// Telegram uses two formats for channel IDs:
/// - Teloxide/Bot API: `-100XXXXXXXXX` (full format with -100 prefix)
/// - Grammers/MTProto: `XXXXXXXXX` (raw format, positive)
///
/// This function converts both formats to the canonical raw format.
///
/// # Examples
/// - `-1001403170292` → `1403170292`
/// - `1403170292` → `1403170292`
pub fn normalize_channel_id(id: i64) -> i64 {
    // Check if ID is in full format (negative with -100 prefix)
    // The -100 prefix means the number is less than -10^12
    if id < -1_000_000_000_000 {
        // Strip the -100 prefix: abs(id) - 10^12
        id.abs() - 1_000_000_000_000
    } else {
        // Already in raw format or positive - just ensure it's positive
        id.abs()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MainMenuMessageType {
    Full,
    Minimal,
}

pub async fn generate_podcast(
    g_client: g_Client,
    bot: Bot,
    chat_id: ChatId,
    app_state: Arc<BotAppState>,
    tg_agent_id: &str,
    nickname: String,
    chat_username: &str,
) -> Result<()> {
    info!("Starting podcast generation by /podcast cmd...");

    if !g_client.is_authorized().await? {
        bot.send_message(chat_id, "System g_Client is NOT okay!")
            .await?;

        return Ok(());
    } else {
        bot.send_message(chat_id, "System g_Client is okay!")
            .await?;
    }

    let podcast = news_block_creation(&g_client, tg_agent_id, app_state, nickname, true).await?;

    let uploaded_file = g_client.upload_file(&podcast).await?;

    let podcast_caption_file = podcast.with_extension("txt");

    let podcast_caption = read_to_string(&podcast_caption_file)
        .map_err(|e| anyhow::anyhow!("Failed to read podcast caption from file: {}", e))?;

    let waveform = generate_waveform(&podcast).await?;

    let input_message_default = InputMessage::default();

    let input_message = input_message_default
        .document(uploaded_file)
        .attribute(Attribute::Voice {
            duration: Duration::from_secs(0),
            waveform: Option::from(waveform),
        });

    let chat = g_client
        .resolve_username(chat_username)
        .await?
        .ok_or_else(|| anyhow!("Channel for broadcasting not found"))?;

    g_client.send_message(&chat, input_message).await?;
    g_client.send_message(&chat, podcast_caption).await?;

    if let Err(e) = save_daily_public_podcast(&podcast, &podcast_caption_file).await {
        error!("Failed to save daily public podcast: {}", e);
    }

    for file in [&podcast, &podcast_caption_file] {
        match remove_file(file) {
            Ok(_) => info!("File {} deleted after broadcast!", file.display()),
            Err(e) => info!("Could not delete {}: {}", file.display(), e),
        }
    }

    Ok(())
}

pub async fn schedule_podcast(
    bot: Bot,
    user_id: ChatId,
    app_state: Arc<BotAppState>,
    app_tg_account_id: Arc<String>,
    nickname: String,
    session_data: Vec<u8>,
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

    let chat_username = "the_viper_room".to_string();

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
        let app_tg_account_id = Arc::clone(&app_tg_account_id);

        async move {
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let g_client = match initialize_grammers_client(session_data.clone()).await {
                            Ok(g_client) => {
                                if let Err(e) = g_client.is_authorized().await {
                                    error!("Client authorization failed: {:?}", e);
                                    continue;
                                }
                                g_client
                            },
                            Err(e) => {
                                error!("Failed to initialize grammers client: {:?}", e);
                                continue;
                            }
                        };

                        if let Err(e) = generate_podcast(
                            g_client,
                            bot.clone(),
                            user_id,
                            app_state.clone(),
                            &app_tg_account_id,
                            nickname.clone(),
                            &chat_username
                         ).await {
                            error!("Error in podcast generation: {:?}", e);
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

pub async fn stop_daily_podcasts(app_state: Arc<BotAppState>) -> Result<()> {
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
    app_state: &Arc<BotAppState>,
    message_type: MainMenuMessageType,
) -> Result<()> {
    if let Some(states) = &app_state.the_viper_room_bot_user_states {
        let mut states_lock = states.lock().await;
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
    app_state: &Arc<BotAppState>,
) -> Result<()> {
    if let Some(states) = &app_state.the_viper_room_bot_user_states {
        let mut states_lock = states.lock().await;
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
    app_state: &Arc<BotAppState>,
) -> Result<()> {
    if let Some(states) = &app_state.the_viper_room_bot_user_states {
        let mut states_lock = states.lock().await;
        states_lock.insert(user_id.0, TheViperRoomBotUserState::ChannelsMenuView);
    }

    let channels_text = "📋 Управление каналами:";

    let inline_keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "👁 Показать мои каналы",
            "channels_show_list",
        )],
        vec![InlineKeyboardButton::callback(
            "➖ Удалить канал",
            "channels_delete"),
             InlineKeyboardButton::callback(
            "➕ Добавить канал",
            "channels_add",
        )],
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
    app_state: &Arc<BotAppState>,
) -> Result<()> {
    let db_pool = match &app_state.db_pool {
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
                let username_info = ch.channel_username
                    .as_ref()
                    .map(|u| format!(" @{}", u))
                    .unwrap_or_default();
                format!("{}. {}{} (ID: {})", i + 1, ch.channel_title, username_info, ch.channel_id)
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
    app_state: &Arc<BotAppState>,
) -> Result<()> {
    if let Some(states) = &app_state.the_viper_room_bot_user_states {
        let mut states_lock = states.lock().await;
        states_lock.insert(user_id.0, TheViperRoomBotUserState::ChannelsAdding);
    }

    if let Some(pending) = &app_state.the_viper_room_bot_pending_channels {
        let mut pending_lock = pending.lock().await;
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

pub enum ChannelInput {
    /// Channel forwarded from a post - contains (channel_id, channel_title, channel_username)
    Forwarded(i64, String, Option<String>),
    /// Text input with channel usernames (without @) and list of ignored invalid inputs
    Usernames(Vec<String>, Vec<String>),
}

/// Parses user input to extract channel information
/// Supports: forwarded posts from channels, @username (multiple comma-separated)
pub fn parse_channel_input(msg: &teloxide::types::Message) -> Result<ChannelInput> {
    if let Some(origin) = msg.forward_origin() {
        match origin {
            MessageOrigin::Channel { chat, .. } => {
                match &chat.kind {
                    ChatKind::Public(public_chat) => {
                        match &public_chat.kind {
                            PublicChatKind::Channel(channel_info) => {
                                let channel_id = normalize_channel_id(chat.id.0);
                                let channel_title = chat.title().unwrap_or("Без названия").to_string();
                                let channel_username = channel_info.username.clone();

                                info!(
                                    "Forwarded from channel: ID={}, title='{}', username={:?}",
                                    channel_id, channel_title, channel_username
                                );

                                return Ok(ChannelInput::Forwarded(channel_id, channel_title, channel_username));
                            }
                            _ => {
                                return Err(anyhow!(
                                    "Источник не является каналом. Перешли сообщение именно из канала."
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(anyhow!(
                            "Источник не является публичным каналом."
                        ));
                    }
                }
            }
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
            return Err(anyhow!("Я получил пустое сообщение. Отправь username канала начиная с @"));
        }

        let parts: Vec<&str> = text
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut valid_usernames: Vec<String> = Vec::new();
        let mut invalid_inputs: Vec<String> = Vec::new();

        for part in parts {
            if part.starts_with('@') {
                valid_usernames.push(part[1..].to_string());
            } else {
                invalid_inputs.push(part.to_string());
            }
        }

        if valid_usernames.is_empty() {
            return Err(anyhow!(
                "Не найдено ни одного валидного username.\n\nUsername канала должен начинаться с '@'\n\nПример: @channelname\nИли несколько: @channel1, @channel2"
            ));
        }

        return Ok(ChannelInput::Usernames(valid_usernames, invalid_inputs));
    }

    Err(anyhow!(
        "Неподдерживаемый тип сообщения. Отправь username канала (@channelname) или перешли пост из канала."
    ))
}

pub(crate) async fn send_actual_daily_public_podcast(bot: Bot, chat_id: ChatId) -> Result<()> {
    let daily_podcast_dir = "common_res/the_viper_room/daily_public_podcast";

    info!("Looking for daily public podcast in: {}", daily_podcast_dir);

    let mut podcast_file: Option<PathBuf> = None;
    let mut caption_file: Option<PathBuf> = None;

    if let Ok(entries) = read_dir(daily_podcast_dir) {
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
            let no_podcast_msg = "К сожалению, сегодняшний подкаст ещё не готов. Попробуйте позже!";
            bot.send_message(chat_id, no_podcast_msg).await?;
            return Ok(());
        }
    };

    info!("Found daily podcast: {:?}", podcast_path);

    let title = extract_podcast_title(&podcast_path);

    let caption = if let Some(caption_path) = caption_file {
        info!("Found caption: {:?}", caption_path);
        read_to_string(&caption_path).unwrap_or_else(|e| {
            error!("Failed to read caption file: {}", e);
            "Сегодняшний подкаст".to_string()
        })
    } else {
        "Сегодняшний подкаст".to_string()
    };

    info!("Sending daily podcast to user...");

    let thumbnail_path = "common_res/the_viper_room/podcast_cover.jpg";

    bot.send_audio(chat_id, InputFile::file(&podcast_path))
        .title(title)
        .performer("The Viper Room")
        .thumbnail(InputFile::file(thumbnail_path))
        .caption(&caption)
        .await?;

    info!("Daily podcast sent successfully!");

    Ok(())
}

fn extract_podcast_title(path: &PathBuf) -> String {
    if let Some(file_name) = path.file_stem() {
        if let Some(name_str) = file_name.to_str() {
            if let Some(date_start) = name_str.rfind('(') {
                if let Some(date_end) = name_str.rfind(')') {
                    let date = &name_str[date_start + 1..date_end];
                    return format!("Daily Podcast [{}]", date);
                }
            }
        }
    }

    "Daily Podcast".to_string()
}

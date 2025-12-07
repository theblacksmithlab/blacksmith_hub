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
use core::utils::the_viper_room::news_block_creation::news_block_creation;
use core::utils::the_viper_room::news_block_creation_utils::{
    generate_waveform, save_daily_podcast,
};
use grammers_client::types::{attributes::Attribute, InputMessage};
use grammers_client::Client as g_Client;
use std::fs::{read_dir, read_to_string, remove_file};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::{ChatId, Requester};
use teloxide::types::MessageOrigin;
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

pub async fn send_generated_podcast_via_bot(
    bot: &Bot,
    chat_id: ChatId,
    podcast_file_path: PathBuf,
    username: &str,
    user_id: i64,
) -> Result<()> {
    info!("Sending podcast via bot to user {} [{}]", username, chat_id);

    let podcast_caption_file = podcast_file_path.with_extension("txt");

    let caption = if podcast_caption_file.exists() {
        read_to_string(&podcast_caption_file).unwrap_or_else(|e| {
            error!("Failed to read caption file: {}", e);
            "Твой персональный подкаст".to_string()
        })
    } else {
        "Твой персональный подкаст".to_string()
    };

    let title = extract_podcast_title(&podcast_file_path);
    let thumbnail_path = "common_res/the_viper_room/podcast_cover.jpg";

    bot.send_audio(chat_id, InputFile::file(&podcast_file_path))
        .title(title)
        .performer("The Viper Room")
        .thumbnail(InputFile::file(thumbnail_path))
        .caption(&caption)
        .await?;

    info!(
        "Podcast sent successfully to user {} [{}]",
        username, chat_id
    );

    if let Err(e) = save_daily_podcast(
        &podcast_file_path,
        &podcast_caption_file,
        Recipient::Private(user_id),
    )
    .await
    {
        error!(
            "Failed to save daily private podcast for user {}: {}",
            user_id, e
        );
    }

    for file in [&podcast_file_path, &podcast_caption_file] {
        if file.exists() {
            match remove_file(file) {
                Ok(_) => info!("File {} deleted after sending!", file.display()),
                Err(e) => info!("Could not delete {}: {}", file.display(), e),
            }
        }
    }

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
                let username_info = ch
                    .channel_username
                    .as_ref()
                    .map(|u| format!(" @{}", u))
                    .unwrap_or_default();
                format!(
                    "{}. {}{} (ID: {})",
                    i + 1,
                    ch.channel_title,
                    username_info,
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
    Forwarded(i64, String, Option<String>),
    Usernames(Vec<String>, Vec<String>),
}

pub fn parse_channel_input(msg: &teloxide::types::Message) -> Result<ChannelInput> {
    if let Some(origin) = msg.forward_origin() {
        match origin {
            MessageOrigin::Channel { chat, .. } => match &chat.kind {
                ChatKind::Public(public_chat) => match &public_chat.kind {
                    PublicChatKind::Channel(channel_info) => {
                        let channel_id = the_viper_room_bot::normalize_channel_id(chat.id.0);
                        let channel_title = chat.title().unwrap_or("Без названия").to_string();
                        let channel_username = channel_info.username.clone();

                        info!(
                            "Forwarded from channel: ID={}, title='{}', username={:?}",
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

pub async fn send_private_daily_podcast(
    bot: &Bot,
    user_id: UserId,
    chat_id: ChatId,
    username: String,
    app_state: Arc<TheViperRoomBotState>,
    recipient: Recipient,
) -> Result<()> {
    let personal_daily_podcast_dir = format!(
        "common_res/the_viper_room/{}/daily_personal_podcast",
        user_id.0
    );

    info!(
        "Looking for personal daily podcast in: {}",
        personal_daily_podcast_dir
    );

    let mut podcast_file: Option<PathBuf> = None;
    let mut caption_file: Option<PathBuf> = None;

    if let Ok(entries) = read_dir(personal_daily_podcast_dir) {
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
        Some(path) => {
            let bot_system_message = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::GrabAFreshPersonalPodcast,
            ))
                .await?;
            bot.send_message(chat_id, bot_system_message).await?;
            
            path
        },
        None => {
            let bot_system_message = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::PleaseWaitForPersonalPodcastSearch,
            ))
            .await?;
            bot.send_message(chat_id, bot_system_message).await?;

            let generated_personal_podcast = generate_podcast(
                app_state.clone(),
                user_id.0 as i64,
                recipient
            )
                .await?;

            if let Err(e) =
                save_daily_podcast(
                    &generated_personal_podcast,
                    &generated_personal_podcast
                        .with_extension("txt"),
                    Recipient::Public
                )
                    .await
            {
                error!("Failed to save daily public podcast: {}", e);
            }

            generated_personal_podcast
        }
    };

    info!("Found daily personal podcast: {:?}", podcast_path);

    let title = extract_podcast_title(&podcast_path);

    let caption = if let Some(caption_path) = caption_file {
        info!("Found caption: {:?}", caption_path);
        read_to_string(&caption_path).unwrap_or_else(|e| {
            error!("Failed to read caption file: {}", e);
            "Твой сегодняшний подкаст".to_string()
        })
    } else {
        "Твой сегодняшний подкаст".to_string()
    };

    info!("Sending personal daily podcast to user: {} [{}]...", username, user_id);

    let thumbnail_path = "common_res/the_viper_room/podcast_cover.jpg";

    bot.send_audio(chat_id, InputFile::file(&podcast_path))
        .title(title)
        .performer("The Viper Room")
        .thumbnail(InputFile::file(thumbnail_path))
        .caption(&caption)
        .await?;

    info!(
        "Personal daily podcast for user: {} [{}] sent successfully!",
        username, user_id
    );

    Ok(())
}

pub async fn send_actual_daily_public_podcast(bot: Bot, chat_id: ChatId) -> Result<()> {
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

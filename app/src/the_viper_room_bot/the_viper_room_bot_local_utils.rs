use anyhow::anyhow;
use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Utc};
use core::grammers::grammers_functionality::initialize_grammers_client;
use core::state::tg_bot::app_state::BotAppState;
use grammers_client::types::{attributes::Attribute, InputMessage};
use grammers_client::Client as g_Client;
use core::utils::the_viper_room::news_block_creation::news_block_creation;
use core::utils::the_viper_room::news_block_creation_utils::generate_waveform;
use std::fs::{read_to_string, remove_file};
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::{ChatId, Requester};
use teloxide::Bot;
use tokio::time;
use tokio::time::Instant;
use tracing::error;
use tracing::log::info;

pub(crate) async fn generate_podcast(
    g_client: g_Client,
    bot: Bot,
    user_id: ChatId,
    app_state: Arc<BotAppState>,
    app_tg_account_id: &str,
    nickname: String,
    chat_username: &str,
) -> anyhow::Result<()> {
    info!("Starting podcast generation by /podcast cmd...");

    if !g_client.is_authorized().await? {
        bot.send_message(user_id, "System g_Client is NOT okay!")
            .await?;
    } else {
        bot.send_message(user_id, "System g_Client is okay!")
            .await?;
    }

    let podcast = news_block_creation(
        &g_client,
        app_tg_account_id,
        app_state,
        nickname,
        true,
    )
    .await?;

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

    for file in [&podcast, &podcast_caption_file] {
        match remove_file(file) {
            Ok(_) => info!("File {} deleted after broadcast!", file.display()),
            Err(e) => info!("Could not delete {}: {}", file.display(), e),
        }
    }

    Ok(())
}

pub(crate) async fn schedule_podcast(
    bot: Bot,
    user_id: ChatId,
    app_state: Arc<BotAppState>,
    app_tg_account_id: Arc<String>,
    nickname: String,
    session_data: Vec<u8>,
) -> anyhow::Result<()> {
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

pub(crate) async fn stop_daily_podcasts(app_state: Arc<BotAppState>) -> anyhow::Result<()> {
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

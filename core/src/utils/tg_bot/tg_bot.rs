use std::env;
use std::process::Command;
use crate::models::common::dialogue_cache::DialogueCache;
use crate::state::tg_bot::app_state::BotAppState;
use std::sync::Arc;
use teloxide::dispatching::{Dispatcher, UpdateHandler};
use teloxide::error_handlers::LoggingErrorHandler;
use teloxide::prelude::{ChatId, Message, Requester};
use teloxide::{dptree, Bot};
use teloxide::net::Download;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use anyhow::Result;

pub async fn check_username(bot: Bot, msg: Message) -> bool {
    if let Some(_username) = msg.chat.username() {
        true
    } else {
        let error_message = "Извините, но для использования приложения необходимо установить username в Telegram.\nПожалуйста, установите username в настройках что бы получить доступ к приложению";
        let _ = bot.send_message(msg.chat.id, error_message).await;
        false
    }
}

pub async fn run_bot_dispatcher(
    bot: Bot,
    handler: UpdateHandler<anyhow::Error>,
    app_state: Arc<BotAppState>,
) -> anyhow::Result<()> {
    Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![app_state])
        .enable_ctrlc_handler()
        .build()
        .dispatch_with_listener(
            teloxide::update_listeners::polling_default(bot).await,
            LoggingErrorHandler::with_custom_text("Dispatcher: an error from the update listener"),
        )
        .await;

    Err(anyhow::anyhow!("Bot dispatcher unexpectedly stopped"))
}

pub async fn add_user_message_to_cache(
    app_state: Arc<BotAppState>,
    user_id: ChatId,
    message: String,
) {
    let mut cache = app_state.temp_cache.lock().await;
    let chat_cache = cache
        .entry(user_id)
        .or_insert_with(|| DialogueCache::new(20));
    chat_cache.add_user_message(message);
}

pub async fn add_llm_response_to_cache(
    app_state: Arc<BotAppState>,
    user_id: ChatId,
    llm_response: String,
) {
    let mut cache = app_state.temp_cache.lock().await;
    let chat_cache = cache
        .entry(user_id)
        .or_insert_with(|| DialogueCache::new(20));
    chat_cache.add_llm_response_to_cache(llm_response);
}

pub async fn get_cache_as_string(app_state: Arc<BotAppState>, user_id: ChatId) -> String {
    let cache = app_state.temp_cache.lock().await;
    cache
        .get(&user_id)
        .map(|chat_cache| chat_cache.get_cache_as_string())
        .unwrap_or_else(|| "[]".to_string())
}

pub async fn download_voice(bot: &Bot, file_id: &str, save_path: &str) -> Result<String> {
    let base_path = env::current_dir()?.join(save_path);

    if let Some(parent_dir) = base_path.parent() {
        tokio::fs::create_dir_all(parent_dir).await?;
    }

    let mut destination = File::create(&base_path).await?;

    let file = bot.get_file(file_id).await?;
    bot.download_file(&file.path, &mut destination).await?;

    destination.flush().await?;

    Ok(base_path.to_str().unwrap().to_string())
}

pub fn check_whisper_installed() -> Result<(), anyhow::Error> {
    let output = Command::new("whisper-cli")
        .arg("--help")
        .output();

    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(anyhow::anyhow!(
            "Whisper CLI failed to respond correctly: {}",
            String::from_utf8_lossy(&output.stderr)
        )),
        Err(err) => Err(anyhow::anyhow!(
            "Whisper CLI not found: {}",
            err
        )),
    }
}

use crate::models::common::app_name::AppName;
use crate::models::common::dialogue_cache::DialogueCache;
use crate::models::common::system_messages::{AppsSystemMessages, ProbiotBotMessages};
use crate::state::tg_bot::app_state::BotAppState;
use crate::temp_cache::temp_cache_traits::TempCacheInit;
use crate::utils::common::get_message;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use teloxide::dispatching::{Dispatcher, UpdateHandler};
use teloxide::error_handlers::LoggingErrorHandler;
use teloxide::net::Download;
use teloxide::prelude::{ChatId, Message, Requester};
use teloxide::sugar::request::RequestReplyExt;
use teloxide::types::{
    ChatAction, InlineKeyboardButton, InlineKeyboardMarkup, MessageEntityKind, User,
};
use teloxide::{dptree, Bot};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::time::sleep;

pub async fn check_username_from_message(bot: &Bot, msg: &Message) -> bool {
    if msg.chat.username().is_some() {
        true
    } else {
        let error_message = "Извините, но для использования приложения необходимо установить username в Telegram.\n\
                            Пожалуйста, установите username в настройках что бы получить доступ к приложению";
        let _ = bot
            .send_message(msg.chat.id, error_message)
            .reply_to(msg.id)
            .await;
        false
    }
}

pub async fn check_username_from_user(bot: &Bot, user: &User, chat_id: ChatId) -> bool {
    if user.username.is_some() {
        true
    } else {
        let error_message = "Извините, но для использования приложения необходимо установить username в Telegram.\n\
                            Пожалуйста, установите username в настройках что бы получить доступ к приложению";
        let _ = bot.send_message(chat_id, error_message).await;
        false
    }
}

// pub fn get_username_from_message(msg: &Message) -> String {
//     msg.from
//         .as_ref()
//         .map(|user| {
//             if let Some(username) = &user.username {
//                 return username.to_string();
//             }
//
//             let first_name = user.first_name.trim();
//             let last_name = user.last_name.as_deref().unwrap_or("").trim();
//
//             match (first_name.is_empty(), last_name.is_empty()) {
//                 (false, false) => format!("{} {}", first_name, last_name),
//                 (false, true) => first_name.to_string(),
//                 (true, false) => last_name.to_string(),
//                 (true, true) => "mommy's_anon".to_string(),
//             }
//         })
//         .unwrap_or_else(|| "mommy's_anon".to_string())
// }

pub fn get_username_from_message(msg: &Message) -> String {
    msg.from
        .as_ref()
        .map(|user| get_username_from_user(user))
        .unwrap_or_else(|| "mommy's_anon".to_string())
}

pub fn get_username_from_user(user: &User) -> String {
    if let Some(username) = &user.username {
        return username.to_string();
    }

    let first_name = user.first_name.trim();
    let last_name = user.last_name.as_deref().unwrap_or("").trim();

    match (first_name.is_empty(), last_name.is_empty()) {
        (false, false) => format!("{} {}", first_name, last_name),
        (false, true) => first_name.to_string(),
        (true, false) => last_name.to_string(),
        (true, true) => "mommy's_anon".to_string(),
    }
}

pub fn get_chat_title(msg: &Message) -> String {
    msg.chat
        .title()
        .map(|title| title.to_string())
        .unwrap_or_else(|| "No Title Chat".to_string())
}

pub async fn is_bot_addressed(bot: &Bot, msg: &Message) -> Result<bool> {
    let bot_user = bot.get_me().await?;
    let bot_username = bot_user
        .username
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Bot has no username"))?;

    if let Some(reply_to) = &msg.reply_to_message() {
        if let Some(from) = &reply_to.from {
            if from.is_bot && from.id == bot_user.id {
                return Ok(true);
            }
        }
    }

    if let Some(text) = msg.text() {
        let mention = format!("@{}", bot_username);
        if text.contains(&mention) {
            return Ok(true);
        }
    }

    if let Some(entities) = msg.entities() {
        for entity in entities {
            match &entity.kind {
                MessageEntityKind::Mention => {
                    if let Some(text) = msg.text() {
                        let start = entity.offset;
                        let end = start + entity.length;
                        let mentioned = &text[start..end];
                        if mentioned == format!("@{}", bot_username) {
                            return Ok(true);
                        }
                    }
                }
                MessageEntityKind::TextMention { user } => {
                    if user.id == bot_user.id {
                        return Ok(true);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(false)
}

pub async fn run_bot_dispatcher(
    bot: Bot,
    main_handler: UpdateHandler<anyhow::Error>,
    app_state: Arc<BotAppState>,
    callback_query_handler: Option<UpdateHandler<anyhow::Error>>,
) -> Result<()> {
    let mut handler_tree = dptree::entry().branch(main_handler);

    if let Some(callback_handler) = callback_query_handler {
        handler_tree = handler_tree.branch(callback_handler);
    }

    Dispatcher::builder(bot.clone(), handler_tree)
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

pub async fn add_user_message_to_cache<T: TempCacheInit + Send + Sync>(
    app_state: Arc<T>,
    user_id: &str,
    message: &str,
) {
    let mut cache = app_state.get_temp_cache().lock().await;
    let chat_cache = cache
        .entry(user_id.to_string())
        .or_insert_with(|| DialogueCache::new(20));
    chat_cache.add_user_message(message.to_string());
}

pub async fn add_llm_response_to_cache<T: TempCacheInit + Send + Sync>(
    app_state: Arc<T>,
    user_id: &str,
    llm_response: &str,
) {
    let mut cache = app_state.get_temp_cache().lock().await;
    let chat_cache = cache
        .entry(user_id.to_string())
        .or_insert_with(|| DialogueCache::new(20));
    chat_cache.add_llm_response_to_cache(llm_response.to_string());
}

pub async fn get_cache_as_string<T: TempCacheInit + Send + Sync>(
    app_state: Arc<T>,
    user_id: &str,
) -> String {
    let cache = app_state.get_temp_cache().lock().await;
    cache
        .get(user_id)
        .map(|chat_cache| chat_cache.get_cache_as_string())
        .unwrap_or_else(|| "[]".to_string())
}

pub async fn download_voice(bot: &Bot, file_id: &str, save_path: &Path) -> Result<String> {
    if let Some(parent_dir) = save_path.parent() {
        tokio::fs::create_dir_all(parent_dir).await?;
    }

    let mut destination = File::create(save_path).await?;

    let file = bot.get_file(file_id).await?;
    bot.download_file(&file.path, &mut destination).await?;

    destination.flush().await?;

    Ok(save_path.to_string_lossy().into_owned())
}

pub async fn get_user_message_count<T: TempCacheInit + Send + Sync>(
    app_state: &Arc<T>,
    user_id: &str,
) -> usize {
    let cache = app_state.get_temp_cache().lock().await;
    cache
        .get(user_id)
        .map(|chat_cache| chat_cache.count_user_messages())
        .unwrap_or(0)
}

pub async fn start_bots_chat_action(
    bot: Bot,
    chat_id: ChatId,
    action: ChatAction,
    typing_flag: Arc<Mutex<bool>>,
) {
    tokio::spawn(async move {
        while *typing_flag.lock().await {
            bot.send_chat_action(chat_id, action.clone()).await.ok();
            sleep(Duration::from_secs(4)).await;
        }
    });
}

pub async fn stop_bots_chat_action(typing_flag: Arc<Mutex<bool>>) {
    *typing_flag.lock().await = false;
}

pub async fn append_footer_if_needed<T: TempCacheInit + Send + Sync>(
    llm_response: &str,
    app_state: Arc<T>,
    chat_id: &str,
    app_name: AppName,
) -> Result<String> {
    let message_count = get_user_message_count(&app_state, chat_id).await;

    if message_count > 0 && message_count % 3 == 0 {
        let footer_message = match app_name {
            AppName::ProbiotBot => {
                get_message(AppsSystemMessages::ProbiotBot(
                    ProbiotBotMessages::ResponseFooter,
                ))
                .await?
            }
            AppName::W3AWeb => {
                "".to_string()
                // // Uncomment to use ResponseFooter for W3AWeb App
                // get_message(AppsSystemMessages::W3ABot(W3ABotMessages::ResponseFooter)).await?
            }
            _ => "".to_string(),
        };

        if !footer_message.is_empty() {
            return Ok(format!("{}\n{}", llm_response, footer_message));
        }
    }

    Ok(llm_response.to_string())
}

pub fn create_tts_button(chat_id: ChatId, message_id: &str) -> InlineKeyboardMarkup {
    let callback_data = format!("tts:{}:{}", chat_id, message_id);
    InlineKeyboardMarkup::default().append_row(vec![InlineKeyboardButton::callback(
        "Озвучить ответ",
        &callback_data,
    )])
}

pub async fn save_tts_payload<T: TempCacheInit + Send + Sync>(
    app_state: Arc<T>,
    user_id: ChatId,
    message_id: &str,
    tts_payload: &str,
) {
    let mut cache = app_state.get_temp_cache().lock().await;
    let dialogue_cache = cache
        .entry(user_id.to_string())
        .or_insert_with(|| DialogueCache::new(100));
    dialogue_cache.add_tts_payload(message_id.to_string(), tts_payload.to_string());
}

pub async fn get_and_remove_tts_payload(
    app_state: Arc<BotAppState>,
    chat_id: ChatId,
    message_id: String,
) -> Option<String> {
    let chat_id_as_integer = chat_id.0;
    let user_id_as_str = chat_id_as_integer.to_string();
    let mut cache = app_state.temp_cache.lock().await;
    if let Some(dialogue_cache) = cache.get_mut(&user_id_as_str) {
        dialogue_cache.get_and_remove_tts_payload(message_id)
    } else {
        None
    }
}

pub fn create_app_tmp_dir(app_name: &AppName) -> std::io::Result<()> {
    let base_tmp = PathBuf::from("tmp");

    if !base_tmp.exists() {
        fs::create_dir_all(&base_tmp)?;
    }

    let app_tmp_dir = app_name.temp_dir();

    if !app_tmp_dir.exists() {
        fs::create_dir_all(&app_tmp_dir)?;
    }

    Ok(())
}

pub fn is_localdb_implemented(app_name: &AppName) -> bool {
    matches!(app_name, AppName::GrootBot)
}

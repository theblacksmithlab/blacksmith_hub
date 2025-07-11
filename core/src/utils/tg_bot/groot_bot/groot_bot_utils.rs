use crate::models::common::app_name::AppName;
use crate::models::common::system_messages::{AppsSystemMessages, GrootBotMessages};
use crate::models::tg_bot::groot_bot::groot_bot::ChatObject;
pub use crate::utils::common::{build_resource_file_path, get_message};
use anyhow::{Context, Result};
use serde_json::{from_reader, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{ChatId, Message, Requester};
use teloxide::types::ChatKind::Public;
use teloxide::types::PublicChatKind::Supergroup;
use teloxide::types::{ChatPublic, MessageId, ThreadId};
use teloxide::Bot;
use tokio::time::sleep;
use tracing::{error, info, warn};
use unicode_segmentation::UnicodeSegmentation;

pub fn load_super_admins(app_name: &AppName) -> HashSet<u64> {
    let path = build_resource_file_path(app_name, "super_admins_list.json");

    let data = fs::read_to_string(&path).unwrap_or_else(|err| {
        error!("Failed to read {}: {}", path.display(), err);
        "[]".to_string()
    });

    let json: Value = serde_json::from_str(&data).unwrap_or_else(|err| {
        error!("Failed to parse JSON in {}: {}", path.display(), err);
        Value::Array(vec![])
    });

    json.as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_u64())
        .collect()
}

pub fn load_paid_chats(app_name: &AppName) -> HashSet<i64> {
    let file_path = build_resource_file_path(app_name, "paid_chats.json");

    let data = fs::read_to_string(&file_path).unwrap_or_else(|err| {
        error!(
            "Unable to read paid_chats.json: {} from {}.",
            err,
            file_path.display()
        );
        "{}".to_string()
    });

    let json: Value = serde_json::from_str(&data).unwrap_or_else(|err| {
        error!("Failed to parse JSON in {}: {}", file_path.display(), err);
        Value::Object(serde_json::Map::new())
    });

    json["paidChats"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|id| id.as_i64())
        .collect()
}

pub fn load_white_listed_users(app_name: &AppName) -> HashSet<i64> {
    let path = build_resource_file_path(app_name, "white_listed_users.json");

    let data = fs::read_to_string(&path).unwrap_or_else(|err| {
        error!("Failed to read {}: {}", path.display(), err);
        "[]".to_string()
    });

    let json: Value = serde_json::from_str(&data).unwrap_or_else(|err| {
        error!("Failed to parse JSON in {}: {}", path.display(), err);
        Value::Array(vec![])
    });

    json.as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_i64())
        .collect()
}

pub fn load_black_listed_users(app_name: &AppName) -> HashSet<i64> {
    let path = build_resource_file_path(app_name, "black_listed_users.json");

    let data = fs::read_to_string(&path).unwrap_or_else(|err| {
        error!("Failed to read {}: {}", path.display(), err);
        "[]".to_string()
    });

    let json: Value = serde_json::from_str(&data).unwrap_or_else(|err| {
        error!("Failed to parse JSON in {}: {}", path.display(), err);
        Value::Array(vec![])
    });

    json.as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_i64())
        .collect()
}

fn is_forum(msg: &Message) -> bool {
    if let Public(ChatPublic {
        kind: Supergroup(supergroup),
        ..
    }) = &msg.chat.kind
    {
        return supergroup.is_forum;
    }
    false
}

pub async fn paid_chat_spam_warning(
    bot: Bot,
    msg: &Message,
    thread_id: Option<ThreadId>,
    bot_system_message_text: String,
    log_message: String,
    app_name: &AppName,
    chat_title: &str,
    username: &str,
) -> Result<()> {
    if is_forum(msg) {
        if let Some(thread_id) = thread_id {
            bot.delete_message(msg.chat.id, msg.id).await?;

            info!("{}", log_message);

            let bot_system_message = bot
                .send_message(msg.chat.id, bot_system_message_text)
                .message_thread_id(thread_id)
                .await?;

            auto_delete_message(
                bot.clone(),
                bot_system_message.chat.id,
                bot_system_message.id,
                Duration::from_secs(120),
            )
            .await;
        } else {
            // If the chat is_forum but the message doesn't have a thread_id (main thread case)
            bot.delete_message(msg.chat.id, msg.id).await?;

            info!("{}", log_message);

            let bot_system_message = bot
                .send_message(msg.chat.id, bot_system_message_text)
                .await?;

            auto_delete_message(
                bot.clone(),
                bot_system_message.chat.id,
                bot_system_message.id,
                Duration::from_secs(120),
            )
            .await;
        }
    } else {
        bot.delete_message(msg.chat.id, msg.id).await?;

        info!("{}", log_message);

        let sent_message = bot
            .send_message(msg.chat.id, bot_system_message_text)
            .await?;

        auto_delete_message(
            bot.clone(),
            sent_message.chat.id,
            sent_message.id,
            Duration::from_secs(120),
        )
        .await;
    }

    if let Some(user) = &msg.from {
        add_penalty_points(app_name, msg.chat.id.0, user.id.0, chat_title, &username).await;
    }

    Ok(())
}

pub async fn unpaid_chat_spam_warning(
    bot: Bot,
    msg: &Message,
    thread_id: Option<ThreadId>,
    chat_title: &str,
) -> Result<()> {
    info!("Unpaid chat | \'{}\' | tried to use Groot bot", chat_title);
    let demo_bot_system_message = get_message(AppsSystemMessages::GrootBot(
        GrootBotMessages::DemoBotSystemMessage,
    ))
    .await?;

    if is_forum(msg) {
        if let Some(thread_id) = thread_id {
            let bot_system_message = bot
                .send_message(msg.chat.id, demo_bot_system_message)
                .message_thread_id(thread_id)
                .await?;

            auto_delete_message(
                bot.clone(),
                bot_system_message.chat.id,
                bot_system_message.id,
                Duration::from_secs(30),
            )
            .await;
        } else {
            // If the chat is_forum but the message doesn't have a thread_id (main thread case)
            let bot_system_message = bot
                .send_message(msg.chat.id, demo_bot_system_message)
                .await?;

            auto_delete_message(
                bot.clone(),
                bot_system_message.chat.id,
                bot_system_message.id,
                Duration::from_secs(30),
            )
            .await;
        }
    } else {
        let bot_system_message = bot
            .send_message(msg.chat.id, demo_bot_system_message)
            .await?;

        auto_delete_message(
            bot.clone(),
            bot_system_message.chat.id,
            bot_system_message.id,
            Duration::from_secs(30),
        )
        .await;
    }

    Ok(())
}

pub async fn add_penalty_points(
    app_name: &AppName,
    chat_id: i64,
    user_id: u64,
    chat_title: &str,
    username: &str,
) {
    let file_path = build_resource_file_path(app_name, "penalty_points.json");
    let mut penalties: HashMap<i64, HashMap<u64, i32>> = HashMap::new();

    if Path::new(&file_path).exists() {
        if let Ok(data) = fs::read_to_string(&file_path) {
            if let Ok(json) = serde_json::from_str::<HashMap<i64, HashMap<u64, i32>>>(&data) {
                penalties = json;
            }
        }
    }

    let chat_penalties = penalties.entry(chat_id).or_insert_with(HashMap::new);
    let new_points = chat_penalties.entry(user_id).or_insert(0);
    *new_points += 1;

    let penalty_count = *new_points;

    info!(
            "[Penalty System] Penalty point added for user: {} with id: {} in chat: {} with id: {}. User has {} penalty points after update",
            username, user_id, chat_title, chat_id, *new_points
        );

    let penalties_serialized = match serde_json::to_string_pretty(&penalties) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize penalty points: {}", e);
            return;
        }
    };

    if let Err(e) = fs::write(&file_path, penalties_serialized) {
        error!("Failed to save penalty points: {}", e);
    }

    if penalty_count >= 7 {
        add_user_to_black_list(app_name, user_id).await;
    }
}

pub fn get_penalty_points(app_name: &AppName, chat_id: i64, user_id: u64) -> i32 {
    let file_path = build_resource_file_path(app_name, "penalty_points.json");

    if let Ok(data) = fs::read_to_string(&file_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&data) {
            if let Some(points) = json
                .get(&chat_id.to_string())
                .and_then(|chat| chat.get(&user_id.to_string()))
                .and_then(|val| val.as_i64())
            {
                return points as i32;
            }
        }
    }
    0
}

pub fn count_emojis(message: &str) -> usize {
    UnicodeSegmentation::graphemes(message, true)
        .filter(|grapheme| emojis::get(grapheme).is_some())
        .count()
}

pub fn load_restricted_words(app_name: &AppName) -> Vec<String> {
    let path = build_resource_file_path(app_name, "restricted_words.json");

    if !Path::new(&path).exists() {
        warn!("Restricted words file not found: {}", path.display());
        return vec![];
    }

    let file = match File::open(&path) {
        Ok(file) => file,
        Err(err) => {
            warn!("Failed to open restricted words file: {}", err);
            return vec![];
        }
    };

    let reader = BufReader::new(file);
    let data: Value = match from_reader(reader) {
        Ok(json) => json,
        Err(err) => {
            warn!("Failed to parse JSON in {}: {}", path.display(), err);
            return vec![];
        }
    };

    data.as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|word| word.as_str().map(|s| s.to_lowercase()))
        .collect()
}

pub fn parsing_restricted_words(app_name: &AppName, text: &str) -> bool {
    let restricted_words = load_restricted_words(app_name);
    let text_lower = text.to_lowercase();

    for word in &restricted_words {
        let word_lower = word.to_lowercase();

        if text_lower.contains(&word_lower) {
            return true;
        }
    }

    false
}

pub fn load_scam_domains(app_name: &AppName) -> Result<Vec<String>> {
    let file_path = build_resource_file_path(app_name, "scam_domains.json");

    let file = File::open(&file_path)
        .with_context(|| format!("Failed to open scam domains file: {}", file_path.display()))?;

    let reader = BufReader::new(file);

    let scam_domains: Vec<String> = from_reader(reader)
        .with_context(|| format!("Failed to parse JSON in file: {}", file_path.display()))?;

    Ok(scam_domains)
}

pub async fn add_user_to_black_list(app_name: &AppName, user_id: u64) {
    let file_path = build_resource_file_path(app_name, "black_listed_users.json");

    let mut black_list: HashSet<u64> = if Path::new(&file_path).exists() {
        match fs::read_to_string(&file_path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_else(|_| HashSet::new()),
            Err(_) => HashSet::new(),
        }
    } else {
        HashSet::new()
    };

    if black_list.contains(&user_id) {
        info!(
            "[Penalty System] User {} is already in the GLOBAL blacklist. Skipping...",
            user_id
        );
        return;
    }

    black_list.insert(user_id);

    if let Ok(json_data) = serde_json::to_string_pretty(&black_list) {
        if let Err(e) = fs::write(&file_path, json_data) {
            error!("Failed to update black_listed_users.json: {}", e);
        } else {
            info!(
                "[Penalty System] User {} has been added to the GLOBAL blacklist due to 10+ penalty points in a single chat.",
                user_id
            );
        }
    }
}

pub async fn is_message_from_linked_channel(bot: &Bot, msg: &Message) -> Result<bool> {
    if let Some(sender_chat) = &msg.sender_chat {
        if let Ok(Some(linked_channel_id)) = get_linked_channel_id(bot, msg.chat.id).await {
            return Ok(sender_chat.id.0 == linked_channel_id);
        }
    }
    Ok(false)
}

pub async fn get_linked_channel_id(
    bot: &Bot,
    chat_id: ChatId,
) -> Result<Option<i64>, reqwest::Error> {
    let token = bot.token();
    let url = format!("https://api.telegram.org/bot{}/getChat", token);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .query(&[("chat_id", chat_id.0.to_string())])
        .send()
        .await?;

    let json: Value = response.json().await?;

    if let Some(result) = json.get("result") {
        if let Some(linked_id) = result.get("linked_chat_id") {
            if let Some(id) = linked_id.as_i64() {
                return Ok(Some(id));
            }
        }
    }

    Ok(None)
}

pub fn load_chats_objects_from_file(app_name: &AppName) -> Result<Vec<ChatObject>> {
    let chats_path = build_resource_file_path(app_name, "chats_list.json");

    if !chats_path.exists() {
        return Err(anyhow::anyhow!(
            "Chats list file not found: {}",
            chats_path.display()
        ));
    }

    let data = fs::read_to_string(&chats_path)
        .with_context(|| format!("Failed to read chats list file: {}", chats_path.display()))?;

    let chats: Vec<ChatObject> = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse JSON in: {}", chats_path.display()))?;

    Ok(chats)
}

pub fn add_chat_to_file(app_name: &AppName, chat_object: ChatObject) -> Result<()> {
    let chats_path = build_resource_file_path(app_name, "chats_list.json");

    let mut chats: Vec<ChatObject> = if chats_path.exists() {
        let data = fs::read_to_string(&chats_path)
            .with_context(|| format!("Failed to read chats list file: {}", chats_path.display()))?;
        serde_json::from_str(&data).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };

    if chats.iter().any(|c| c.chat_id == chat_object.chat_id) {
        info!(
            "Chat: {} with id: {} already in the chats list. Continue",
            chat_object.username, chat_object.chat_id
        );
        return Ok(());
    }

    chats.push(chat_object.clone());

    let new_data = serde_json::to_string_pretty(&chats)
        .with_context(|| "Failed to serialize updated chat list")?;
    fs::write(&chats_path, new_data).with_context(|| {
        format!(
            "Failed to write updated chat list to file: {}",
            chats_path.display()
        )
    })?;

    info!(
        "New chat: {} with id: {} added to chats list file {}.",
        chat_object.username,
        chat_object.chat_id,
        chats_path.display()
    );

    Ok(())
}

pub async fn auto_delete_message(
    bot: Bot,
    chat_id: ChatId,
    message_id: MessageId,
    delay: Duration,
) {
    tokio::spawn(async move {
        sleep(delay).await;
        bot.delete_message(chat_id, message_id).await.ok();
    });
}

pub fn get_username(msg: &Message) -> String {
    msg.from
        .as_ref()
        .map(|user| {
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
        })
        .unwrap_or_else(|| "mommy's_anon".to_string())
}

pub fn get_chat_title(msg: &Message) -> String {
    msg.chat
        .title()
        .map(|title| title.to_string())
        .unwrap_or_else(|| "No Title Chat".to_string())
}

pub fn get_chat_username(msg: &Message) -> String {
    msg.chat
        .username()
        .map(|username| username.to_string())
        .unwrap_or_else(|| "_".to_string())
}

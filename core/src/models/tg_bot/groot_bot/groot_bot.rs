use crate::models::common::app_name::AppName;
use crate::utils::tg_bot::groot_bot::groot_bot_utils::{
    add_chat_to_file, build_resource_file_path, load_chats_objects_from_file,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use teloxide::macros::BotCommands;
use teloxide::types::ChatId;
use tracing::{error, info, warn};
use crate::telegram_client::telegram_client::TelegramAgent;

pub struct ResourcesDialogState {
    pub awaiting_option_choice: bool,
    pub awaiting_edit_type: bool,
    pub awaiting_show_type: bool,
    pub edit_type: EditType,
    pub show_type: ShowType,
    pub awaiting_data_entry: bool,
    pub awaiting_ask_message: bool,
}

#[derive(PartialEq, Eq)]
pub enum EditType {
    None,
    UsersToWhiteList,
    UsersToBlackList,
    Words,
}

#[derive(PartialEq, Eq)]
pub enum ShowType {
    None,
    UsersFromWhiteList,
    UsersFromBlackList,
    Words,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageCounts {
    pub counts: HashMap<i64, HashMap<u64, i32>>,
}

impl MessageCounts {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    pub async fn load_message_counts(app_name: &AppName) -> Result<Self> {
        let path = build_resource_file_path(app_name, "message_counts.json");

        let data = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());

        let message_counts: Self = serde_json::from_str(&data).unwrap_or_else(|_| Self::new());

        Ok(message_counts)
    }

    pub async fn save_message_counts(&self, app_name: &AppName) -> Result<()> {
        let path = build_resource_file_path(app_name, "message_counts.json");

        let data =
            serde_json::to_string_pretty(&self).context("Failed to serialize message counts")?;

        fs::write(&path, data).context(format!(
            "Failed to write message counts to {}",
            path.display()
        ))?;

        Ok(())
    }

    pub fn get_message_count(&self, chat_id: i64, user_id: u64) -> i32 {
        self.counts
            .get(&chat_id)
            .and_then(|users| users.get(&user_id))
            .cloned()
            .unwrap_or(0)
    }

    pub fn increment_message_count(&mut self, chat_id: i64, user_id: u64) {
        *self
            .counts
            .entry(chat_id)
            .or_insert_with(HashMap::new)
            .entry(user_id)
            .or_insert(0) += 1;
    }
}

pub struct ChatMessageStats {
    pub fetching_message_counts: HashMap<i64, HashMap<u64, u32>>,
}

impl ChatMessageStats {
    pub fn new() -> Self {
        Self {
            fetching_message_counts: HashMap::new(),
        }
    }

    pub async fn fetch_chat_history_for_single_chat(
        &self,
        chat_object: &ChatObject,
        telegram_agent: &TelegramAgent,
    ) -> Result<Vec<MessageIterationObject>> {
        let mut collected_messages = Vec::new();

        if let Some(chat_username) = telegram_agent.client.resolve_username(&chat_object.username).await? {
            let mut msgs = telegram_agent.client.iter_messages(chat_username).limit(5000);

            while let Some(msg) = msgs.next().await? {
                if let Some(sender) = msg.sender() {
                    let message_obj = MessageIterationObject {
                        user_id: sender.id() as u64,
                        username: sender.username().unwrap_or("Anonymous User").to_string(),
                    };
                    collected_messages.push(message_obj);
                }
            }
        } else {
            warn!(
                "Chat: {} with id: {} has no username set. Chat's history will NOT be fetched",
                chat_object.username, chat_object.chat_id
            );
        }

        Ok(collected_messages)
    }

    pub async fn fetch_chat_history_for_all_chats(&mut self, app_name: &AppName) -> Result<()> {
        info!("Fetching chats history at bot's start...");
        let g_client= match TelegramAgent::new(&app_name, "current.session").await {
            Ok(agent) => agent,
            Err(e) => {
                error!("Failed to initialize TelegramAgent: {}", e);
                return Err(e);
            }
        };

        let chats_objects_list = load_chats_objects_from_file(app_name)?;

        for chat_object in chats_objects_list {
            info!(
                "Fetching chat history for chat: {} with id: {}",
                chat_object.username, chat_object.chat_id
            );

            let messages = self
                .fetch_chat_history_for_single_chat(&chat_object, &g_client)
                .await?;

            info!(
                "Got {} messages for chat: {} with id: {}",
                messages.len(),
                chat_object.username,
                chat_object.chat_id
            );

            let mut user_message_count = HashMap::new();

            for msg in messages {
                *user_message_count.entry(msg.user_id).or_insert(0) += 1;
            }

            self.fetching_message_counts
                .insert(chat_object.chat_id, user_message_count);
        }

        Ok(())
    }

    pub async fn fetch_chat_history_for_new_chat(
        &mut self,
        app_name: &AppName,
        chat_id: ChatId,
        chat_username: &str,
    ) -> Result<()> {
        info!("Fetching chat history for a new chat...");
        let telegram_agent= match TelegramAgent::new(&app_name, "current.session").await {
            Ok(agent) => agent,
            Err(e) => {
                error!("Failed to initialize TelegramAgent: {}", e);
                return Err(e);
            }
        };

        let chat_object = ChatObject {
            chat_id: chat_id.0,
            username: chat_username.to_string(),
        };

        let messages = self
            .fetch_chat_history_for_single_chat(&chat_object, &telegram_agent)
            .await?;

        let mut user_message_count = HashMap::new();
        for msg in messages {
            *user_message_count.entry(msg.user_id).or_insert(0) += 1;
        }

        self.fetching_message_counts
            .insert(chat_object.chat_id, user_message_count);

        info!(
            "Chat history successfully fetched for chat: {} with id: {}. Users quantity in fetched messages: {}",
            chat_object.username,
            chat_object.chat_id,
            self.fetching_message_counts.get(&chat_object.chat_id).unwrap().len()
        );

        add_chat_to_file(app_name, chat_object.clone())?;

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageIterationObject {
    user_id: u64,
    username: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatObject {
    pub chat_id: i64,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageReports {
    pub reports: HashMap<String, u32>,
}

impl MessageReports {
    pub fn new() -> Self {
        Self {
            reports: HashMap::new(),
        }
    }

    pub async fn load_message_reports(app_name: &AppName) -> Result<Self> {
        let path = build_resource_file_path(app_name, "message_reports.json");

        let data = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());

        let message_reports: Self = serde_json::from_str(&data).unwrap_or_else(|_| Self::new());

        Ok(message_reports)
    }

    pub async fn save_message_reports(&self, app_name: &AppName) -> Result<()> {
        let path = build_resource_file_path(app_name, "message_reports.json");

        let data =
            serde_json::to_string_pretty(&self).context("Failed to serialize message reports")?;

        fs::write(&path, data).context(format!(
            "Failed to write message reports to {}",
            path.display()
        ))?;

        Ok(())
    }

    pub fn get_report_count(&self, chat_id: i64, message_id: i32) -> u32 {
        let key = format!("{}_{}", chat_id, message_id);
        *self.reports.get(&key).unwrap_or(&0)
    }

    pub fn add_report(&mut self, chat_id: i64, message_id: i32) -> u32 {
        let key = format!("{}_{}", chat_id, message_id);
        let count = self.reports.entry(key).or_insert(0);
        *count += 1;
        *count
    }
}

#[derive(BotCommands, Clone, PartialEq, Debug)]
#[command(rename_rule = "lowercase")]
pub enum GrootBotCommands {
    Start,
    About,
    Resources,
    Manual,
    Results,
    Groot,
    Subscription,
    Status,
    ForceSubscription,
}

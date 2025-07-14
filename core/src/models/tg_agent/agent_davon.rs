use std::collections::HashMap;
use crate::utils::tg_bot::groot_bot::groot_bot_utils::get_linked_channel_id;
use anyhow::Result;
use teloxide::prelude::Requester;
use teloxide::types::ChatId;
use teloxide::Bot;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct ReportedChatInfo {
    pub chat_id: i64,
    pub username: String,
    pub chat_title: String,
    pub owner: ChatMember,
    pub administrators: Vec<ChatMember>,
    pub linked_channel_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ChatMember {
    pub user_id: i64,
    pub username: Option<String>,
    pub first_name: String,
    pub last_name: Option<String>,
    pub role: MemberRole,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemberRole {
    Owner,
    Administrator,
}

impl ReportedChatInfo {
    pub async fn new(bot: &Bot, chat_id: i64) -> Result<Self> {
        let chat = bot
            .get_chat(ChatId(chat_id))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get chat info: {}", e))?;
        
        let chat_title = chat.title().unwrap_or("No Title Chat").to_string();
        let username = chat.username().unwrap_or("_").to_string();

        let linked_channel_id = match get_linked_channel_id(bot, ChatId(chat_id)).await {
            Ok(Some(id)) => {
                info!("Found linked channel {} for chat {}", id, chat_id);
                Some(id)
            }
            Ok(None) => {
                info!("No linked channel found for chat {}", chat_id);
                None
            }
            Err(e) => {
                warn!("Failed to get linked channel for chat {}: {}", chat_id, e);
                None
            }
        };

        let admins = bot
            .get_chat_administrators(ChatId(chat_id))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get chat administrators: {}", e))?;

        let mut owner = None;
        let mut administrators = Vec::new();

        for admin in admins {
            let chat_member = ChatMember {
                user_id: admin.user.id.0 as i64,
                username: admin.user.username.clone(),
                first_name: admin.user.first_name.clone(),
                last_name: admin.user.last_name.clone(),
                role: if admin.status() == teloxide::types::ChatMemberStatus::Owner {
                    MemberRole::Owner
                } else {
                    MemberRole::Administrator
                },
            };

            if admin.status() == teloxide::types::ChatMemberStatus::Owner {
                owner = Some(chat_member.clone());
            }

            administrators.push(chat_member);
        }

        let owner = owner.ok_or_else(|| anyhow::anyhow!("No owner found in chat"))?;

        Ok(Self {
            chat_id,
            username,
            chat_title,
            owner,
            administrators,
            linked_channel_id,
        })
    }

    pub fn get_all_admins(&self) -> Vec<&ChatMember> {
        self.administrators.iter().collect()
    }
}

#[derive(Debug, Clone)]
pub struct UserMessageCount {
    pub user_id: i64,
    pub username: Option<String>,
    pub message_count: u32,
}

#[derive(Debug)]
pub struct ChatMessageStats {
    pub chat_message_counts: HashMap<i64, HashMap<i64, u32>>,
}

impl ChatMessageStats {
    pub fn new() -> Self {
        Self {
            chat_message_counts: HashMap::new(),
        }
    }

    pub fn get_user_message_count(&self, chat_id: i64, user_id: i64) -> u32 {
        self.chat_message_counts
            .get(&chat_id)
            .and_then(|users| users.get(&user_id))
            .copied()
            .unwrap_or(0)
    }

    pub fn is_chat_stats_fetched(&self, chat_id: i64) -> bool {
        self.chat_message_counts.contains_key(&chat_id)
    }
}

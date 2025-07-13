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

    // pub fn get_owner_with_username(&self) -> Option<&ChatMember> {
    //     if self.owner.username.is_some() {
    //         Some(&self.owner)
    //     } else {
    //         None
    //     }
    // }

    // pub fn get_admins_with_username(&self) -> Vec<&ChatMember> {
    //     self.administrators
    //         .iter()
    //         .filter(|admin| admin.role == MemberRole::Administrator && admin.username.is_some())
    //         .collect()
    // }
}

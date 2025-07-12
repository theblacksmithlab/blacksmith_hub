use crate::telegram_client::telegram_client::TelegramAgent;
use anyhow::Result;
use grammers_client::types::Chat;
use tracing::info;

pub struct GrootBotAlias {
    pub bot_id: i64,
    pub bot_username: String,
}

impl GrootBotAlias {
    pub fn new(bot_id: i64, bot_username: String) -> Self {
        Self {
            bot_id,
            bot_username,
        }
    }

    pub async fn check_bot_presence(
        &self,
        telegram_agent: &TelegramAgent,
        chat: &Chat,
    ) -> Result<bool> {
        let packed_chat = chat.pack();

        let mut participants = telegram_agent.client.iter_participants(packed_chat);

        while let Some(participant) = participants.next().await? {
            if participant.user.id() == self.bot_id {
                info!("Bot {} found in chat {}", self.bot_id, chat.id());
                return Ok(true);
            }
        }
        info!("Packed chat: {}", packed_chat);
        info!("Chat: {:?}", chat);
        info!("Bot {} not found in chat {}", self.bot_id, chat.id());
        Ok(false)
    }

    pub async fn should_process_chat(
        &self,
        telegram_agent: &TelegramAgent,
        chat: &Chat,
    ) -> Result<bool> {
        let has_username = match chat {
            Chat::User(_) => false,
            Chat::Group(group) => group.username().is_some(),
            Chat::Channel(channel) => channel.username().is_some(),
        };

        if !has_username {
            info!("Chat {} is private (no username), skipping", chat.id());
            return Ok(false);
        }

        if self.check_bot_presence(telegram_agent, chat).await? {
            info!("Bot already present in chat {}, skipping", chat.id());
            return Ok(false);
        }

        info!(
            "Chat {} is public and bot not present, processing",
            chat.id()
        );
        Ok(true)
    }

    pub async fn get_chat_info_by_username(
        &self,
        telegram_agent: &TelegramAgent,
        chat_username: &str,
    ) -> Result<Option<(String, i64)>> {
        match telegram_agent.client.resolve_username(chat_username).await {
            Ok(Some(chat)) => {
                let title = chat.name().to_string();
                let chat_id = chat.id();
                Ok(Some((title, chat_id)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow::anyhow!(
                "Could not get chat info for {}: {}",
                chat_username,
                e
            )),
        }
    }

    pub async fn send_message_to_bot(
        &self,
        telegram_agent: &TelegramAgent,
        message: &str,
    ) -> Result<()> {
        let bot_chat = telegram_agent
            .client
            .resolve_username(&self.bot_username)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Bot {} not found", self.bot_username))?;

        telegram_agent
            .client
            .send_message(bot_chat.pack(), message)
            .await?;

        info!("Message sent to bot {}: {}", self.bot_username, message);
        Ok(())
    }
}

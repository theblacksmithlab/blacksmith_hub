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

    pub async fn should_process_chat(&self, chat: &Chat) -> Result<bool> {
        let has_username = match chat {
            Chat::User(_) => false,
            Chat::Group(group) => group.username().is_some(),
            Chat::Channel(channel) => channel.username().is_some(),
        };

        if !has_username {
            info!("Chat {} is private (no username), skipping", chat.id());
            return Ok(false);
        }

        info!("Chat {} is public (got username), processing", chat.id());
        Ok(true)
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

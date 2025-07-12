use crate::models::common::app_name::AppName;
use crate::models::tg_agent::bot_alias::GrootBotAlias;
use crate::utils::common::build_resource_file_path;
use anyhow::Result;
use chrono::{DateTime, Utc};
use grammers_client::types::{Chat, Message, Update, User};
use grammers_client::{Client as g_Client, Config as g_Config};
use grammers_session::Session;
use sqlx::{Row, SqlitePool};
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use std::{env, fs};
use tracing::log::warn;
use tracing::{error, info};

enum AnalysisResult {
    Spam,
    Clear,
}

pub struct TelegramAgent {
    pub client: g_Client,
}

impl TelegramAgent {
    pub async fn new(app_name: &AppName, session_file_name: &str) -> Result<Self> {
        let session_data = Self::load_session_data(app_name, session_file_name)?;
        let client = Self::initialize_client(session_data).await?;

        if !client.is_authorized().await? {
            return Err(anyhow::anyhow!(
                "Grammers client is not authorized! Session file may be invalid or expired."
            ));
        }

        info!("TelegramAgent initialized successfully");
        Ok(Self { client })
    }

    fn load_session_data(app_name: &AppName, session_file_name: &str) -> Result<Vec<u8>> {
        let session_path = build_resource_file_path(app_name, session_file_name);

        if !session_path.exists() {
            return Err(anyhow::anyhow!(
                "Session file not found: {}. Ensure the session file exists and is valid.",
                session_path.display()
            ));
        }

        fs::read(&session_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read session file {}: {}",
                session_path.display(),
                e
            )
        })
    }

    async fn initialize_client(session_data: Vec<u8>) -> Result<g_Client> {
        let api_id: i32 = env::var("TELEGRAM_API_ID")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_API_ID environment variable not set"))?
            .parse()
            .map_err(|_| anyhow::anyhow!("TELEGRAM_API_ID must be a valid number"))?;

        let api_hash = env::var("TELEGRAM_API_HASH")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_API_HASH environment variable not set"))?;

        let client = g_Client::connect(g_Config {
            session: Session::load(&session_data)
                .map_err(|e| anyhow::anyhow!("Failed to load session data: {}", e))?,
            api_id,
            api_hash,
            params: Default::default(),
        })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect Grammers client: {}", e))?;

        Ok(client)
    }

    pub async fn start_monitoring(
        &self,
        groot_bot_alias: GrootBotAlias,
        db_pool: SqlitePool,
    ) -> Result<()> {
        info!("Agent Davon is starting monitoring updates...");

        let me = self.client.get_me().await?;

        match me.last_name().is_some() {
            true => {
                info!(
                    "Monitoring as: {} {:?} ({})",
                    me.first_name(),
                    me.last_name(),
                    me.id()
                );
            }
            false => {
                info!("Monitoring as: {} ({})", me.first_name(), me.id());
            }
        }

        loop {
            match self.client.next_update().await {
                Ok(update) => {
                    if let Err(e) = self
                        .process_update(update, &groot_bot_alias, &db_pool, &me)
                        .await
                    {
                        error!("Error processing update: {}", e);
                    }
                }
                Err(e) => {
                    error!("Error receiving update: {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn process_update(
        &self,
        update: Update,
        groot_bot_alias: &GrootBotAlias,
        db_pool: &SqlitePool,
        me: &User,
    ) -> Result<()> {
        match update {
            Update::NewMessage(message) => {
                self.handle_new_message(message, groot_bot_alias, db_pool, me)
                    .await?;
            }
            _ => {
                info!("Agent Davon got non-text update... ignore");
            }
        }
        Ok(())
    }

    async fn handle_new_message(
        &self,
        message: Message,
        groot_bot_alias: &GrootBotAlias,
        db_pool: &SqlitePool,
        me: &User,
    ) -> Result<()> {
        if let Some(sender) = message.sender() {
            if sender.id() == me.id() {
                return Ok(());
            }

            if sender.id() == groot_bot_alias.bot_id {
                info!(
                    "Got message from bot: {}: {}",
                    groot_bot_alias.bot_username,
                    message.text()
                );
                return Ok(());
            }
        } else {
            return Ok(());
        }

        let chat = message.chat();

        if !groot_bot_alias.should_process_chat(self, &chat).await? {
            return Ok(());
        }

        let text = message.text();
        if text.is_empty() {
            return Ok(());
        }

        match self.analyze_message(&text).await {
            Ok(AnalysisResult::Spam) => {
                self.save_spam_message(&message, &chat, db_pool).await?;
                self.update_chat_stats(&chat, db_pool, true, &groot_bot_alias)
                    .await?;
            }
            Ok(AnalysisResult::Clear) => {
                self.update_chat_stats(&chat, db_pool, false, &groot_bot_alias)
                    .await?;
            }
            Err(e) => {
                warn!("Failed to analyze message: {}", e);
                self.update_chat_stats(&chat, db_pool, false, &groot_bot_alias)
                    .await?;
            }
        }

        Ok(())
    }

    async fn analyze_message(&self, text: &str) -> Result<AnalysisResult> {
        if text.contains("PUMP") || text.contains("🚀") {
            Ok(AnalysisResult::Spam)
        } else if text.contains("Send 0.1 ETH") || text.contains("giveaway") {
            Ok(AnalysisResult::Spam)
        } else {
            Ok(AnalysisResult::Clear)
        }
    }

    async fn save_spam_message(
        &self,
        message: &Message,
        chat: &Chat,
        db_pool: &SqlitePool,
    ) -> Result<()> {
        let sender = message.sender().unwrap();

        let username = if let Some(username) = sender.username() {
            username.to_string()
        } else {
            "mommy's_anon".to_string()
        };

        sqlx::query("INSERT INTO spam_messages (chat_id, user_id, username, message_text, detected_at) VALUES (?, ?, ?, ?, ?)")
            .bind(chat.id())
            .bind(sender.id())
            .bind(username)
            .bind(message.text())
            .bind(Utc::now().to_rfc3339())
            .execute(db_pool)
            .await?;

        info!("Saved spam message from chat {}", chat.id());
        Ok(())
    }

    async fn update_chat_stats(
        &self,
        chat: &Chat,
        db_pool: &SqlitePool,
        is_spam: bool,
        groot_bot_alias: &GrootBotAlias,
    ) -> Result<()> {
        let chat_id = chat.id();
        let chat_title = chat.name().to_string();
        let chat_username = chat.username().map(|u| u.to_string());

        let existing = sqlx::query("SELECT spam_count, total_messages, first_message_time, status, last_report_sent FROM chat_monitoring WHERE chat_id = ?")
            .bind(chat_id)
            .fetch_optional(db_pool)
            .await?;

        let (new_spam_count, new_total_messages, first_time, should_update) = match existing {
            Some(record) => {
                let status: String = record.get("status");
                let last_report_sent: Option<String> = record.get("last_report_sent");

                match status.as_str() {
                    "not_relevant" => {
                        return Ok(());
                    }
                    "silence" => {
                        if let Some(last_sent_str) = last_report_sent {
                            if let Ok(last_sent_time) = DateTime::parse_from_rfc3339(&last_sent_str)
                            {
                                let elapsed = Utc::now() - last_sent_time.with_timezone(&Utc);
                                if elapsed.num_days() < 11 {
                                    return Ok(());
                                } else {
                                    let spam_count = if is_spam { 1 } else { 0 };
                                    let total_count = 1;
                                    let first_time = Utc::now().to_rfc3339();
                                    (spam_count, total_count, Some(first_time), true)
                                }
                            } else {
                                return Ok(());
                            }
                        } else {
                            return Ok(());
                        }
                    }
                    "collecting" => {
                        let spam_count: i32 = record.get("spam_count");
                        let total_messages: i32 = record.get("total_messages");
                        let first_time: Option<String> = record.get("first_message_time");

                        let new_spam = if is_spam { spam_count + 1 } else { spam_count };
                        let new_total = total_messages + 1;
                        (new_spam, new_total, first_time, true)
                    }
                    _ => {
                        let spam_count: i32 = record.get("spam_count");
                        let total_messages: i32 = record.get("total_messages");
                        let first_time: Option<String> = record.get("first_message_time");

                        let new_spam = if is_spam { spam_count + 1 } else { spam_count };
                        let new_total = total_messages + 1;
                        (new_spam, new_total, first_time, true)
                    }
                }
            }
            None => {
                let spam_count = if is_spam { 1 } else { 0 };
                let total_count = 1;
                let first_time = Utc::now().to_rfc3339();
                (spam_count, total_count, Some(first_time), true)
            }
        };

        if should_update {
            sqlx::query("INSERT OR REPLACE INTO chat_monitoring (chat_id, chat_title, chat_username, first_message_time, spam_count, total_messages, status) VALUES (?, ?, ?, ?, ?, ?, ?)")
                .bind(chat_id)
                .bind(chat_title)
                .bind(chat_username)
                .bind(&first_time)
                .bind(new_spam_count)
                .bind(new_total_messages)
                .bind("collecting")
                .execute(db_pool)
                .await?;

            if let Some(first_time_str) = first_time {
                self.check_report_ready(
                    chat_id,
                    &first_time_str,
                    new_spam_count,
                    db_pool,
                    groot_bot_alias,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn check_report_ready(
        &self,
        chat_id: i64,
        first_time_str: &str,
        spam_count: i32,
        db_pool: &SqlitePool,
        groot_bot: &GrootBotAlias,
    ) -> Result<()> {
        let first_time = DateTime::parse_from_rfc3339(first_time_str)?;
        let elapsed = Utc::now() - first_time.with_timezone(&Utc);

        if elapsed.num_minutes() >= 15 {
            // if elapsed.num_days() >= 3 {
            if spam_count >= 1 {
                info!(
                    "Sending report for chat {}: {} spam messages in {} days",
                    chat_id,
                    spam_count,
                    elapsed.num_days()
                );

                self.send_report(chat_id, db_pool, groot_bot).await?;
            } else {
                info!(
                    "Chat {} not relevant: only {} spam messages in {} days",
                    chat_id,
                    spam_count,
                    elapsed.num_days()
                );

                sqlx::query("UPDATE chat_monitoring SET status = 'not_relevant' WHERE chat_id = ?")
                    .bind(chat_id)
                    .execute(db_pool)
                    .await?;
            }
        }

        Ok(())
    }

    async fn send_report(
        &self,
        chat_id: i64,
        db_pool: &SqlitePool,
        groot_bot_alias: &GrootBotAlias,
    ) -> Result<()> {
        let spam_messages = sqlx::query("SELECT user_id, username, message_text, detected_at FROM spam_messages WHERE chat_id = ? ORDER BY detected_at")
            .bind(chat_id)
            .fetch_all(db_pool)
            .await?;

        let csv_filename = format!("{}_report.csv", chat_id);
        let csv_path = format!("common_res/agent_davon/reports/{}", csv_filename);

        if !Path::new(&csv_path).exists() {
            if let Some(parent) = Path::new(&csv_path).parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
        }

        let mut file = fs::File::create(&csv_path)?;

        file.write_all(b"\xEF\xBB\xBF")?;

        let mut wtr = csv::WriterBuilder::new().from_writer(file);

        wtr.write_record(&["user_id", "username", "message_text", "detected_at"])?;

        for row in spam_messages {
            let user_id: i64 = row.get("user_id");
            let username: Option<String> = row.get("username");
            let message_text: String = row.get("message_text");
            let detected_at: String = row.get("detected_at");

            wtr.write_record(&[
                user_id.to_string(),
                username.unwrap_or_else(|| "Unknown".to_string()),
                message_text,
                detected_at,
            ])?;
        }

        wtr.flush()?;

        info!("CSV report created: {}", csv_path);

        let command = format!("/agent_report {}", chat_id);
        groot_bot_alias.send_message_to_bot(self, &command).await?;

        info!("Report command sent to bot for chat {}", chat_id);

        sqlx::query(
            "UPDATE chat_monitoring SET status = 'silence', last_report_sent = ? WHERE chat_id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(chat_id)
        .execute(db_pool)
        .await?;

        Ok(())
    }
}

use crate::ai::common::common::raw_llm_processing_json;
use crate::models::common::ai::LlmModel;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::AgentDavonRoleType;
use crate::models::tg_agent::agent_davon::{ChatMember, MemberRole};
use crate::models::tg_agent::bot_alias::GrootBotAlias;
use crate::state::tg_agent::app_state::AgentAppState;
use crate::utils::common::{build_resource_file_path, get_system_role_or_fallback};
use anyhow::Result;
use chrono::{DateTime, Utc};
use grammers_client::types::{Chat, Message, Update, User};
use grammers_client::{Client as g_Client, Config as g_Config};
use grammers_session::Session;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs};
use tracing::log::warn;
use tracing::{debug, error, info};

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
        app_state: Arc<AgentAppState>,
    ) -> Result<()> {
        let me = self.client.get_me().await?;
        let last_name = me.last_name().unwrap_or("");

        info!(
            "Monitoring as: {} {} [id: {}]",
            me.first_name(),
            last_name,
            me.id()
        );

        loop {
            match self.client.next_update().await {
                Ok(update) => {
                    if let Err(e) = self
                        .process_update(update, &groot_bot_alias, app_state.clone(), &me)
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
        app_state: Arc<AgentAppState>,
        me: &User,
    ) -> Result<()> {
        match update {
            Update::NewMessage(message) => {
                self.handle_new_message(message, groot_bot_alias, app_state, me)
                    .await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_new_message(
        &self,
        message: Message,
        groot_bot_alias: &GrootBotAlias,
        app_state: Arc<AgentAppState>,
        me: &User,
    ) -> Result<()> {
        let chat = message.chat();
        let chat_title = chat.name().to_string();
        let _chat_username = chat.username().map(|u| u.to_string());

        if let Some(sender) = message.sender() {
            if sender.id() == groot_bot_alias.bot_id && chat.id() == groot_bot_alias.bot_id {
                let text = message.text();
                return if let Some((chat_id, response_details)) =
                    self.parse_report_response(&text).await?
                {
                    info!(
                        "Received report response for chat [id: {}]: {}",
                        chat_id, response_details
                    );
                    self.process_report_response(chat_id, response_details, &app_state.db_pool)
                        .await?;
                    Ok(())
                } else {
                    info!("Got message from Groot Bot but not a report response, ignoring");
                    Ok(())
                };
            }

            if sender.id() == chat.id() {
                info!(
                    "Skipping message from chat {} [id: {}] writing to itself",
                    chat_title,
                    chat.id()
                );
                return Ok(());
            }

            if sender.id() == me.id() {
                info!("Skipping message from Telegram Agent writing to the chat");
                return Ok(());
            }

            if sender.id() == groot_bot_alias.bot_id {
                info!(
                    "Skipping message from Groot Bot writing to chat {} [id: {}]",
                    chat_title,
                    chat.id()
                );
                return Ok(());
            }

            // // TEMP
            // use grammers_client::grammers_tl_types as tl;
            // 
            // if let Some(sender) = message.sender() {
            //     if let Chat::User(user) = sender {
            //         let access_hash = user.raw.access_hash.unwrap_or_default();
            //         let input_user = tl::types::InputUser {
            //             user_id: user.id(),
            //             access_hash,
            //         };
            // 
            //         let request = tl::functions::users::GetFullUser {
            //             id: input_user.into(),
            //         };
            // 
            //         match self.client.invoke(&request).await {
            //             Ok(result) => match result {
            //                 tl::enums::users::UserFull::Full(user_full_wrapper) => {
            //                     let full_user = &user_full_wrapper.full_user;
            // 
            //                     match full_user {
            //                         tl::enums::UserFull::Full(actual_user_full) => {
            //                             info!("User ID: {}", actual_user_full.id);
            // 
            //                             if let Some(photo) = actual_user_full.profile_photo.clone()
            //                             {
            //                                 info!("Has profile photo: {:?}", photo);
            //                             }
            // 
            //                             if let Some(about) = &actual_user_full.about {
            //                                 info!("User bio: {}", about);
            //                             }
            // 
            //                             if let Some(bot_info) = &actual_user_full.bot_info {
            //                                 info!("Bot info available: {:?}", bot_info);
            //                             }
            // 
            //                             if let Some(personal_channel_id) =
            //                                 actual_user_full.personal_channel_id
            //                             {
            //                                 info!("Personal channel ID: {}", personal_channel_id);
            //                             }
            // 
            //                             info!(
            //                                 "Common chats count: {}",
            //                                 actual_user_full.common_chats_count
            //                             );
            //                         }
            //                     }
            //                 }
            //             },
            //             Err(e) => {
            //                 warn!("Failed to get full user info: {}", e);
            //             }
            //         }
            //     }
            // }
            // // TEMP

            let stats_fetched = {
                let stats = app_state.chat_message_stats.lock().await;
                stats.is_chat_stats_fetched(chat.id())
            };

            if !stats_fetched {
                info!(
                    "Message stats not fetched for chat {} [id: {}], fetching...",
                    chat_title,
                    chat.id(),
                );
                if let Err(e) = self
                    .fetch_chat_message_stats(&chat, &app_state, &chat_title)
                    .await
                {
                    warn!(
                        "Failed to fetch message stats for chat {} [id: {}]: {}",
                        chat_title,
                        chat.id(),
                        e
                    );
                } else {
                    info!(
                        "Successfully fetched message stats for chat {} [id: {}]",
                        chat_title,
                        chat.id()
                    );
                }
            }

            let user_message_count = {
                let stats = app_state.chat_message_stats.lock().await;
                stats.get_user_message_count(chat.id(), sender.id())
            };

            if user_message_count >= 500 {
                info!(
                    "Skipping message from active user {} ({}+ messages) in chat {} [id: {}]",
                    sender.id(),
                    user_message_count,
                    chat_title,
                    chat.id()
                );
                self.update_chat_stats(&chat, &app_state.db_pool, false, &groot_bot_alias)
                    .await?;
                return Ok(());
            }
        } else {
            info!(
                "No sender in message.sender(), probably got message from chat: {:?} writing to itself: {}",
                message.sender(),
                chat.id()
            );
            return Ok(());
        }

        if !groot_bot_alias.should_process_chat(&chat).await? {
            return Ok(());
        }

        let admins_fetched = self
            .are_admins_fetched(chat.id(), &app_state.db_pool)
            .await?;

        if !admins_fetched {
            if let Err(e) = self
                .fetch_chat_admins_grammers(&chat, &app_state.db_pool)
                .await
            {
                warn!("Failed to fetch admins for chat {}: {}", chat.id(), e);
            }
        }

        if let Some(sender) = message.sender() {
            if self
                .is_sender_admin(sender.id(), chat.id(), &app_state.db_pool)
                .await?
            {
                info!(
                    "Skipping message from admin {} in chat {} [id: {}]",
                    sender.id(),
                    chat_title,
                    chat.id()
                );
                return Ok(());
            }
        }

        if let Ok(status) = self.get_chat_status(chat.id(), &app_state.db_pool).await {
            if status == "not_relevant" {
                info!(
                    "Skipping message from chat {} [id: {}] - marked as not_relevant",
                    chat_title,
                    chat.id()
                );
                return Ok(());
            }
            if status == "silence" {
                if let Ok(should_resume) = self
                    .should_resume_monitoring(chat.id(), &app_state.db_pool)
                    .await
                {
                    if should_resume {
                        info!(
                            "Resuming monitoring for chat {} - silence period expired",
                            chat.id()
                        );
                        sqlx::query("UPDATE chat_monitoring SET status = 'collecting', spam_count = 0, total_messages = 0, first_message_time = ? WHERE chat_id = ?")
                            .bind(Utc::now().to_rfc3339())
                            .bind(chat.id())
                            .execute(app_state.db_pool.as_ref())
                            .await?;
                    } else {
                        info!(
                            "Skipping message from chat {} [id: {}] - still in silence period",
                            chat_title,
                            chat.id()
                        );
                        return Ok(());
                    }
                } else {
                    info!(
                        "Skipping message from chat {} [id: {}] in silence period",
                        chat_title,
                        chat.id()
                    );
                    return Ok(());
                }
            }
        }

        let text = message.text();

        info!("Raw message text: {:?}", text);
        info!("Text bytes: {:?}", text.as_bytes());
        
        if text.is_empty() {
            return Ok(());
        }

        match self.analyze_message(&text, app_state.clone()).await {
            Ok(AnalysisResult::Spam) => {
                debug!("debugging analyze_message 1");
                self.update_chat_stats(&chat, &app_state.db_pool, true, &groot_bot_alias)
                    .await?;
                debug!("debugging analyze_message 2");
                self.save_spam_message(&message, &chat, &app_state.db_pool)
                    .await?;
            }
            Ok(AnalysisResult::Clear) => {
                debug!("debugging analyze_message 3");
                self.update_chat_stats(&chat, &app_state.db_pool, false, &groot_bot_alias)
                    .await?;
            }
            Err(e) => {
                warn!("Failed to analyze message: {}", e);
                debug!("debugging analyze_message 4");
                self.update_chat_stats(&chat, &app_state.db_pool, false, &groot_bot_alias)
                    .await?;
            }
        }

        Ok(())
    }

    async fn analyze_message(
        &self,
        text: &str,
        app_state: Arc<AgentAppState>,
    ) -> Result<AnalysisResult> {
        let system_role = get_system_role_or_fallback(
            &AppName::AgentDavon,
            AgentDavonRoleType::MessageCheck,
            None,
        );

        debug!("analyzing message: {}", text);
        let scam_detection_result =
            raw_llm_processing_json(&system_role, text, app_state, LlmModel::Complex2).await?;

        let is_scam: bool = match serde_json::from_str::<serde_json::Value>(&scam_detection_result)
        {
            Ok(json) => match json.get("is_scam") {
                Some(value) => match value.as_bool() {
                    Some(is_scam) => is_scam,
                    None => {
                        error!("'is_scam' value is not a boolean: {}", value);
                        false
                    }
                },
                None => {
                    error!("No 'is_scam' field in response: {}", json);
                    false
                }
            },
            Err(err) => {
                error!(
                    "Failed to parse JSON response: '{}'. Error: {}",
                    scam_detection_result, err
                );
                false
            }
        };
        
        debug!("LLM response: {}", scam_detection_result);
        debug!("LLM response result: {}", is_scam);
        
        if is_scam {
            info!(
                "🚨 Spam detected in message: {}",
                text.chars().take(50).collect::<String>()
            );
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

        // let username = if let Chat::User(user) = sender.clone() {
        //     if let Some(username) = user.username() {
        //         username.to_string()
        //     } else {
        //         let first = user.first_name();
        //         let last = user.last_name();
        //
        //         match (first, last) {
        //             (f, Some(l)) => format!("{} {}", f, l),
        //             (f, None) => f.to_string()
        //         }
        //     }
        // } else {
        //     sender.name().to_string()
        // };

        let username = if let Chat::User(user) = sender.clone() {
            if let Some(username) = user.username() {
                username.to_string()
            } else {
                "mommy's_anon".to_string()
            }
        } else {
            sender.username().as_deref().unwrap_or("_").to_string()
        };

        sqlx::query("INSERT INTO spam_messages (chat_id, user_id, username, message_text, detected_at) VALUES (?, ?, ?, ?, ?)")
            .bind(chat.id())
            .bind(sender.id())
            .bind(username)
            .bind(message.text())
            .bind(Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .execute(db_pool)
            .await?;
        debug!("debugging analyze_message 6");
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
        
        debug!("debugging update_chat_stats 1");
        
        let existing = sqlx::query("SELECT spam_count, total_messages, first_message_time, status, last_report_sent FROM chat_monitoring WHERE chat_id = ?")
            .bind(chat_id)
            .fetch_optional(db_pool)
            .await?;

        debug!("debugging update_chat_stats 2");
        
        let (new_spam_count, new_total_messages, first_time, should_update) = match existing {
            Some(record) => {
                debug!("debugging update_chat_stats 3");
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
                                    let first_time =
                                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
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
                let first_time = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
                (spam_count, total_count, Some(first_time), true)
            }
        };
        debug!("debugging update_chat_stats 4");
        if should_update {
            sqlx::query("INSERT OR REPLACE INTO chat_monitoring (chat_id, chat_title, chat_username, first_message_time, spam_count, total_messages, status) VALUES (?, ?, ?, ?, ?, ?, ?)")
                .bind(chat_id)
                .bind(chat_title.clone())
                .bind(chat_username)
                .bind(&first_time)
                .bind(new_spam_count)
                .bind(new_total_messages)
                .bind("collecting")
                .execute(db_pool)
                .await?;

            debug!("debugging update_chat_stats 5");
            
            if let Some(first_time_str) = first_time {
                self.check_report_ready(
                    chat_id,
                    &chat_title,
                    &first_time_str,
                    new_spam_count,
                    db_pool,
                    groot_bot_alias,
                )
                .await?;
            }
        }

        debug!("debugging update_chat_stats 6");
        
        Ok(())
    }

    async fn check_report_ready(
        &self,
        chat_id: i64,
        chat_title: &str,
        first_time_str: &str,
        spam_count: i32,
        db_pool: &SqlitePool,
        groot_bot: &GrootBotAlias,
    ) -> Result<()> {
        debug!("check_report_ready START: chat_title='{}', first_time_str='{}'", chat_title, first_time_str);

        debug!("Parsing RFC3339 time...");
        let first_time = DateTime::parse_from_rfc3339(first_time_str)?;
        debug!("RFC3339 parsed successfully");
        let elapsed = Utc::now() - first_time.with_timezone(&Utc);

        if elapsed.num_days() >= 3 {
            if spam_count >= 10 {
                info!(
                    "Sending report for chat {} [id: {}]: {} spam messages in {} days",
                    chat_title,
                    chat_id,
                    spam_count,
                    elapsed.num_days()
                );

                self.send_report(chat_id, db_pool, groot_bot).await?;
            } else {
                info!(
                    "Chat {} [id: {}] not relevant: only {} spam messages in {} days",
                    chat_title,
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

        let chat_data =
            sqlx::query("SELECT chat_title, chat_username FROM chat_monitoring WHERE chat_id = ?")
                .bind(chat_id)
                .fetch_one(db_pool)
                .await?;

        let chat_title: String = chat_data.get("chat_title");
        let chat_username: Option<String> = chat_data.get("chat_username");

        let bot_api_chat_id = -(1000000000000 + chat_id);
        let csv_filename = format!("{}_report.csv", bot_api_chat_id);
        let csv_path = format!("common_res/agent_davon/reports/{}", csv_filename);

        if let Some(parent) = Path::new(&csv_path).parent() {
            if !parent.exists() {
                match fs::create_dir_all(parent) {
                    Ok(_) => info!("Created directory: {}", parent.display()),
                    Err(e) => {
                        error!(
                            "Failed to create directory {}: {}. Skipping report.",
                            parent.display(),
                            e
                        );
                        return Ok(());
                    }
                }
            }
        }

        let mut file = match fs::File::create(&csv_path) {
            Ok(file) => {
                info!("Created CSV file: {}", csv_path);
                file
            }
            Err(e) => {
                error!(
                    "Failed to create CSV file {}: {}. Skipping report.",
                    csv_path, e
                );
                return Ok(());
            }
        };

        if let Err(e) = file.write_all(b"\xEF\xBB\xBF") {
            error!("Failed to write BOM to CSV: {}. Skipping report.", e);
            return Ok(());
        }

        let mut wtr = csv::WriterBuilder::new().from_writer(file);

        if let Err(e) = wtr.write_record(&[
            "chat_title",
            "chat_username",
            "user_id",
            "username",
            "message_text",
            "detected_at",
        ]) {
            error!("Failed to write CSV headers: {}. Skipping report.", e);
            return Ok(());
        }

        for row in spam_messages {
            let user_id: i64 = row.get("user_id");
            let username: Option<String> = row.get("username");
            let message_text: String = row.get("message_text");
            let detected_at: String = row.get("detected_at");

            if let Err(e) = wtr.write_record(&[
                &chat_title,
                &chat_username.clone().unwrap_or_else(|| "_".to_string()),
                &user_id.to_string(),
                &username.unwrap_or_else(|| "mommy's_anon".to_string()),
                &message_text,
                &detected_at,
            ]) {
                error!(
                    "Failed to write CSV record: {}. Continuing with next record.",
                    e
                );
                continue;
            }
        }

        if let Err(e) = wtr.flush() {
            error!("Failed to flush CSV file: {}. File may be incomplete.", e);
        }

        info!("CSV report created: {}", csv_path);

        let admins = sqlx::query("SELECT user_id, role FROM chat_admins WHERE chat_id = ?")
            .bind(chat_id)
            .fetch_all(db_pool)
            .await?;

        let admins_filename = format!("{}_admins.csv", bot_api_chat_id);
        let admins_path = format!("common_res/agent_davon/reports/{}", admins_filename);

        match fs::File::create(&admins_path) {
            Ok(mut file) => {
                info!("Created admins CSV file: {}", admins_path);

                if let Err(e) = file.write_all(b"\xEF\xBB\xBF") {
                    error!("Failed to write BOM to admins CSV: {}. Continuing.", e);
                } else {
                    let mut admins_wtr = csv::WriterBuilder::new().from_writer(file);

                    if let Err(e) =
                        admins_wtr.write_record(&["chat_title", "chat_username", "user_id", "role"])
                    {
                        error!("Failed to write admins CSV headers: {}. Continuing.", e);
                    } else {
                        for admin in admins {
                            let user_id: i64 = admin.get("user_id");
                            let role: String = admin.get("role");

                            if let Err(e) = admins_wtr.write_record(&[
                                &chat_title,
                                &chat_username.clone().unwrap_or_else(|| "_".to_string()),
                                &user_id.to_string(),
                                &role,
                            ]) {
                                error!("Failed to write admin record: {}. Continuing with next record.", e);
                                continue;
                            }
                        }

                        if let Err(e) = admins_wtr.flush() {
                            error!(
                                "Failed to flush admins CSV file: {}. File may be incomplete.",
                                e
                            );
                        } else {
                            info!("Admins CSV report created: {}", admins_path);
                        }
                    }
                }
            }
            Err(e) => {
                error!(
                    "Failed to create admins CSV file {}: {}. Continuing without admin data.",
                    admins_path, e
                );
            }
        }

        let command = format!("/agent_report {}", bot_api_chat_id);
        match groot_bot_alias.send_message_to_bot(self, &command).await {
            Ok(_) => info!(
                "Report command sent to bot for chat {} (Bot API: {})",
                chat_id, bot_api_chat_id
            ),
            Err(e) => error!("Failed to send report command to bot: {}", e),
        }

        info!(
            "Report command sent to bot for chat {} [id: {}]",
            chat_title, chat_id
        );

        match sqlx::query(
            "UPDATE chat_monitoring SET status = 'silence', last_report_sent = ? WHERE chat_id = ?",
        )
        .bind(Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .bind(chat_id)
        .execute(db_pool)
        .await
        {
            Ok(_) => info!(
                "Updated chat {} [id: {}] status to 'silence' - monitoring suspended",
                chat_title, chat_id
            ),
            Err(e) => error!("Database update failed for chat {}: {}", chat_id, e),
        }

        Ok(())
    }

    async fn are_admins_fetched(&self, chat_id: i64, db_pool: &SqlitePool) -> Result<bool> {
        let count = sqlx::query("SELECT COUNT(*) as count FROM chat_admins WHERE chat_id = ?")
            .bind(chat_id)
            .fetch_one(db_pool)
            .await?;

        let admin_count: i64 = count.get("count");
        Ok(admin_count > 0)
    }

    async fn fetch_chat_admins_grammers(&self, chat: &Chat, db_pool: &SqlitePool) -> Result<()> {
        let mut owner = None;
        let mut administrators: Vec<ChatMember> = Vec::new();
        let chat_title = chat.name().to_string();

        for attempt in 1..=3 {
            let mut participants = self.client.iter_participants(chat.pack());
            let mut found_new_owner = false;
            let mut found_new_admins = false;

            while let Some(participant) = participants.next().await? {
                match &participant.role {
                    grammers_client::types::Role::Creator(_) => {
                        if owner.is_none() {
                            owner = Some(ChatMember {
                                user_id: participant.user.id(),
                                username: participant.user.username().map(|u| u.to_string()),
                                first_name: participant.user.first_name().to_string(),
                                last_name: participant.user.last_name().map(|l| l.to_string()),
                                role: MemberRole::Owner,
                            });
                            found_new_owner = true;
                            info!(
                                "Owner found on attempt {} for chat {} [id: {}]",
                                attempt,
                                chat_title,
                                chat.id()
                            );
                        }
                    }
                    grammers_client::types::Role::Admin(_) => {
                        let admin_id = participant.user.id();
                        if !administrators.iter().any(|a| a.user_id == admin_id) {
                            administrators.push(ChatMember {
                                user_id: admin_id,
                                username: participant.user.username().map(|u| u.to_string()),
                                first_name: participant.user.first_name().to_string(),
                                last_name: participant.user.last_name().map(|l| l.to_string()),
                                role: MemberRole::Administrator,
                            });
                            found_new_admins = true;
                        }
                    }
                    _ => {}
                }
            }

            if found_new_owner && found_new_admins {
                info!(
                    "Found owner + {} new admins on attempt {}",
                    administrators.len(),
                    attempt
                );
            } else if found_new_owner {
                info!("Found owner on attempt {}", attempt);
            } else if found_new_admins {
                info!(
                    "Found {} admins on attempt {}",
                    administrators.len(),
                    attempt
                );
            }

            if owner.is_some() && !administrators.is_empty() {
                info!(
                    "Both owner and admins collected, stopping at attempt {}",
                    attempt
                );
                break;
            }

            if attempt < 3 {
                warn!(
                    "Attempt {}: owner={}, admins={}. Retrying...",
                    attempt,
                    if owner.is_some() { "found" } else { "missing" },
                    administrators.len()
                );
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }

        let linked_channel_id = self.get_linked_channel_id(chat).await.ok();

        self.save_chat_admins_to_db(
            chat.id(),
            owner.as_ref(),
            &administrators,
            linked_channel_id,
            db_pool,
        )
        .await?;

        if owner.is_some() && !administrators.is_empty() {
            info!("Fetched and saved admins for chat {}: owner={}, admins count={}, linked_channel={:?}", 
          chat.id(), owner.as_ref().unwrap().user_id, administrators.len(), linked_channel_id);
        } else if owner.is_some() {
            warn!(
                "Fetched owner but no admins for chat {}: owner={}, linked_channel={:?}",
                chat.id(),
                owner.as_ref().unwrap().user_id,
                linked_channel_id
            );
        } else if !administrators.is_empty() {
            warn!(
                "No owner found but {} admins saved for chat {}, linked_channel={:?}",
                administrators.len(),
                chat.id(),
                linked_channel_id
            );
        } else {
            error!(
                "No owner or admins found for chat {}, only linked_channel={:?}",
                chat.id(),
                linked_channel_id
            );
        }

        Ok(())
    }

    async fn get_linked_channel_id(&self, chat: &Chat) -> Result<i64> {
        match chat {
            Chat::Group(group) => {
                info!("Processing group to find linked channel");
                use grammers_client::grammers_tl_types as tl;

                if let tl::enums::Chat::Channel(channel) = &group.raw {
                    let input_channel = tl::types::InputChannel {
                        channel_id: channel.id,
                        access_hash: channel.access_hash
                            .ok_or_else(|| anyhow::anyhow!("Channel access_hash is required but missing"))?,
                    };

                    let request = tl::functions::channels::GetFullChannel {
                        channel: input_channel.into(),
                    };

                    let result = self.client.invoke(&request).await?;

                    match result {
                        tl::enums::messages::ChatFull::Full(chat_full_data) => {
                            match &chat_full_data.full_chat {
                                tl::enums::ChatFull::ChannelFull(channel_full) => {
                                    channel_full.linked_chat_id.ok_or_else(|| {
                                        anyhow::anyhow!("Megagroup has no linked channel")
                                    })
                                }
                                tl::enums::ChatFull::Full(_) => {
                                    Err(anyhow::anyhow!("This is a regular group, not a channel"))
                                }
                            }
                        }
                    }
                } else {
                    info!("Regular groups don't have linked channels");
                    Err(anyhow::anyhow!("Regular groups don't have linked channels"))
                }
            },
            Chat::Channel(_) => {
                Err(anyhow::anyhow!("This method is for finding linked channels from groups, not discussion groups from channels"))
            },
            Chat::User(_) => {
                info!("Private chats don't have linked channels");
                Err(anyhow::anyhow!("Private chats don't have linked channels"))
            },
        }
    }

    async fn is_sender_admin(
        &self,
        sender_id: i64,
        chat_id: i64,
        db_pool: &SqlitePool,
    ) -> Result<bool> {
        let count = sqlx::query(
            "SELECT COUNT(*) as count FROM chat_admins 
         WHERE chat_id = ? AND user_id = ? AND role IN ('owner', 'admin', 'linked_channel')",
        )
        .bind(chat_id)
        .bind(sender_id)
        .fetch_one(db_pool)
        .await?;

        let admin_count: i64 = count.get("count");
        Ok(admin_count > 0)
    }

    async fn parse_report_response(&self, text: &str) -> Result<Option<(i64, String)>> {
        if !text.starts_with("report_response:") {
            return Ok(None);
        }

        let parts: Vec<&str> = text.splitn(3, ':').collect();
        if parts.len() >= 2 {
            if let Ok(bot_api_chat_id) = parts[1].parse::<i64>() {
                let response_details = if parts.len() >= 3 {
                    parts[2].to_string()
                } else {
                    "Unknown response".to_string()
                };

                let grammers_chat_id = if bot_api_chat_id < 0 {
                    -(bot_api_chat_id + 1000000000000)
                } else {
                    bot_api_chat_id
                };

                info!(
                    "Bot api chat_id parsed from {} to {}",
                    bot_api_chat_id, grammers_chat_id
                );

                return Ok(Some((grammers_chat_id, response_details)));
            }
        }

        Ok(None)
    }

    async fn save_chat_admins_to_db(
        &self,
        chat_id: i64,
        owner: Option<&ChatMember>,
        administrators: &[ChatMember],
        linked_channel_id: Option<i64>,
        db_pool: &SqlitePool,
    ) -> Result<()> {
        sqlx::query("DELETE FROM chat_admins WHERE chat_id = ?")
            .bind(chat_id)
            .execute(db_pool)
            .await?;

        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

        if let Some(owner) = owner {
            sqlx::query("INSERT INTO chat_admins (chat_id, user_id, role, fetched_at) VALUES (?, ?, 'owner', ?)")
                .bind(chat_id)
                .bind(owner.user_id)
                .bind(&timestamp)
                .execute(db_pool)
                .await?;
        }

        for admin in administrators {
            sqlx::query("INSERT INTO chat_admins (chat_id, user_id, role, fetched_at) VALUES (?, ?, 'admin', ?)")
                .bind(chat_id)
                .bind(admin.user_id)
                .bind(&timestamp)
                .execute(db_pool)
                .await?;
        }

        if let Some(linked_id) = linked_channel_id {
            sqlx::query("INSERT INTO chat_admins (chat_id, user_id, role, fetched_at) VALUES (?, ?, 'linked_channel', ?)")
                .bind(chat_id)
                .bind(linked_id)
                .bind(&timestamp)
                .execute(db_pool)
                .await?;
        }

        Ok(())
    }

    async fn process_report_response(
        &self,
        chat_id: i64,
        response: String,
        db_pool: &SqlitePool,
    ) -> Result<()> {
        sqlx::query("UPDATE chat_monitoring SET report_response = ? WHERE chat_id = ?")
            .bind(&response)
            .bind(chat_id)
            .execute(db_pool)
            .await?;

        if response.contains("Offer sent:") {
            info!(
                "Report successfully processed for chat {}: {}",
                chat_id, response
            );
        } else if response.contains("Error") {
            warn!(
                "Report processing failed for chat {}: {}",
                chat_id, response
            );
        } else {
            info!(
                "Report response recorded for chat {}: {}",
                chat_id, response
            );
        }

        Ok(())
    }

    async fn get_chat_status(&self, chat_id: i64, db_pool: &SqlitePool) -> Result<String> {
        let result = sqlx::query("SELECT status FROM chat_monitoring WHERE chat_id = ?")
            .bind(chat_id)
            .fetch_optional(db_pool)
            .await?;

        match result {
            Some(row) => {
                let status: String = row.get("status");
                Ok(status)
            }
            None => Ok("collecting".to_string()),
        }
    }

    async fn should_resume_monitoring(&self, chat_id: i64, db_pool: &SqlitePool) -> Result<bool> {
        let result = sqlx::query("SELECT last_report_sent FROM chat_monitoring WHERE chat_id = ?")
            .bind(chat_id)
            .fetch_optional(db_pool)
            .await?;

        match result {
            Some(row) => {
                if let Some(last_sent_str) = row.get::<Option<String>, _>("last_report_sent") {
                    if let Ok(last_sent_time) = DateTime::parse_from_rfc3339(&last_sent_str) {
                        let elapsed = Utc::now() - last_sent_time.with_timezone(&Utc);
                        Ok(elapsed.num_days() >= 11)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            None => Ok(false),
        }
    }

    async fn fetch_chat_message_stats(
        &self,
        chat: &Chat,
        app_state: &Arc<AgentAppState>,
        chat_title: &str,
    ) -> Result<()> {
        const MAX_RETRIES: u32 = 3;

        for attempt in 0..MAX_RETRIES {
            match self
                .fetch_chat_message_stats_internal(chat, app_state, &chat_title)
                .await
            {
                Ok(()) => return Ok(()),
                Err(e) if attempt < MAX_RETRIES - 1 => {
                    warn!(
                        "Attempt {}/{}: Failed to fetch stats for chat {}: {}. Retrying...",
                        attempt + 1,
                        MAX_RETRIES,
                        chat.id(),
                        e
                    );
                    tokio::time::sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                }
                Err(e) => {
                    error!(
                        "Failed to fetch stats for chat {} after {} attempts: {}",
                        chat.id(),
                        MAX_RETRIES,
                        e
                    );
                    return Err(e);
                }
            }
        }
        unreachable!()
    }

    async fn fetch_chat_message_stats_internal(
        &self,
        chat: &Chat,
        app_state: &Arc<AgentAppState>,
        chat_title: &str,
    ) -> Result<()> {
        let mut user_counts: HashMap<i64, u32> = HashMap::new();

        let mut msgs = self.client.iter_messages(chat.pack()).limit(1000);

        while let Some(msg) = msgs.next().await? {
            if let Some(sender) = msg.sender() {
                *user_counts.entry(sender.id()).or_insert(0) += 1;
            }
        }

        {
            let mut stats = app_state.chat_message_stats.lock().await;
            stats
                .chat_message_counts
                .insert(chat.id(), user_counts.clone());
        }

        info!(
            "Fetched message stats for chat {} [id: {}]: {} unique users",
            chat_title,
            chat.id(),
            user_counts.len()
        );
        Ok(())
    }

    // async fn fetch_chat_message_stats_internal(
    //     &self,
    //     chat: &Chat,
    //     app_state: &Arc<AgentAppState>,
    // ) -> Result<()> {
    //
    //     let mut user_counts: HashMap<i64, u32> = HashMap::new();
    //
    //     let batch_size = 1000;
    //     let total_messages = 6000;
    //     let mut processed = 0;
    //
    //     let mut msgs = self.client.iter_messages(chat.pack()).limit(total_messages);
    //
    //     while let Some(msg) = msgs.next().await? {
    //         if let Some(sender) = msg.sender() {
    //             *user_counts.entry(sender.id()).or_insert(0) += 1;
    //         }
    //
    //         processed += 1;
    //
    //         if processed % batch_size == 0 {
    //             let mut rng = rand::rng();
    //             let delay = rng.random_range(1500..3000);
    //             info!("Processed {} messages, pausing {}ms...", processed, delay);
    //             tokio::time::sleep(Duration::from_millis(delay)).await;
    //         }
    //     }
    //
    //     {
    //         let mut stats = app_state.chat_message_stats.lock().await;
    //         stats.chat_message_counts.insert(chat.id(), user_counts.clone());
    //     }
    //
    //     info!(
    //     "Fetched message stats for chat {}: {} unique users from {} messages",
    //     chat.id(), user_counts.len(), processed
    // );
    //     Ok(())
    // }

    // async fn fetch_chat_message_stats_internal(
    //     &self,
    //     chat: &Chat,
    //     app_state: &Arc<AgentAppState>,
    // ) -> Result<()> {
    //     use rand::Rng;
    //
    //     let mut user_counts: HashMap<i64, u32> = HashMap::new();
    //
    //     let batch_size = 1000;
    //     let total_messages = 5000;
    //     let mut processed = 0;
    //
    //     let mut msgs = self.client.iter_messages(chat.pack()).limit(total_messages);
    //
    //     while let Some(msg) = msgs.next().await? {
    //         if let Some(sender) = msg.sender() {
    //             *user_counts.entry(sender.id()).or_insert(0) += 1;
    //         }
    //
    //         processed += 1;
    //
    //         if processed % batch_size == 0 {
    //             if processed == 3000 {
    //                 info!("Processed {} messages - FLOOD_WAIT protection: pausing 7 seconds...", processed);
    //                 tokio::time::sleep(Duration::from_secs(10)).await;
    //             } else {
    //                 let mut rng = rand::rng();
    //                 let delay = rng.random_range(1500..3000);
    //                 info!("Processed {} messages, pausing {}ms...", processed, delay);
    //                 tokio::time::sleep(Duration::from_millis(delay)).await;
    //             }
    //         }
    //     }
    //
    //     {
    //         let mut stats = app_state.chat_message_stats.lock().await;
    //         stats.chat_message_counts.insert(chat.id(), user_counts.clone());
    //     }
    //
    //     info!(
    //     "Fetched message stats for chat {}: {} unique users from {} messages",
    //     chat.id(), user_counts.len(), processed
    // );
    //     Ok(())
    // }
}

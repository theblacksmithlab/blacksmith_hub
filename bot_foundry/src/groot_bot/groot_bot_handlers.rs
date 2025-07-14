use crate::groot_bot::chat_moderation::chat_moderation;
use crate::groot_bot::chat_moderation_utils::handle_groot_report;
use anyhow::Result;
use chrono::{DateTime, Utc};
use core::local_db::tg_bot::groot_bot::subscription_management::check_chat_payment;
use core::local_db::tg_bot::groot_bot::subscription_management::create_subscription;
use core::local_db::tg_bot::groot_bot::subscription_management::get_subscription_info;
use core::models::common::system_messages::{AppsSystemMessages, GrootBotMessages, AgentDavonMessages};
use core::models::tg_agent::agent_davon::MemberRole;
use core::models::tg_agent::agent_davon::ReportedChatInfo;
use core::models::tg_bot::groot_bot::groot_bot::GrootBotCommands;
use core::models::tg_bot::groot_bot::groot_bot::{EditType, ResourcesDialogState, ShowType};
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::tg_bot::groot_bot::groot_bot_utils::{
    auto_delete_message, get_chat_title, get_chat_username, get_username,
    is_message_from_linked_channel, load_super_admins, read_admins_from_csv
};
use core::utils::tg_bot::groot_bot::subscription_utils::{
    show_plan_selection, PaymentProcess, SubscriptionState,
};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Message, Request, Requester, Update};
use teloxide::types::{KeyboardButton, KeyboardMarkup, UpdateKind};
use teloxide::Bot;
use teloxide_core::payloads::SendDocumentSetters;
use teloxide_core::prelude::ChatId;
use tracing::{error, info};

pub async fn groot_bot_command_handler(
    bot: Bot,
    msg: Message,
    cmd: GrootBotCommands,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let app_name = &app_state.app_name;
    let super_admins = load_super_admins(app_name);
    let user_id = msg.clone().from.unwrap().id.0;
    let chat_title = get_chat_title(&msg);
    let chat_id = msg.chat.id;
    let chat_username = get_chat_username(&msg);
    let username = get_username(&msg);
    let mut is_admin = false;
    let mut is_from_linked_channel = false;

    // Checking subscription
    let is_paid_chat = if let Some(db_pool) = &app_state.db_pool {
        check_chat_payment(db_pool, msg.chat.id.0)
            .await
            .unwrap_or(false)
    } else {
        false
    };

    let is_paid_chat = is_paid_chat || msg.chat.id.0 == -1001576410541;

    // Getting public chat's administrators
    if !msg.chat.is_private() {
        match bot.get_chat_administrators(msg.chat.id).send().await {
            Ok(admins) => {
                is_admin = msg
                    .from
                    .as_ref()
                    .map(|user| admins.iter().any(|admin| admin.user.id == user.id))
                    .unwrap_or(false);
            }
            Err(err) => {
                error!(
                    "Error getting admins list from chat '{}' [{}] [id: {}]: {:?}",
                    chat_title, chat_username, chat_id, err
                );
            }
        }

        if let Ok(true) = is_message_from_linked_channel(&bot, &msg).await {
            is_from_linked_channel = true;
            info!("Command from linked channel detected");
        }
    }

    // Getting info about chat owner
    let is_chat_owner = if !msg.chat.is_private() {
        match bot.get_chat_administrators(msg.chat.id).await {
            Ok(admins) => admins.iter().any(|admin| {
                admin.status() == teloxide::types::ChatMemberStatus::Owner
                    && admin.user.id.0 == user_id
            }),
            Err(_) => false,
        }
    } else {
        false
    };

    // Setting-up LORD_ADMIN_ID
    let lord_admin_id = match env::var("LORD_ADMIN_ID") {
        Ok(val) => match val.parse::<u64>() {
            Ok(id) => id,
            Err(_) => {
                error!("Error: LORD_ADMIN_ID .env has incorrect format!");

                let bot_system_message = bot
                    .send_message(msg.chat.id, "Error getting LORD_ADMIN_ID.")
                    .await?;

                auto_delete_message(
                    bot.clone(),
                    bot_system_message.chat.id,
                    bot_system_message.id,
                    Duration::from_secs(120),
                )
                .await;

                return Ok(());
            }
        },
        Err(_) => {
            error!("Error: LORD_ADMIN_ID must be set in .env!");
            bot.send_message(msg.chat.id, "Error getting LORD_ADMIN_ID.")
                .await?;
            return Ok(());
        }
    };

    // Execution area check
    if cmd != GrootBotCommands::Start
        && cmd != GrootBotCommands::Subscription
        && cmd != GrootBotCommands::Groot
        && cmd != GrootBotCommands::Status
        && !msg.chat.is_private()
    {
        info!(
            "User {} [id: {}] tried to use {:?} command in public chat '{}' [{}] [{}]",
            username, user_id, cmd, chat_title, chat_username, chat_id
        );
        let bot_msg = get_message(AppsSystemMessages::GrootBot(
            GrootBotMessages::PrivateCmdUsedInPublicChat,
        ))
        .await?;

        let bot_system_message = bot.send_message(msg.chat.id, bot_msg).await?;

        auto_delete_message(
            bot.clone(),
            bot_system_message.chat.id,
            bot_system_message.id,
            Duration::from_secs(120),
        )
        .await;

        return Ok(());
    }

    if (cmd == GrootBotCommands::Start
        || cmd == GrootBotCommands::Groot
        || cmd == GrootBotCommands::Status
        || cmd == GrootBotCommands::Subscription)
        && msg.chat.is_private()
    {
        info!(
            "User {} [id: {}] tried to use /{:?} command in private chat",
            username, user_id, cmd
        );

        let bot_msg = match cmd {
            GrootBotCommands::Start => {
                get_message(AppsSystemMessages::GrootBot(
                    GrootBotMessages::StartCmdUsedInPrivateChat,
                ))
                .await?
            }
            GrootBotCommands::Groot => {
                get_message(AppsSystemMessages::GrootBot(
                    GrootBotMessages::PublicCmdUsedInPrivateChat,
                ))
                .await?
            }
            GrootBotCommands::Status => {
                get_message(AppsSystemMessages::GrootBot(
                    GrootBotMessages::PublicCmdUsedInPrivateChat,
                ))
                .await?
            }
            GrootBotCommands::Subscription => {
                get_message(AppsSystemMessages::GrootBot(
                    GrootBotMessages::SubscriptionCmdUsedInPrivateChat,
                ))
                .await?
            }
            _ => unreachable!(),
        };

        let bot_system_message = bot.send_message(msg.chat.id, bot_msg).await?;

        auto_delete_message(
            bot.clone(),
            bot_system_message.chat.id,
            bot_system_message.id,
            Duration::from_secs(120),
        )
        .await;

        return Ok(());
    }

    // Executor check
    if cmd == GrootBotCommands::Start
        && !is_admin
        && !is_from_linked_channel
        && user_id != lord_admin_id
    {
        info!(
            "User | {} | with id: {} tried to use /{:?} command in public chat",
            username, user_id, cmd
        );

        let bot_msg = get_message(AppsSystemMessages::GrootBot(
            GrootBotMessages::CommonStartCmdReaction,
        ))
        .await?;

        let bot_system_message = bot.send_message(msg.chat.id, bot_msg).await?;

        auto_delete_message(
            bot.clone(),
            bot_system_message.chat.id,
            bot_system_message.id,
            Duration::from_secs(120),
        )
        .await;

        return Ok(());
    }

    if cmd == GrootBotCommands::Start
        && !msg.chat.is_private()
        && (is_admin || is_from_linked_channel || user_id == lord_admin_id)
    {
        if msg.chat.username().is_none() {
            info!(
                "Admin | {} | with id: {} tried to use /{:?} command in chat without username",
                username, user_id, cmd
            );

            let bot_msg = get_message(AppsSystemMessages::GrootBot(
                GrootBotMessages::NoUsernameForChatAlert,
            ))
            .await?;

            let bot_system_message = bot.send_message(msg.chat.id, bot_msg).await?;

            auto_delete_message(
                bot.clone(),
                bot_system_message.chat.id,
                bot_system_message.id,
                Duration::from_secs(120),
            )
            .await;

            return Ok(());
        }
    }

    if cmd == GrootBotCommands::Resources && !super_admins.contains(&user_id) {
        info!(
            "Non-super-admin user | {} | with id: {} tried to use /{:?} command",
            username, user_id, cmd,
        );
        let bot_msg = get_message(AppsSystemMessages::GrootBot(
            GrootBotMessages::NoRightsForUseCmd,
        ))
        .await?;

        let bot_system_message = bot.send_message(msg.chat.id, bot_msg).await?;

        auto_delete_message(
            bot.clone(),
            bot_system_message.chat.id,
            bot_system_message.id,
            Duration::from_secs(120),
        )
        .await;

        return Ok(());
    }

    if (cmd == GrootBotCommands::Subscription || cmd == GrootBotCommands::Status)
        && !is_admin
        && !is_from_linked_channel
        && !is_chat_owner
        && user_id != lord_admin_id
    {
        info!(
            "Non-admin user | {} | with id: {} tried to use /{:?} command",
            username, user_id, cmd,
        );
        let bot_msg = get_message(AppsSystemMessages::GrootBot(
            GrootBotMessages::NoRightsForUseCmd,
        ))
        .await?;

        let bot_system_message = bot.send_message(msg.chat.id, bot_msg).await?;

        auto_delete_message(
            bot.clone(),
            bot_system_message.chat.id,
            bot_system_message.id,
            Duration::from_secs(120),
        )
        .await;

        return Ok(());
    }

    if cmd == GrootBotCommands::ForceSubscription && lord_admin_id == user_id {
        let parts: Vec<&str> = msg.text().unwrap().split_whitespace().collect();
        if parts.len() >= 6 {
            let chat_id: i64 = parts[1].parse()?;
            let chat_username = parts[2];
            let paid_by_user_id: i64 = parts[3].parse()?;
            let paid_by_username = if parts[4] != "null" {
                Some(parts[4])
            } else {
                None
            };
            let plan_type = parts[5];

            if let Some(db_pool) = &app_state.db_pool {
                create_subscription(
                    db_pool,
                    chat_id,
                    chat_username,
                    paid_by_user_id,
                    paid_by_username,
                    plan_type,
                )
                .await?;

                if let Some(chat_stats_mutex) = &app_state.chat_message_stats {
                    let mut chat_stats = chat_stats_mutex.lock().await;
                    let _ = chat_stats
                        .fetch_chat_history_for_new_chat(
                            &app_state.app_name,
                            ChatId(chat_id),
                            chat_username,
                        )
                        .await;
                }

                bot.send_message(msg.chat.id,
                                 format!("✅ Подписка активирована вручную для чата:\n• Чат: {}\n• ID: {}\n• Плательщик: {} ({})\n• Тарифный план: {}",
                                         chat_username, chat_id, paid_by_user_id,
                                         paid_by_username.unwrap_or("null"), plan_type))
                    .await?;
            }
        } else {
            bot.send_message(
                msg.chat.id,
                "❌ Формат: /forcesubscription CHAT_ID USERNAME USER_ID USERNAME_OR_NULL PLAN_TYPE",
            )
            .await?;
        }
    }

    match cmd {
        GrootBotCommands::Start => {
            let bot_msg = get_message(AppsSystemMessages::GrootBot(
                GrootBotMessages::StartCmdUsedInPublicChat,
            ))
            .await?;
            bot.send_message(msg.chat.id, bot_msg).await?;

            if !is_paid_chat {
                let demo_msg = get_message(AppsSystemMessages::GrootBot(
                    GrootBotMessages::DemoModeMessage,
                ))
                .await?;
                bot.send_message(msg.chat.id, demo_msg).await?;
            } else {
                let chat_username = msg.chat.username().unwrap();

                info!(
                    "Chat: {} with id: {} is paid. Fetching chat history...",
                    chat_username, msg.chat.id
                );

                let mut chat_stats = app_state.chat_message_stats.as_ref().unwrap().lock().await;
                if let Err(err) = chat_stats
                    .fetch_chat_history_for_new_chat(
                        &app_state.app_name,
                        msg.chat.id,
                        chat_username,
                    )
                    .await
                {
                    error!(
                        "Error fetching chat history for a new chat: {} with id: {}: {}",
                        chat_username, msg.chat.id, err
                    );
                }
            }
        }
        GrootBotCommands::ForceSubscription => {
            info!("Force subscription cmd executed!");
        }
        GrootBotCommands::About => {
            let bot_msg =
                get_message(AppsSystemMessages::GrootBot(GrootBotMessages::About)).await?;
            bot.send_message(msg.chat.id, bot_msg).await?;
        }
        GrootBotCommands::Resources => {
            if let Some(dialog_states_mutex) = &app_state.dialog_states {
                let mut dialog_states = dialog_states_mutex.lock().await;

                let state = dialog_states
                    .entry(user_id)
                    .or_insert(ResourcesDialogState {
                        awaiting_option_choice: false,
                        awaiting_edit_type: false,
                        awaiting_show_type: false,
                        edit_type: EditType::None,
                        show_type: ShowType::None,
                        awaiting_data_entry: false,
                        awaiting_ask_message: false,
                    });

                state.awaiting_option_choice = true;

                let keyboard = KeyboardMarkup::new(vec![
                    vec![KeyboardButton::new("ПОКАЗАТЬ resources")],
                    vec![KeyboardButton::new("ДОБАВИТЬ resources")],
                    vec![KeyboardButton::new("Cancel")],
                ]);

                bot.send_message(msg.chat.id, "Choose an option:")
                    .reply_markup(keyboard)
                    .await?;
            }
        }
        GrootBotCommands::Manual => {
            let bot_msg = get_message(AppsSystemMessages::GrootBot(
                GrootBotMessages::ManualMessage,
            ))
            .await?;
            bot.send_message(msg.chat.id, bot_msg).await?;
        }
        GrootBotCommands::Results => {
            let bot_msg = get_message(AppsSystemMessages::GrootBot(
                GrootBotMessages::ResultsTempMessage,
            ))
            .await?;

            let bot_system_message = bot.send_message(msg.chat.id, bot_msg).await?;

            auto_delete_message(
                bot.clone(),
                bot_system_message.chat.id,
                bot_system_message.id,
                Duration::from_secs(120),
            )
            .await;
        }
        GrootBotCommands::Groot => {
            if let Some(replied_msg) = msg.reply_to_message() {
                let is_lord_admin = user_id == lord_admin_id;

                if is_lord_admin {
                    info!(
                "LORD_ADMIN with id: {} reported message in chat {}. Silent immediate deletion.",
                user_id, msg.chat.username().unwrap_or_default()
            );

                    if let Err(e) = bot.delete_message(msg.chat.id, replied_msg.id).await {
                        error!("Error deleting message by LORD_ADMIN request: {:?}", e);
                    }

                    if let Err(e) = bot.delete_message(msg.chat.id, msg.id).await {
                        error!("Error deleting LORD_ADMIN command message: {:?}", e);
                    }

                    return Ok(());
                }

                let reported_user_id = match replied_msg.clone().from {
                    Some(user) => user.id.0 as i64,
                    None => {
                        let bot_msg = get_message(AppsSystemMessages::GrootBot(
                            GrootBotMessages::NoUserIdWarn,
                        ))
                        .await?;

                        bot.send_message(msg.chat.id, bot_msg).await?;
                        return Ok(());
                    }
                };

                handle_groot_report(
                    &bot,
                    &app_state,
                    &msg,
                    &replied_msg,
                    user_id as i64,
                    &username,
                    reported_user_id,
                )
                .await?;
            } else {
                let bot_msg = get_message(AppsSystemMessages::GrootBot(
                    GrootBotMessages::NotCorrectReportUsage,
                ))
                .await?;

                let bot_system_message = bot.send_message(msg.chat.id, bot_msg).await?;

                auto_delete_message(
                    bot.clone(),
                    bot_system_message.chat.id,
                    bot_system_message.id,
                    Duration::from_secs(120),
                )
                .await;
            }
        }
        GrootBotCommands::Subscription => {
            info!(
                "User {} [id: {}] tried to use /{:?} command in public chat '{}' [{}] [id: {}]",
                username, user_id, cmd, chat_title, chat_username, chat_id
            );

            handle_subscription_command(bot.clone(), msg.clone(), app_state.clone()).await?;
        }
        GrootBotCommands::Status => {
            info!(
                "User {} [id: {}] tried to use /{:?} command in public chat '{}' [{}] [id: {}]",
                username, user_id, cmd, chat_title, chat_username, chat_id
            );

            handle_status_command(bot.clone(), msg.clone(), app_state.clone()).await?;
        }
        GrootBotCommands::DavonReport => {
            let message_text = msg.text().unwrap_or("");
            let parts: Vec<&str> = message_text.split_whitespace().collect();

            if parts.len() >= 2 {
                match parts[1].parse::<i64>() {
                    Ok(reported_chat_id) => {
                        let report_response_prefix =
                            format!("report_response:{}", reported_chat_id);
                        info!("Processing new agent report for chat: {}... Reading admin data from CSV", reported_chat_id);

                        let admins_csv_path = format!("common_res/agent_davon/reports/{}_admins.csv", reported_chat_id);

                        let (chat_title, chat_username, admins) = match read_admins_from_csv(&admins_csv_path).await {
                            Ok(data) => data,
                            Err(e) => {
                                error!("Failed to read admin data from CSV: {}", e);
                                bot.send_message(
                                    msg.chat.id,
                                    format!("{}:Error reading admin data: {}", report_response_prefix, e),
                                ).await?;
                                return Ok(());
                            }
                        };

                        info!("Admin data loaded from CSV: chat '{}' (@{}) [id: {}] with {} admins", chat_title, chat_username, chat_id, admins.len());

                        let template = get_message(AppsSystemMessages::AgentDavon(AgentDavonMessages::Offer)).await?;

                        let offer = template
                            .replace("{chat_title}", &chat_title)
                            .replace("{chat_username}", &chat_username)
                            .replace("{chat_id}", &reported_chat_id.to_string());

                        let mut sent_count = 0;

                        for admin in &admins {
                            match bot.send_message(ChatId(lord_admin_id as i64), &offer).await {
                                // match bot.send_message(ChatId(admin.user_id), &offer).await {
                                Ok(_) => {
                                    info!("Message sent to {} {} [id: {}]", admin.role, admin.user_id, admin.user_id);
                                    sent_count += 1;
                                }
                                Err(e) => {
                                    error!("Failed to send to {} {}: {}", admin.role, admin.user_id, e);
                                }
                            }
                        }

                        if sent_count > 0 {
                            info!("Messages sent to {}/{} admins", sent_count, admins.len());

                            let csv_path = format!(
                                "common_res/agent_davon/reports/{}_report.csv",
                                reported_chat_id
                            );
                            let mut file_sent_count = 0;

                            for admin in admins {
                                let document = teloxide::types::InputFile::file(&csv_path);

                                match bot
                                    // .send_document(ChatId(admin.user_id), document)
                                    .send_document(ChatId(lord_admin_id as i64), document)
                                    .caption("Отчет о спам-сообщениях:")
                                    .await
                                {
                                    Ok(_) => {
                                        info!("CSV sent to {} {} [id: {}]", admin.role, admin.user_id, admin.user_id);
                                        file_sent_count += 1;
                                    }
                                    Err(e) => {
                                        error!("Failed to send CSV to {} {}: {}", admin.role, admin.user_id, e);
                                    }
                                }
                            }

                            bot.send_message(
                                msg.chat.id,
                                format!(
                                    "{}:Offer sent: {} participants, {} CSV files for chat: {}",
                                    report_response_prefix, sent_count, file_sent_count, chat_title
                                ),
                            ).await?;
                        } else {
                            error!(
                                "Failed to send messages to any admin for chat {}",
                                reported_chat_id
                            );
                            bot.send_message(
                                msg.chat.id,
                                format!("{}:Error sending offers to chat administration (probably personal messaging is turned-off)", report_response_prefix)
                            ).await?;
                        }
                    }
                    Err(e) => {
                        error!(
                            "Invalid chat_id in agent report: {}. Error: {}",
                            parts[1], e
                        );
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "report_response:{}:Got invalid chat_id: {}",
                                parts[1], parts[1]
                            ),
                        )
                        .await?;
                    }
                }
            } else {
                error!("Agent report command missing chat_id argument");
                bot.send_message(
                    msg.chat.id,
                    "report_response:unknown:Got invalid cmd format",
                )
                .await?;
            }
        }
        GrootBotCommands::ChatAdminsRequest => {
            let message_text = msg.text().unwrap_or("");
            let parts: Vec<&str> = message_text.split_whitespace().collect();

            if parts.len() >= 2 {
                match parts[1].parse::<i64>() {
                    Ok(requested_chat_id) => {
                        info!(
                            "Processing chat admins request for chat: {}",
                            requested_chat_id
                        );

                        let chat_info = match ReportedChatInfo::new(&bot, requested_chat_id).await {
                            Ok(info) => info,
                            Err(e) => {
                                error!("Failed to get chat info: {}", e);
                                bot.send_message(
                                    msg.chat.id,
                                    format!(
                                        "chat_admins_response:{}:error:Failed to get chat info",
                                        requested_chat_id
                                    ),
                                )
                                .await?;
                                return Ok(());
                            }
                        };

                        let admin_ids: Vec<i64> = chat_info
                            .administrators
                            .iter()
                            .filter(|admin| admin.role == MemberRole::Administrator)
                            .map(|admin| admin.user_id)
                            .collect();

                        let response_json = serde_json::json!({
                            "owner": chat_info.owner.user_id,
                            "admins": admin_ids,
                            "linked": chat_info.linked_channel_id
                        });

                        let response = format!(
                            "chat_admins_response:{}:{}",
                            requested_chat_id, response_json
                        );

                        bot.send_message(msg.chat.id, response).await?;
                        info!("Sent admins list for chat {}", requested_chat_id);
                    }
                    Err(_e) => {
                        error!("Invalid chat_id in admins request: {}", parts[1]);
                        bot.send_message(
                            msg.chat.id,
                            "chat_admins_response:invalid:error:Invalid chat_id",
                        )
                        .await?;
                    }
                }
            } else {
                error!("Chat admins request missing chat_id argument");
                bot.send_message(
                    msg.chat.id,
                    "chat_admins_response:missing:error:Missing chat_id",
                )
                .await?;
            }
        }
    }

    Ok(())
}

pub async fn groot_bot_message_handler(
    bot: Bot,
    update: Update,
    bot_app_state: Arc<BotAppState>,
) -> Result<()> {
    let msg = match update {
        Update {
            kind: UpdateKind::Message(message),
            ..
        } => message,
        Update {
            kind: UpdateKind::EditedMessage(message),
            ..
        } => message,
        _ => return Ok(()),
    };

    // Checking subscription
    let is_paid_chat = if let Some(db_pool) = &bot_app_state.db_pool {
        check_chat_payment(db_pool, msg.chat.id.0)
            .await
            .unwrap_or(false)
    } else {
        false
    };

    let is_paid_chat = is_paid_chat || msg.chat.id.0 == -1001576410541;

    chat_moderation(bot, msg, bot_app_state, is_paid_chat).await?;

    Ok(())
}

pub async fn handle_subscription_command(
    bot: Bot,
    msg: Message,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let target_chat_id = msg.chat.id.0;
    let target_chat_title = get_chat_title(&msg);
    let target_chat_username = match msg.chat.username() {
        Some(username) => username.to_string(),
        None => {
            let bot_system_message = bot.send_message(msg.chat.id, "❌ Чат должен иметь username (быть публичным), чтобы я мог защищать его от нечисти.\n\
            Установите его в настройках и попробуйте вызвать команду /subscription снова")
                .await?;

            auto_delete_message(
                bot.clone(),
                bot_system_message.chat.id,
                bot_system_message.id,
                Duration::from_secs(60),
            )
            .await;

            return Ok(());
        }
    };

    if let Some(db_pool) = &app_state.db_pool {
        if check_chat_payment(db_pool, target_chat_id)
            .await
            .unwrap_or(false)
        {
            let bot_system_message = bot
                .send_message(
                    msg.chat.id,
                    &format!(
                        "ℹ️ Чат '{}' уже имеет активную подписку!",
                        target_chat_username
                    ),
                )
                .await?;

            auto_delete_message(
                bot.clone(),
                bot_system_message.chat.id,
                bot_system_message.id,
                Duration::from_secs(60),
            )
            .await;

            return Ok(());
        }
    }

    let is_from_linked_channel = is_message_from_linked_channel(&bot, &msg)
        .await
        .unwrap_or(false);

    if is_from_linked_channel {
        info!("Subscription command from linked channel - finding chat owner");

        let chat_owner = match bot.get_chat_administrators(msg.chat.id).await {
            Ok(admins) => admins
                .into_iter()
                .find(|admin| admin.status() == teloxide::types::ChatMemberStatus::Owner)
                .map(|owner| owner.user.clone()),
            Err(e) => {
                error!("Failed to get chat administrators: {}", e);
                bot.send_message(
                    msg.chat.id,
                    "❌ Не удалось получить информацию об администраторах чата.",
                )
                .await?;
                return Ok(());
            }
        };

        let owner = match chat_owner {
            Some(owner) => owner,
            None => {
                bot.send_message(msg.chat.id, "❌ Не удалось найти владельца чата")
                    .await?;
                return Ok(());
            }
        };

        let _owner_username = match &owner.username {
            Some(username) => username.clone(),
            None => {
                bot.send_message(msg.chat.id, "❌ У владельца чата нет username.\n\
                Данные владельца чата необходимы, чтобы я мог связаться с ним для оформления подписки.")
                    .await?;
                return Ok(());
            }
        };

        if let Some(payment_states_mutex) = &app_state.payment_states {
            let mut payment_states = payment_states_mutex.lock().await;

            payment_states.insert(
                owner.id.0,
                PaymentProcess {
                    state: SubscriptionState::AwaitingPlanSelection,
                    target_chat_id: Some(target_chat_id),
                    target_chat_username: Some(target_chat_username.clone()),
                    target_chat_title: Some(target_chat_title.clone()),
                    selected_plan: None,
                    payment_amount: None,
                    payment_id: None,
                    heleket_invoice_uuid: None,
                    heleket_order_id: None,
                    original_price: None,
                    discount_percent: None,
                    final_price: None,
                    discount_reason: None,
                },
            );
        }

        let group_msg = "✅ Я отправил инструкцию по оформлению подписки владельцу чата в ЛС.";
        let bot_system_message = bot.send_message(msg.chat.id, group_msg).await?;

        auto_delete_message(
            bot.clone(),
            bot_system_message.chat.id,
            bot_system_message.id,
            Duration::from_secs(60),
        )
        .await;

        show_plan_selection(
            bot,
            ChatId(owner.id.0 as i64),
            &target_chat_username,
            &target_chat_title,
            owner.id.0 as i64,
            target_chat_id,
            app_state.clone(),
        )
        .await
    } else {
        info!("Subscription command from regular admin");

        let user_id = msg.from.as_ref().unwrap().id.0;
        let user_username = match &msg.from.as_ref().unwrap().username {
            Some(username) => username.clone(),
            None => {
                let bot_system_message = bot.send_message(msg.chat.id,
                                                          "❌ У вас нет username. Установите его в настройках Telegram, чтобы я мог написать вам в ЛС для оформления подписки.")
                    .await?;

                auto_delete_message(
                    bot.clone(),
                    bot_system_message.chat.id,
                    bot_system_message.id,
                    Duration::from_secs(60),
                )
                .await;

                return Ok(());
            }
        };

        if let Some(payment_states_mutex) = &app_state.payment_states {
            let mut payment_states = payment_states_mutex.lock().await;

            payment_states.insert(
                user_id,
                PaymentProcess {
                    state: SubscriptionState::AwaitingPlanSelection,
                    target_chat_id: Some(target_chat_id),
                    target_chat_username: Some(target_chat_username.clone()),
                    target_chat_title: Some(target_chat_title.clone()),
                    selected_plan: None,
                    payment_amount: None,
                    payment_id: None,
                    heleket_invoice_uuid: None,
                    heleket_order_id: None,
                    original_price: None,
                    discount_percent: None,
                    final_price: None,
                    discount_reason: None,
                },
            );
        }

        let group_msg = format!("✅ @{}, проверьте ЛС для оплаты подписки", user_username);
        let bot_system_message = bot.send_message(msg.chat.id, group_msg).await?;

        auto_delete_message(
            bot.clone(),
            bot_system_message.chat.id,
            bot_system_message.id,
            Duration::from_secs(30),
        )
        .await;

        show_plan_selection(
            bot,
            ChatId(user_id as i64),
            &target_chat_username,
            &target_chat_title,
            user_id as i64,
            target_chat_id,
            app_state.clone(),
        )
        .await
    }
}

pub async fn handle_status_command(
    bot: Bot,
    msg: Message,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let chat_id = msg.chat.id.0;

    if msg.chat.is_private() {
        bot.send_message(msg.chat.id, "❌ Эта команда работает только в группах")
            .await?;
        return Ok(());
    }

    let chat_username = match msg.chat.username() {
        Some(username) => username,
        None => {
            bot.send_message(
                msg.chat.id,
                "❌ Чат должен иметь username для проверки статуса подписки",
            )
            .await?;
            return Ok(());
        }
    };

    let chat_title = get_chat_title(&msg);

    if let Some(db_pool) = &app_state.db_pool {
        match get_subscription_info(db_pool, chat_id).await {
            Ok(Some(subscription)) => {
                let end_date = DateTime::parse_from_rfc3339(&subscription.end_date)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&chrono::FixedOffset::east_opt(3 * 3600).unwrap())
                    .format("%d.%m.%Y %H:%M UTC+3");

                let start_date = DateTime::parse_from_rfc3339(&subscription.start_date)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&chrono::FixedOffset::east_opt(3 * 3600).unwrap())
                    .format("%d.%m.%Y %H:%M UTC+3");

                let plan_name = match subscription.plan_type.as_str() {
                    "monthly" => "Месячная подписка",
                    "yearly" => "Годовая подписка",
                    _ => "Неизвестный план",
                };

                let now = Utc::now();
                let end_date_utc = DateTime::parse_from_rfc3339(&subscription.end_date)
                    .unwrap_or_else(|_| now.into())
                    .with_timezone(&Utc);

                let is_active = end_date_utc > now;
                let status_emoji = if is_active { "✅" } else { "❌" };
                let status_text = if is_active {
                    "Активна"
                } else {
                    "Истекла"
                };

                let days_left = if is_active {
                    let duration = end_date_utc.signed_duration_since(now);
                    duration.num_days()
                } else {
                    0
                };

                let days_info = if is_active && days_left > 0 {
                    format!("\n🗓️ **Осталось дней:** {}", days_left)
                } else if is_active {
                    "\n🗓️ **Истекает сегодня**".to_string()
                } else {
                    "".to_string()
                };

                let status_msg = format!(
                    "{} Статус подписки\n\n\
                    🏠 Чат: {} (@{})\n\
                    📊 Статус: {}\n\
                    📋 Тарифный план: {}\n\
                    📅 Начало: {}\n\
                    ⏰ Окончание: {}{}\n\
                    🛡️ Защита от спама: {}",
                    status_emoji,
                    chat_title,
                    chat_username,
                    status_text,
                    plan_name,
                    start_date,
                    end_date,
                    days_info,
                    if is_active {
                        "Включена"
                    } else {
                        "Отключена"
                    }
                );

                let bot_system_message = bot.send_message(msg.chat.id, status_msg).await?;

                auto_delete_message(
                    bot.clone(),
                    bot_system_message.chat.id,
                    bot_system_message.id,
                    Duration::from_secs(120),
                )
                .await;
            }
            Ok(None) => {
                let no_subscription_msg = format!(
                    "❌ Подписка не найдена\n\n\
                    🏠 Чат: @{}\n\
                    📊 Статус: Не активна\n\n\
                    💡 Для активации подписки используйте: /subscription",
                    chat_username
                );

                let bot_system_message = bot.send_message(msg.chat.id, no_subscription_msg).await?;
                auto_delete_message(
                    bot.clone(),
                    bot_system_message.chat.id,
                    bot_system_message.id,
                    Duration::from_secs(120),
                )
                .await;
            }
            Err(e) => {
                error!(
                    "Error checking subscription status for chat {}: {}",
                    chat_id, e
                );
                let bot_system_message = bot
                    .send_message(msg.chat.id, "❌ Ошибка при проверке статуса подписки")
                    .await?;
                auto_delete_message(
                    bot.clone(),
                    bot_system_message.chat.id,
                    bot_system_message.id,
                    Duration::from_secs(120),
                )
                .await;
            }
        }
    } else {
        let bot_system_message = bot
            .send_message(msg.chat.id, "❌ База данных недоступна")
            .await?;

        auto_delete_message(
            bot.clone(),
            bot_system_message.chat.id,
            bot_system_message.id,
            Duration::from_secs(120),
        )
        .await;
    }

    Ok(())
}

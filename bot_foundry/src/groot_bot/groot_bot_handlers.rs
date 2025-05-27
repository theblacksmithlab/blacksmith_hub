use crate::groot_bot::chat_moderation::chat_moderation;
use crate::groot_bot::chat_moderation_utils::handle_groot_report;
use crate::groot_bot::groot_bot_utils::{auto_delete_message, load_super_admins};
use anyhow::Result;
use core::models::common::system_messages::{AppsSystemMessages, GrootBotMessages};
use core::models::tg_bot::groot_bot::groot_bot::GrootBotCommands;
use core::models::tg_bot::groot_bot::groot_bot::{EditType, ResourcesDialogState, ShowType};
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::tg_bot::groot_bot::build_resource_file_path;
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Message, Request, Requester, Update};
use teloxide::types::{InputFile, KeyboardButton, KeyboardMarkup, UpdateKind};
use teloxide::Bot;
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
    let username = msg
        .clone()
        .from
        .unwrap()
        .username
        .unwrap_or("Anonymous User".to_string());
    let mut is_admin = false;

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
                error!("Error getting admins list from public chat: {:?}", err);
            }
        }
    }

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

    if cmd != GrootBotCommands::Start && cmd != GrootBotCommands::Groot && !msg.chat.is_private() {
        info!(
            "User | {} | with id: {} tried to use {:?} command in public chat {}",
            username,
            user_id,
            cmd,
            msg.chat.username().unwrap_or_default()
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

    if (cmd == GrootBotCommands::Resources || cmd == GrootBotCommands::Logs)
        && !super_admins.contains(&user_id)
    {
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

    if cmd == GrootBotCommands::Ask {
        if !super_admins.contains(&user_id) {
            info!(
                "Non-super-admin | {} | with id: {} tried to use /{:?} command",
                username, user_id, cmd
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

        // TODO: implement an interactive ai-system of usage instructions
    }

    if (cmd == GrootBotCommands::Start || cmd == GrootBotCommands::Groot) && msg.chat.is_private() {
        info!(
            "User | {} | with id: {} tried to use /{:?} command in private chat",
            username, user_id, cmd
        );

        let bot_msg = get_message(AppsSystemMessages::GrootBot(
            GrootBotMessages::PublicCmdUsedInPrivateChat,
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

    if cmd == GrootBotCommands::Start && !is_admin && user_id != lord_admin_id {
        info!(
            "User | {} | with id: {} tried to use /{:?} command in public chat: ",
            username, user_id, cmd
        );

        let bot_msg = get_message(AppsSystemMessages::GrootBot(
            GrootBotMessages::StartCmdReaction,
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

    match cmd {
        GrootBotCommands::Start => {
            let bot_msg =
                get_message(AppsSystemMessages::GrootBot(GrootBotMessages::StartMessage)).await?;
            bot.send_message(msg.chat.id, bot_msg).await?;

            if let Some(chat_username) = msg.chat.username() {
                info!(
                    "Chat: {} with id: {} has username set. Fetching chat history...",
                    chat_username, msg.chat.id
                );

                let mut chat_stats = app_state.chat_message_stats.as_ref().unwrap().lock().await;
                if let Err(err) = chat_stats
                    .fetch_chat_history_for_new_chat(
                        &app_state.app_name,
                        msg.clone(),
                        chat_username,
                    )
                    .await
                {
                    error!(
                        "Error fetching chat history for a new chat: {} with id: {}: {}",
                        chat_username, msg.chat.id, err
                    );
                }
            } else {
                let bot_msg = get_message(AppsSystemMessages::GrootBot(
                    GrootBotMessages::NoUsernameForChatAlert,
                ))
                .await?;
                bot.send_message(msg.chat.id, bot_msg).await?;
                return Ok(());
            }
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
        GrootBotCommands::Ask => {
            let bot_msg = get_message(AppsSystemMessages::GrootBot(GrootBotMessages::Ask)).await?;
            bot.send_message(msg.chat.id, bot_msg).await?;
        }
        GrootBotCommands::Backup => {
            if user_id != lord_admin_id {
                let bot_msg = get_message(AppsSystemMessages::GrootBot(
                    GrootBotMessages::NoRightsForUseCmd,
                ))
                .await?;

                bot.send_message(msg.chat.id, bot_msg).await?;
            } else {
                let files_to_send = [
                    (
                        build_resource_file_path(app_name, "white_listed_users.json"),
                        "white_listed_users.json",
                    ),
                    (
                        build_resource_file_path(app_name, "black_listed_users.json"),
                        "black_listed_users.json",
                    ),
                    (
                        build_resource_file_path(app_name, "message_counts.json"),
                        "message_counts.json",
                    ),
                    (
                        build_resource_file_path(app_name, "restricted_words.json"),
                        "restricted_words.json",
                    ),
                    (
                        build_resource_file_path(app_name, "chats_list.json"),
                        "chats_list.json",
                    ),
                    (
                        build_resource_file_path(app_name, "penalty_points.json"),
                        "penalty_points.json",
                    ),
                ];

                for (file_path, file_name) in files_to_send.iter() {
                    let path = Path::new(file_path);
                    if path.exists() {
                        info!("Backup requested by LORD_ADMIN");
                        bot.send_document(
                            msg.chat.id,
                            InputFile::file(path).file_name(file_name.to_string()),
                        )
                        .await?;
                    } else {
                        bot.send_message(msg.chat.id, format!("File {} not found", file_name))
                            .await?;
                    }
                }
            }
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

                bot.send_message(msg.chat.id, bot_msg).await?;
            }
        }
        _ => {
            bot.send_message(msg.chat.id, "Invalid cmd").await?;
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

    chat_moderation(bot, msg, bot_app_state).await?;

    Ok(())
}

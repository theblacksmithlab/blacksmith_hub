use crate::groot_bot::chat_moderation::chat_moderation;
use crate::groot_bot::chat_moderation_utils::handle_groot_report;
use anyhow::Result;
use core::models::common::system_messages::{AppsSystemMessages, GrootBotMessages};
use core::models::tg_bot::groot_bot::groot_bot::GrootBotCommands;
use core::models::tg_bot::groot_bot::groot_bot::{EditType, ResourcesDialogState, ShowType};
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::tg_bot::groot_bot::groot_bot_utils::{
    auto_delete_message, is_message_from_linked_channel, load_super_admins,
};
use core::utils::tg_bot::groot_bot::subscription_payment::handle_subscription_command;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Message, Request, Requester, Update};
use teloxide::types::{KeyboardButton, KeyboardMarkup, UpdateKind};
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
    let mut is_from_linked_channel = false;

    // // Checking subscription
    // let is_paid_chat = if let Some(db_pool) = &app_state.db_pool {
    //     check_chat_payment(db_pool, msg.chat.id.0).await.unwrap_or(false)
    // } else {
    //     false
    // };

    let is_paid_chat = true;

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
                error!("Error getting admins list from public chat: {:?}", err);
            }
        }

        if let Ok(true) = is_message_from_linked_channel(&bot, &msg).await {
            is_from_linked_channel = true;
            info!("Command from linked channel detected");
        }
    }

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

    if (cmd == GrootBotCommands::Start || cmd == GrootBotCommands::Groot) && msg.chat.is_private() {
        info!(
            "User | {} | with id: {} tried to use /{:?} command in private chat",
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
            "User | {} | with id: {} tried to use /{:?} command",
            username, user_id, cmd,
            );

            handle_subscription_command(bot.clone(), msg.clone(), app_state.clone()).await?;
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

    let is_paid_chat = true;

    // let is_paid_chat = if let Some(db_pool) = &bot_app_state.db_pool {
    //     check_chat_payment(db_pool, msg.chat.id.0).await.unwrap_or(false)
    // } else {
    //     false
    // };

    chat_moderation(bot, msg, bot_app_state, is_paid_chat).await?;

    Ok(())
}

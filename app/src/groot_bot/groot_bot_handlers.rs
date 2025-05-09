use crate::groot_bot::chat_moderation::chat_moderation;
use crate::groot_bot::groot_bot_utils::{load_super_admins, load_white_listed_users};
use anyhow::Result;
use core::models::common::system_messages::{AppsSystemMessages, GrootBotMessages};
use core::models::tg_bot::groot_bot::groot_bot::{EditType, ResourcesDialogState, ShowType};
use core::models::tg_bot::groot_bot::groot_bot::GrootBotCommands;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::tg_bot::groot_bot::build_resource_file_path;
use std::env;
use std::path::Path;
use std::sync::Arc;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Message, Request, Requester, Update};
use teloxide::types::{InputFile, KeyboardButton, KeyboardMarkup, UpdateKind};
use teloxide::Bot;
use tracing::{error, info};
use crate::groot_bot::chat_moderation_utils::ai_check;


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
                bot.send_message(msg.chat.id, "Error getting LORD_ADMIN_ID.")
                    .await?;
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
        bot.send_message(msg.chat.id, bot_msg).await?;
        return Ok(());
    }

    if (cmd == GrootBotCommands::Resources || cmd == GrootBotCommands::Logs) && !super_admins.contains(&user_id) {
        info!(
        "Non-super-admin user | {} | with id: {} tried to use /{:?} command",
        username, user_id, cmd,
    );
        let bot_msg = get_message(AppsSystemMessages::GrootBot(
            GrootBotMessages::NoRightsForUseCmd,
        ))
            .await?;
        bot.send_message(msg.chat.id, bot_msg).await?;
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
            bot.send_message(msg.chat.id, bot_msg).await?;
            return Ok(());
        }

        // TODO: implement an interactive ai-system of usage instructions
    }

    if cmd == GrootBotCommands::Start && msg.chat.is_private() {
        info!(
            "User | {} | with id: {} tried to use /{:?} command in private chat",
            username, user_id, cmd
        );

        let bot_msg = get_message(AppsSystemMessages::GrootBot(
            GrootBotMessages::StartCmdInPrivateChat,
        ))
        .await?;
        bot.send_message(msg.chat.id, bot_msg).await?;
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
        bot.send_message(msg.chat.id, bot_msg).await?;
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
                bot.send_message(
                    msg.chat.id,
                    "Извините, у вас нет прав для использования этой команды. 🤷",
                )
                .await?;
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
                        bot.send_message(msg.chat.id, format!("Файл {} не найден", file_name))
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
            bot.send_message(msg.chat.id, bot_msg).await?;
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
                        bot.send_message(
                            msg.chat.id,
                            "Невозможно обработать жалобу на это сообщение - отсутствует информация об отправителе."
                        ).await?;
                        return Ok(());
                    }
                };

                let reported_username = replied_msg
                    .clone()
                    .from
                    .and_then(|user| user.username.clone())
                    .unwrap_or_else(|| "Unknown User".to_string());
                
                let is_reported_user_admin = if !msg.chat.is_private() {
                    match bot.get_chat_administrators(msg.chat.id).send().await {
                        Ok(admins) => {
                            admins.iter().any(|admin| admin.user.id.0 as i64 == reported_user_id)
                        }
                        Err(err) => {
                            error!("Error getting admins list for reported user check: {:?}", err);
                            false
                        }
                    }
                } else {
                    false
                };
                
                let white_listed_users = load_white_listed_users(&app_name);
                
                if white_listed_users.contains(&reported_user_id) {
                    info!(
                        "Ignoring report on white-listed user {} (ID: {})",
                        reported_username, reported_user_id
                    );
                    
                    bot.send_message(
                        msg.chat.id,
                        "Я не буду проверять сообщения от доверенных пользователей."
                    ).await?;
                    
                    return Ok(());
                }
                
                if is_reported_user_admin {
                    info!(
                        "Ignoring report on admin user {} (ID: {})",
                        reported_username, reported_user_id
                    );
                    
                    bot.send_message(
                        msg.chat.id,
                        "Я не буду проверять сообщения от администраторов чата, найдите себе другую забаву."
                    ).await?;
                    
                    return Ok(());
                }
                
                info!(
                    "User | {} | with id: {} reported message in chat {}",
                    username, user_id, msg.chat.username().unwrap_or_default()
                );
                
                let reported_message_id = replied_msg.id;
                let reported_chat_id = msg.chat.id;
                let reported_text = replied_msg.text().unwrap_or("Empty text");
                let chat_title = replied_msg.chat.title().unwrap_or_else(|| "Unknown Chat");

                let reports_count = {
                    let message_reports = app_state.message_reports.as_ref().unwrap();
                    let mut reports = message_reports.lock().await;
                    let message_id = replied_msg.id.0;
                    let count = reports.add_report(msg.chat.id.0, message_id);
                    
                    if let Err(e) = reports.save_message_reports(&app_state.app_name).await {
                        error!("Error saving message reports: {}", e);
                    }

                    count
                };

                info!(
                    "Message reported: chat_id: {}, message_id: {}, text: {}, total reports: {}",
                    reported_chat_id, reported_message_id, reported_text, reports_count
                );
                
                if reports_count >= 3 {
                    info!(
                        "Message received 3 or more reports, deleting: chat_id: {}, message_id: {}",
                        reported_chat_id, reported_message_id
                    );
                    
                    if let Err(e) = bot.delete_message(reported_chat_id, reported_message_id).await {
                        error!("Error deleting message: {:?}", e);
                        bot.send_message(
                            msg.chat.id,
                            "Не удалось удалить сообщение. Возможно, оно уже удалено или у бота нет прав."
                        ).await?;
                    } else {
                        bot.send_message(
                            msg.chat.id,
                            "Сообщение было удалено по многочисленным жалобам участников чата."
                        ).await?;
                    }
                } else {
                    let message_to_check = format!(
                        "Текст проверяемого сообщения: \"{}\"\nВНИМАНИЕ! На сообщение поступило: {} жалоб, сообщение необходимо проверить с особым пристрастием!",
                        reported_text, reports_count
                    );

                    if let Err(err) = ai_check(
                        bot.clone(),
                        replied_msg.clone(),
                        &message_to_check,
                        true,
                        &app_state.app_name,
                        chat_title,
                        &reported_username,
                        reported_user_id as u64,
                        app_state.clone()
                    ).await {
                        error!("Error processing reported message: {:?}", err);
                        bot.send_message(
                            msg.chat.id,
                            "Произошла ошибка при обработке сообщения. Пожалуйста, попробуйте позже."
                        ).await?;
                        return Ok(());
                    }
                    
                    let confirm_msg = format!(
                        "Сообщение зарегистрировано для проверки (всего жалоб: {}). Спасибо за бдительность!",
                        reports_count
                    );
                    bot.send_message(msg.chat.id, confirm_msg).await?;
                }
            } else {
                let warn_msg = "Команда /groot должна быть использована в ответ на сообщение, которое вы хотите пометить.";
                bot.send_message(msg.chat.id, warn_msg).await?;
            }
        },
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

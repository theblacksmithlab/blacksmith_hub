use crate::groot_bot::groot_bot_utils::load_super_admins;
use anyhow::Result;
use core::models::common::system_messages::{AppsSystemMessages, GrootBotMessages};
use core::models::tg_bot::groot_bot::groot_bot::ResourcesDialogState;
use core::models::tg_bot::groot_bot::groot_bot_commands::GrootBotCommands;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use std::sync::Arc;
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;
use tracing::info;

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
        .from
        .unwrap()
        .username
        .unwrap_or("Anonymous User".to_string());

    if cmd != GrootBotCommands::Start && !msg.chat.is_private() {
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

    if cmd == GrootBotCommands::Resources && !super_admins.contains(&user_id) {
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

    if cmd == GrootBotCommands::Logs && !super_admins.contains(&user_id) {
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
        if !msg.chat.is_private() {
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

    // let mut dialog_states = app_state.dialog_states.lock().await;
    // let state = dialog_states.entry(user_id).or_insert(ResourcesDialogState {
    //     awaiting_option_choice: false,
    //     awaiting_edit_type: false,
    //     awaiting_show_type: false,
    //     edit_type: EditType::None,
    //     show_type: ShowType::None,
    //     awaiting_data_entry: false,
    //     awaiting_ask_message: false,
    // });

    match cmd {
        GrootBotCommands::Start => {
            bot.send_message(
                msg.chat.id,
                format!("Hello, here's super admins: {:?}!", super_admins),
            )
            .await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "invalid cmd").await?;
        }
    }

    Ok(())
}

pub async fn groot_bot_message_handler() -> Result<()> {
    Ok(())
}

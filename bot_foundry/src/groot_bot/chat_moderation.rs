use crate::groot_bot::chat_moderation_utils::{
    ai_check, check_sender, is_user_active, media_restriction_check, message_caption_check,
    message_entities_check, message_with_web_url_check, restricted_words_check,
    save_message_counts_to_file, scam_emojis_check, scam_stories_check, update_user_message_count,
    via_bot_message_check,
};
use crate::groot_bot::resources_cmd_handler::resources_cmd_handler;
use anyhow::Result;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::tg_bot::groot_bot::groot_bot_utils::{
    is_message_from_linked_channel, load_black_listed_users,
    load_white_listed_users,
};
use std::sync::Arc;
use teloxide::prelude::Message;
use teloxide::Bot;
use teloxide_core::prelude::{Request, Requester};
use tracing::{error, info};

pub async fn chat_moderation(
    bot: Bot,
    msg: Message,
    app_state: Arc<BotAppState>,
    is_paid_chat: bool,
) -> Result<()> {
    let user_id = msg.clone().from.unwrap().id.0;
    let username = msg
        .from
        .as_ref()
        .map(|user| {
            if let Some(username) = &user.username {
                username.to_string()
            } else {
                let first_name = &user.first_name;
                let last_name = user.last_name.as_deref().unwrap_or("");

                if !first_name.is_empty() || !last_name.is_empty() {
                    format!("{} {}", first_name, last_name).trim().to_string()
                } else {
                    "Anonymous User".to_string()
                }
            }
        })
        .unwrap_or_else(|| "Anonymous User".to_string());

    if let Some(dialog_states_mutex) = &app_state.dialog_states {
        let mut dialog_states = dialog_states_mutex.lock().await;

        if let Some(state) = dialog_states.get_mut(&user_id) {
            if state.awaiting_option_choice
                || state.awaiting_show_type
                || state.awaiting_edit_type
                || state.awaiting_data_entry
            {
                return resources_cmd_handler(
                    bot,
                    msg,
                    state,
                    app_state.clone(),
                    &username,
                    user_id,
                )
                .await;
            }
        }
    }

    let mut is_admin = false;
    let mut is_from_linked_channel = false;

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
        info!("Message from linked channel detected");
    }

    if is_admin {
        info!("Message from chat admin - skipping moderation");
        return Ok(());
    }

    if is_from_linked_channel {
        info!("Message from linked channel - skipping moderation");
        return Ok(());
    }

    let app_name = &app_state.app_name;
    let chat_title = msg.chat.title().unwrap_or_else(|| "Unknown Chat");
    // let _paid_chats = load_paid_chats(app_name);
    // let is_paid_chat = true;
    // paid_chats.contains(&msg.chat.id.0);
    let white_listed_users = load_white_listed_users(app_name);
    let black_listed_users = load_black_listed_users(app_name);
    let message_to_check = if let Some(text) = msg.text() {
        text.to_lowercase()
    } else if let Some(caption) = msg.caption() {
        caption.to_lowercase()
    } else {
        "Empty text".to_string()
    };
    let truncated_message: String = message_to_check
        .char_indices()
        .nth(100)
        .map(|(idx, _)| &message_to_check[..idx])
        .unwrap_or(&message_to_check)
        .to_string();

    info!(
        "[START processing message] Message to check: | {} | in chat: | {} | from user: | {} | with user_id: | {} |",
        truncated_message, chat_title, username, user_id
    );

    if check_sender(
        bot.clone(),
        msg.clone(),
        white_listed_users,
        black_listed_users,
        chat_title,
        is_paid_chat,
        app_name,
        &username,
    )
    .await?
    .is_some()
    {
        return Ok(());
    };

    if is_user_active(
        app_state.clone(),
        msg.chat.id.0,
        user_id,
        &username,
        chat_title,
    )
    .await
    {
        return Ok(());
    }

    if via_bot_message_check(
        bot.clone(),
        msg.clone(),
        chat_title,
        is_paid_chat,
        app_name,
        &username,
        user_id,
    )
    .await?
    .is_some()
    {
        return Ok(());
    };

    if scam_stories_check(
        bot.clone(),
        msg.clone(),
        chat_title,
        is_paid_chat,
        app_name,
        &username,
        user_id,
    )
    .await?
    .is_some()
    {
        return Ok(());
    };

    // anonymous_user_treatment(bot.clone(), msg.clone(), is_paid_chat, app_name, chat_title, &username, user_id).await?;

    if scam_emojis_check(
        bot.clone(),
        msg.clone(),
        &message_to_check,
        is_paid_chat,
        app_name,
        chat_title,
        &username,
        user_id,
    )
    .await?
    .is_some()
    {
        return Ok(());
    }

    if restricted_words_check(
        bot.clone(),
        msg.clone(),
        &message_to_check,
        is_paid_chat,
        app_name,
        chat_title,
        &username,
        user_id,
    )
    .await?
    .is_some()
    {
        return Ok(());
    }

    if message_entities_check(
        bot.clone(),
        msg.clone(),
        is_paid_chat,
        app_name,
        chat_title,
        &username,
        user_id,
    )
    .await?
    .is_some()
    {
        return Ok(());
    };

    if message_caption_check(
        bot.clone(),
        msg.clone(),
        is_paid_chat,
        app_name,
        chat_title,
        &username,
        user_id,
    )
    .await?
    .is_some()
    {
        return Ok(());
    };

    if message_with_web_url_check(
        bot.clone(),
        msg.clone(),
        &message_to_check,
        is_paid_chat,
        app_name,
        chat_title,
        &username,
        user_id,
    )
    .await?
    .is_some()
    {
        return Ok(());
    };

    if media_restriction_check(
        bot.clone(),
        msg.clone(),
        is_paid_chat,
        chat_title,
        &username,
        user_id,
        app_name,
    )
    .await?
    .is_some()
    {
        return Ok(());
    };

    if ai_check(
        bot.clone(),
        msg.clone(),
        &message_to_check,
        is_paid_chat,
        app_name,
        chat_title,
        &username,
        user_id,
        app_state.clone(),
    )
    .await?
    .is_some()
    {
        return Ok(());
    };

    info!(
        "[Message processed successfully] Checked message: | {} | in chat: | {} | from user: | {} | with user_id: | {} |",
        truncated_message, chat_title, username, user_id
    );

    update_user_message_count(
        app_state.clone(),
        chat_title,
        msg.chat.id.0,
        user_id,
        &username,
    )
    .await;
    save_message_counts_to_file(app_state.clone()).await;

    Ok(())
}

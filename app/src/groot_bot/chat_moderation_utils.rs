use crate::groot_bot::groot_bot_utils::{
    count_emojis, load_scam_domains, paid_chat_spam_warning, parsing_restricted_words,
    unpaid_chat_spam_warning,
};
use anyhow::Result;
use core::ai::common::common::raw_llm_processing_json;
use core::models::common::app_name::AppName;
use core::models::common::system_messages::{AppsSystemMessages, GrootBotMessages};
use core::models::common::system_roles::GrootRoleType;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::common::get_system_role_or_fallback;
use core::utils::common::LlmModel;
use regex::Regex;
use std::collections::HashSet;
use std::sync::Arc;
use teloxide::types::{MediaKind, Message, MessageKind};
use teloxide::Bot;
use tracing::{error, info, warn};

pub async fn check_sender(
    bot: Bot,
    msg: Message,
    white_listed_users: HashSet<i64>,
    black_listed_users: HashSet<i64>,
    chat_title: &str,
    is_paid_chat: bool,
    app_name: &AppName,
    username: &str,
) -> Result<Option<()>> {
    if let Some(sender_chat) = &msg.sender_chat {
        let sender_chat_id = sender_chat.id.0;
        if white_listed_users.contains(&sender_chat_id) {
            info!("Got message from white-listed channel or chat... Ok");
            return Ok(Some(()));
        }

        let violator_chat_title = msg.sender_chat.as_ref().map_or_else(
            || "Unknown Chat".to_string(),
            |chat| {
                chat.title()
                    .map(|title| title.to_string())
                    .unwrap_or_else(|| {
                        chat.username()
                            .map(|username| format!("@{}", username))
                            .unwrap_or_else(|| "Unknown Chat".to_string())
                    })
            },
        );

        if is_paid_chat {
            let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                GrootBotMessages::AlertForViolatorChannels,
            ))
            .await?;
            let formatted_bot_system_message_text =
                bot_system_message_text.replace("{}", &violator_chat_title);

            paid_chat_spam_warning(
                bot.clone(),
                &msg,
                msg.thread_id,
                formatted_bot_system_message_text,
                format!(
                    "Got message from NON-white-listed channel: {} ... message DELETED",
                    violator_chat_title
                ),
                app_name,
                chat_title,
                username,
            )
            .await?;

            return Ok(Some(()));
        } else {
            unpaid_chat_spam_warning(bot.clone(), &msg, msg.thread_id, chat_title).await?;
            return Ok(Some(()));
        }
    }

    if let Some(user) = &msg.from {
        let user_id = user.id.0 as i64;

        if white_listed_users.contains(&user_id) {
            info!("Got message from white-listed User... Ok");
            return Ok(Some(()));
        }

        if black_listed_users.contains(&user_id) {
            if is_paid_chat {
                let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                    GrootBotMessages::AlertForBlackListed,
                ))
                .await?;
                let formatted_bot_system_message_text =
                    bot_system_message_text.replace("{}", &username);

                paid_chat_spam_warning(
                    bot.clone(),
                    &msg,
                    msg.thread_id,
                    formatted_bot_system_message_text,
                    format!(
                        "Got message from black-listed user: {} with id: {} ... message DELETED",
                        username, user_id
                    ),
                    app_name,
                    chat_title,
                    username,
                )
                .await?;
                return Ok(Some(()));
            } else {
                unpaid_chat_spam_warning(bot.clone(), &msg, msg.thread_id, chat_title).await?;
                return Ok(Some(()));
            }
        }
    } else {
        warn!("Received a message with NO sender_chat id and NO user id!");
    }

    Ok(None)
}

pub async fn via_bot_message_check(
    bot: Bot,
    msg: Message,
    chat_title: &str,
    is_paid_chat: bool,
    app_name: &AppName,
    username: &str,
    user_id: u64,
) -> Result<Option<()>> {
    if msg.via_bot.is_some() {
        if is_paid_chat {
            let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                GrootBotMessages::DefaultScamAlert,
            ))
            .await?;
            let formatted_bot_system_message_text =
                bot_system_message_text.replace("{}", &username);

            paid_chat_spam_warning(
                bot.clone(),
                &msg,
                msg.thread_id,
                formatted_bot_system_message_text,
                format!(
                    "Inline bot message detected... message DELETED. | Violator id: {}",
                    user_id
                ),
                app_name,
                chat_title,
                username,
            )
            .await?;
            return Ok(Some(()));
        } else {
            unpaid_chat_spam_warning(bot, &msg, msg.thread_id, chat_title).await?;
            return Ok(Some(()));
        }
    }

    Ok(None)
}

pub async fn scam_stories_check(
    bot: Bot,
    msg: Message,
    chat_title: &str,
    is_paid_chat: bool,
    app_name: &AppName,
    username: &str,
    user_id: u64,
) -> Result<Option<()>> {
    if let MessageKind::Common(common) = &msg.kind {
        if let MediaKind::Story(_) = common.media_kind {
            if is_paid_chat {
                let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                    GrootBotMessages::DefaultScamAlert,
                ))
                .await?;
                let formatted_bot_system_message_text =
                    bot_system_message_text.replace("{}", &username);

                paid_chat_spam_warning(
                    bot.clone(),
                    &msg,
                    msg.thread_id,
                    formatted_bot_system_message_text,
                    format!(
                        "Scam-story detected... message DELETED. | Violator id: {}",
                        user_id,
                    ),
                    app_name,
                    chat_title,
                    username,
                )
                .await?;
                return Ok(Some(()));
            } else {
                unpaid_chat_spam_warning(bot.clone(), &msg, msg.thread_id, chat_title).await?;
                return Ok(Some(()));
            }
        }
    }

    Ok(None)
}

pub async fn anonymous_user_treatment(
    bot: Bot,
    msg: Message,
    is_paid_chat: bool,
    app_name: &AppName,
    chat_title: &str,
    username: &str,
    user_id: u64,
) -> Result<()> {
    let username_check = msg.from.as_ref().and_then(|user| user.username.as_ref());

    let username_synthetic = msg
        .from
        .as_ref()
        .map(|user| {
            let first_name = &user.first_name;
            info!(
                "TEMP log: Telegram channel's message case check: first name: {}",
                first_name
            );
            let last_name = user.last_name.as_deref().unwrap_or("");
            info!(
                "TEMP log: Telegram channel's message case check: last name: {}",
                last_name
            );
            format!("{} {}", first_name, last_name).trim().to_string()
        })
        .unwrap_or_default();

    if username_check.is_none() {
        if let Some(user) = msg.from.as_ref() {
            if user.first_name == "Telegram" {
                info!("Got message from parent Telegram channel... Ok");
                return Ok(());
            } else {
                if is_paid_chat {
                    let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                        GrootBotMessages::AnonymousUserAlert,
                    ))
                    .await?;
                    let formatted_bot_system_message_text =
                        bot_system_message_text.replace("{}", &username_synthetic);

                    paid_chat_spam_warning(
                        bot.clone(),
                        &msg,
                        msg.thread_id,
                        formatted_bot_system_message_text,
                        format!("Anon with no username sent message... message DELETED. | Violator id: {}", user_id),
                        app_name,
                        chat_title,
                        username,
                    )
                        .await?;
                    return Ok(());
                } else {
                    unpaid_chat_spam_warning(bot, &msg, msg.thread_id, chat_title).await?;
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

pub async fn scam_emojis_check(
    bot: Bot,
    msg: Message,
    message_to_check: &str,
    is_paid_chat: bool,
    app_name: &AppName,
    chat_title: &str,
    username: &str,
    user_id: u64,
) -> Result<Option<()>> {
    let emoji_count = count_emojis(message_to_check);

    if emoji_count > 5 {
        if is_paid_chat {
            let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                GrootBotMessages::DefaultScamAlert,
            ))
            .await?;
            let formatted_bot_system_message_text = bot_system_message_text.replace("{}", username);
            paid_chat_spam_warning(
                bot.clone(),
                &msg,
                msg.thread_id,
                formatted_bot_system_message_text,
                format!(
                    "Message contains more than 5 emojis, presumably scam... message DELETED. | Violator id: {}",
                    user_id
                ),
                app_name,
                chat_title,
                username
            ).await?;
            return Ok(Some(()));
        } else {
            unpaid_chat_spam_warning(bot, &msg, msg.thread_id, chat_title).await?;
            return Ok(Some(()));
        }
    }

    Ok(None)
}

pub async fn restricted_words_check(
    bot: Bot,
    msg: Message,
    message_to_check: &str,
    is_paid_chat: bool,
    app_name: &AppName,
    chat_title: &str,
    username: &str,
    user_id: u64,
) -> Result<Option<()>> {
    if parsing_restricted_words(app_name, message_to_check) {
        if is_paid_chat {
            let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                GrootBotMessages::DefaultScamAlert,
            ))
            .await?;
            let formatted_bot_system_message_text = bot_system_message_text.replace("{}", username);

            paid_chat_spam_warning(
                bot.clone(),
                &msg,
                msg.thread_id,
                formatted_bot_system_message_text,
                format!(
                    "Restricted keyword detected... message DELETED. | Violator id: {}",
                    user_id
                ),
                app_name,
                chat_title,
                username,
            )
            .await?;
            return Ok(Some(()));
        } else {
            unpaid_chat_spam_warning(bot, &msg, msg.thread_id, chat_title).await?;
            return Ok(Some(()));
        }
    }

    Ok(None)
}

pub async fn message_with_web_url_check(
    bot: Bot,
    msg: Message,
    message_to_check: &str,
    is_paid_chat: bool,
    app_name: &AppName,
    chat_title: &str,
    username: &str,
    user_id: u64,
) -> Result<Option<()>> {
    let url_pattern = Regex::new(r"(?i)\b((?:https?://|www\.)?[\w.-]+\.\w{2,})(?:\S+)?")?;

    if url_pattern.is_match(&message_to_check) {
        info!("Some url detected in message text... Let's check it in the scam-domains base...");

        let scam_domains = load_scam_domains(&app_name)?;

        for url in url_pattern.find_iter(&message_to_check) {
            let url_text = url.as_str();

            let cleaned_url = url_text
                .replace("http://", "")
                .replace("https://", "")
                .replace("www.", "");

            let domain = cleaned_url.split('/').next().unwrap_or(&cleaned_url);

            if scam_domains.iter().any(|d| domain == d) {
                if is_paid_chat {
                    let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                        GrootBotMessages::ScamDomainAlert,
                    ))
                    .await?;
                    let formatted_bot_system_message_text =
                        bot_system_message_text.replace("{}", username);

                    paid_chat_spam_warning(
                        bot.clone(),
                        &msg,
                        msg.thread_id,
                        formatted_bot_system_message_text,
                        format!(
                            "Scam domain link detected... message DELETED. | Violator id: {}",
                            user_id
                        ),
                        app_name,
                        chat_title,
                        username,
                    )
                    .await?;
                    return Ok(Some(()));
                } else {
                    unpaid_chat_spam_warning(bot.clone(), &msg, msg.thread_id, chat_title).await?;
                    return Ok(Some(()));
                }
            }
        }
    }

    Ok(None)
}

pub async fn message_caption_check(
    bot: Bot,
    msg: Message,
    is_paid_chat: bool,
    app_name: &AppName,
    chat_title: &str,
    username: &str,
    user_id: u64,
) -> Result<Option<()>> {
    if let Some(caption_entities) = msg.caption_entities() {
        for caption in caption_entities {
            match &caption.kind {
                teloxide::types::MessageEntityKind::TextLink { .. } => {
                    if is_paid_chat {
                        let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                            GrootBotMessages::DefaultScamAlert,
                        ))
                        .await?;
                        let formatted_bot_system_message_text =
                            bot_system_message_text.replace("{}", username);

                        paid_chat_spam_warning(
                            bot.clone(),
                            &msg,
                            msg.thread_id,
                            formatted_bot_system_message_text,
                            format!(
                                "Link detected in caption entities... message DELETED. | Violator id: {}",
                                user_id
                            ),
                            app_name,
                            chat_title,
                            username
                        )
                            .await?;
                        return Ok(Some(()));
                    } else {
                        unpaid_chat_spam_warning(bot, &msg, msg.thread_id, chat_title).await?;
                        return Ok(Some(()));
                    }
                }
                _ => continue,
            }
        }
    }

    Ok(None)
}

pub async fn message_entities_check(
    bot: Bot,
    msg: Message,
    is_paid_chat: bool,
    app_name: &AppName,
    chat_title: &str,
    username: &str,
    user_id: u64,
) -> Result<Option<()>> {
    if let Some(entities) = msg.entities() {
        for entity in entities {
            match &entity.kind {
                teloxide::types::MessageEntityKind::TextLink { .. } => {
                    if is_paid_chat {
                        let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                            GrootBotMessages::DefaultScamAlert,
                        ))
                        .await?;
                        let formatted_bot_system_message_text =
                            bot_system_message_text.replace("{}", username);

                        paid_chat_spam_warning(
                            bot.clone(),
                            &msg,
                            msg.thread_id,
                            formatted_bot_system_message_text,
                            format!(
                                "Link detected in message entities... message DELETED. | Violator id: {}",
                                user_id
                            ),
                            app_name,
                            chat_title,
                            username
                        )
                            .await?;
                        return Ok(Some(()));
                    } else {
                        unpaid_chat_spam_warning(bot, &msg, msg.thread_id, chat_title).await?;
                        return Ok(Some(()));
                    }
                }
                _ => continue,
            }
        }
    }

    Ok(None)
}

pub async fn media_restriction_check(
    bot: Bot,
    msg: Message,
    is_paid_chat: bool,
    chat_title: &str,
    username: &str,
    user_id: u64,
    app_name: &AppName,
) -> Result<Option<()>> {
    if msg.photo().is_some() || msg.video().is_some() {
        if is_paid_chat {
            let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                GrootBotMessages::MediaRestrictionAlert,
            ))
            .await?;
            let formatted_bot_system_message_text = bot_system_message_text.replace("{}", username);

            paid_chat_spam_warning(
                bot.clone(),
                &msg,
                msg.thread_id,
                formatted_bot_system_message_text,
                format!(
                    "Image/video from chat's newbie detected... message DELETED. | Violator id: {}",
                    user_id
                ),
                app_name,
                chat_title,
                username,
            )
            .await?;

            return Ok(Some(()));
        } else {
            unpaid_chat_spam_warning(bot, &msg, msg.thread_id, chat_title).await?;
            return Ok(Some(()));
        }
    }

    Ok(None)
}

pub async fn ai_check(
    bot: Bot,
    msg: Message,
    message_to_check: &str,
    is_paid_chat: bool,
    app_name: &AppName,
    chat_title: &str,
    username: &str,
    user_id: u64,
    app_state: Arc<BotAppState>,
) -> Result<Option<()>> {
    let system_role =
        get_system_role_or_fallback(&AppName::GrootBot, GrootRoleType::MessageCheck, None);

    let scam_detection_result =
        raw_llm_processing_json(&system_role, message_to_check, app_state, LlmModel::Complex)
            .await?;

    let is_scam: bool = match serde_json::from_str::<serde_json::Value>(&scam_detection_result) {
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

    if is_scam {
        if is_paid_chat {
            let bot_system_message_text = get_message(AppsSystemMessages::GrootBot(
                GrootBotMessages::LLMCheckAlert,
            ))
            .await?;
            let formatted_bot_system_message_text = bot_system_message_text.replace("{}", username);

            paid_chat_spam_warning(
                bot.clone(),
                &msg,
                msg.thread_id,
                formatted_bot_system_message_text,
                format!(
                    "Spam message detected by LLM... message DELETED. | Violator id: {}",
                    user_id
                ),
                app_name,
                chat_title,
                username,
            )
            .await?;
            return Ok(Some(()));
        } else {
            unpaid_chat_spam_warning(bot, &msg, msg.thread_id, chat_title).await?;
            return Ok(Some(()));
        }
    }

    info!(
        "[Scam detection ai-system] Result for message {} from user: {} with id: {} is: {}",
        message_to_check, username, user_id, is_scam
    );

    Ok(None)
}

pub async fn is_user_active(
    app_state: Arc<BotAppState>,
    chat_id: i64,
    user_id: u64,
    username: &str,
    chat_title: &str,
) -> bool {
    let message_counts_num = {
        let counts = app_state.message_counts.as_ref().unwrap().lock().await;
        counts.get_message_count(chat_id, user_id)
    };

    let chat_message_stats_num = {
        let stats = app_state.chat_message_stats.as_ref().unwrap().lock().await;
        stats
            .fetching_message_counts
            .get(&chat_id)
            .and_then(|users| users.get(&user_id))
            .cloned()
            .unwrap_or(0)
    };

    if message_counts_num > 10 || chat_message_stats_num > 10 {
        info!(
            "User: {} with id: {} is active enough in chat: {}. \
            Found {} msgs in MessageCounts and {} in ChatMessageStats. \
            Pass further checks.",
            username, user_id, chat_title, message_counts_num, chat_message_stats_num
        );
        return true;
    }

    info!(
        "User: {} with id: {} sent {} msgs in MessageCounts and {} in ChatMessageStats \
        in chat: {}. Continue checking...",
        username, user_id, message_counts_num, chat_message_stats_num, chat_title
    );

    false
}

pub async fn update_user_message_count(
    app_state: Arc<BotAppState>,
    chat_title: &str,
    chat_id: i64,
    user_id: u64,
    username: &str,
) {
    let mut counts = app_state.message_counts.as_ref().unwrap().lock().await;

    counts.increment_message_count(chat_id, user_id);

    info!(
        "Messages quantity for user: {} with id: {} successfully updated in chat {} with id: {}. Actual messages quantity: {}",
        username,
        user_id,
        chat_title,
        chat_id,
        counts.get_message_count(chat_id, user_id)
    );
}

pub async fn save_message_counts_to_file(app_state: Arc<BotAppState>) {
    let counts = app_state.message_counts.as_ref().unwrap().lock().await;

    if let Err(e) = counts.save_message_counts(&app_state.app_name).await {
        error!("Error saving message_counts to file: {}", e);
    } else {
        info!("Message_counts data successfully saved to file");
    }
}

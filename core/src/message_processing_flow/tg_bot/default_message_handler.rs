use crate::message_processing_flow::message_processing_flow::process_user_query;
use crate::models::common::app_name::AppName;
use crate::models::common::system_messages::AppsSystemMessages;
use crate::models::common::system_messages::{CommonMessages, ProbiotBotMessages};
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::state::qdrant_client_init_trait::QdrantClientInit;
use crate::state::tg_bot::app_state::AppNameProvider;
use crate::temp_cache::temp_cache_traits::TempCacheInit;
use crate::utils::common::{convert_markdown_to_telegram, get_message, markdown_to_html, transcribe_voice_message};
use crate::utils::tg_bot::tg_bot::{add_llm_response_to_cache, download_voice, get_chat_title, get_username_from_message, start_bots_chat_action, stop_bots_chat_action};
use crate::utils::tg_bot::tg_bot::{append_footer_if_needed, create_tts_button, save_tts_payload};
use std::sync::Arc;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Message, Requester};
use teloxide::types::{ChatAction, ParseMode, ReplyParameters};
use teloxide::Bot;
use tokio::sync::Mutex;
use tracing::error;
use tracing::log::info;
use uuid::Uuid;

pub async fn default_message_handler<T>(
    bot: Bot,
    msg: Message,
    app_state: Arc<T>,
) -> anyhow::Result<()>
where
    T: AppNameProvider
        + OpenAIClientInit
        + QdrantClientInit
        + TempCacheInit
        + Send
        + Sync
        + 'static,
{
    let app_name = app_state.app_name();
    let chat_id = msg.chat.id;
    let user = msg
        .from
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let user_id = user.id.0;
    let username = get_username_from_message(&msg);
    let chat_id_as_integer = chat_id.0;
    let chat_id_as_str = chat_id_as_integer.to_string();
    let bot_data = bot.get_me().await?;
    let user_raw_request = msg.text().unwrap_or("Empty request").to_string();
    let chat_title = get_chat_title(&msg);

    if msg.chat.is_private() {
        info!(
            "Got message: {} from user: {} [{}]",
            user_raw_request,
            user_id, username
        );

        if let Some(voice) = msg.voice() {
            info!(
                "Message received from user: {} [{}] is voice message. Let's process it...",
                user_id, username
            );

            let typing_flag = Arc::new(Mutex::new(true));
            start_bots_chat_action(
                bot.clone(),
                chat_id,
                ChatAction::Typing,
                Arc::clone(&typing_flag),
            )
            .await;

            let file_path = app_name.temp_dir().join(format!("{}.ogg", voice.file.id));

            match download_voice(&bot, &voice.file.id, &file_path).await {
                Ok(path) => path,
                Err(err) => {
                    error!("Failed to download voice message: {}", err);
                    let bot_msg = get_message(AppsSystemMessages::Common(
                        CommonMessages::ErrorDownloadingVoiceMessageFile,
                    ))
                    .await?;
                    stop_bots_chat_action(typing_flag).await;
                    bot.send_message(chat_id, bot_msg).await?;
                    return Ok(());
                }
            };

            match transcribe_voice_message(&file_path).await {
                Ok(Some(user_voice_transcribed)) => {
                    info!("Voice message transcribed successfully...");
                    match process_user_query(
                        &chat_id_as_str,
                        &user_voice_transcribed,
                        app_state.clone(),
                        app_name.clone(),
                    )
                    .await
                    {
                        Ok((llm_response, _extra_data)) => {
                            let full_response = append_footer_if_needed(
                                &llm_response,
                                app_state.clone(),
                                &chat_id_as_str,
                                app_name.clone(),
                            )
                            .await
                            .unwrap_or_else(|_| llm_response.clone());

                            let message_id = Uuid::new_v4().to_string();

                            save_tts_payload(
                                app_state.clone(),
                                chat_id,
                                &message_id,
                                &llm_response,
                            )
                            .await;

                            stop_bots_chat_action(typing_flag).await;

                            bot.send_message(chat_id, &full_response)
                                .reply_markup(create_tts_button(chat_id, &message_id))
                                .await?;

                            info!(
                                "Successfully processed voice message from user: {} [{}]",
                                user_id, username
                            );

                            add_llm_response_to_cache(
                                app_state.clone(),
                                &chat_id_as_str,
                                &full_response,
                            )
                            .await;
                        }
                        Err(err) => {
                            error!("Error in process_user_raw_request: {}", err);
                            let bot_msg = get_message(AppsSystemMessages::Common(
                                CommonMessages::ErrorProcessingRequest,
                            ))
                            .await?;
                            stop_bots_chat_action(typing_flag).await;
                            bot.send_message(chat_id, bot_msg).await?;
                        }
                    }
                }
                Ok(None) => {
                    let bot_msg = get_message(AppsSystemMessages::Common(
                        CommonMessages::ErrorProcessingVoiceMessage,
                    ))
                    .await?;
                    stop_bots_chat_action(typing_flag).await;
                    bot.send_message(chat_id, bot_msg).await?;
                }
                Err(err) => {
                    error!("Error in handle_voice_message: {}", err);
                    let bot_msg = get_message(AppsSystemMessages::Common(
                        CommonMessages::GlobalErrorProcessingVoiceMessage,
                    ))
                    .await?;
                    stop_bots_chat_action(typing_flag).await;
                    bot.send_message(chat_id, bot_msg).await?;
                }
            }
        } else if let Some(text) = msg.text() {
            info!(
                "Message received from user {} [{}] is a text message. Let's process it...",
                user_id, username
            );

            let typing_flag = Arc::new(Mutex::new(true));
            start_bots_chat_action(
                bot.clone(),
                chat_id,
                ChatAction::Typing,
                Arc::clone(&typing_flag),
            )
            .await;

            match process_user_query(&chat_id_as_str, text, app_state.clone(), app_name.clone())
                .await
            {
                Ok((llm_response, _extra_data)) => {
                    // TODO: Truncate long message to 4096 symbols

                    let full_response = append_footer_if_needed(
                        &llm_response,
                        app_state.clone(),
                        &chat_id_as_str,
                        app_name.clone(),
                    )
                    .await
                    .unwrap_or_else(|_| llm_response.clone());

                    let converted_to_markdown_v2_full_response =
                        convert_markdown_to_telegram(&full_response);

                    // Testing
                    let htmled_full_response = markdown_to_html(&full_response);

                    let message_id = Uuid::new_v4().to_string();

                    save_tts_payload(app_state.clone(), chat_id, &message_id, &llm_response).await;

                    stop_bots_chat_action(typing_flag).await;

                    bot.send_message(chat_id, htmled_full_response)
                        .reply_markup(create_tts_button(chat_id, &message_id))
                        .parse_mode(ParseMode::Html)
                        .await?;

                    info!(
                        "Successfully processed text message from user: {} [{}]",
                        user_id, username
                    );

                    add_llm_response_to_cache(app_state.clone(), &chat_id_as_str, &full_response)
                        .await;
                }
                Err(err) => {
                    error!("Error in process_user_raw_request: {}", err);
                    let bot_msg = get_message(AppsSystemMessages::Common(
                        CommonMessages::ErrorProcessingRequest,
                    ))
                    .await?;
                    stop_bots_chat_action(typing_flag).await;
                    bot.send_message(chat_id, bot_msg).await?;
                }
            }
        } else {
            info!(
                "Message received from user: {} [{}] is neither voice nor text. No need to process it...",
                user_id, username
            );
            let bot_msg = get_message(AppsSystemMessages::Common(
                CommonMessages::InvalidRequestContent,
            ))
            .await?;
            bot.send_message(chat_id, bot_msg).await?;
        }
    } else {
        if user_raw_request.contains(&format!(
            "@{}",
            bot_data.user.clone().username.unwrap_or_default()
        )) || (msg.reply_to_message().is_some()
            && msg
                .reply_to_message()
                .and_then(|reply| reply.from.as_ref())
                .map(|user| user.id == bot_data.id)
                .unwrap_or(false))
        {
            info!(
                "Got message from user: {} [{}] in public chat '{}'. User invited for private messaging",
                user_id, username, chat_title
            );

            if let Some(message_enum) = match app_name {
                AppName::ProbiotBot => Some(AppsSystemMessages::ProbiotBot(
                    ProbiotBotMessages::PrivateChatInvitation,
                )),
                _ => None,
            } {
                let bot_msg = get_message(message_enum).await?;
                bot.send_message(chat_id, bot_msg)
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
            }
        }
    }

    Ok(())
}

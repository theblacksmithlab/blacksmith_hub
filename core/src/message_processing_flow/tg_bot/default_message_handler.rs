use crate::message_processing_flow::message_processing_flow::process_user_raw_request;
use crate::models::common::app_name::AppName;
use crate::models::common::system_messages::AppsSystemMessages;
use crate::models::common::system_messages::{CommonMessages, ProbiotBotMessages, W3ABotMessages};
use crate::state::tg_bot::app_state::BotAppState;
use crate::utils::common::{convert_markdown_to_telegram, get_message, transcribe_voice_message};
use crate::utils::tg_bot::tg_bot::{
    add_llm_response_to_cache, download_voice, start_bots_chat_action, stop_bots_chat_action,
};
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

pub async fn default_message_handler(
    bot: Bot,
    msg: Message,
    app_state: Arc<BotAppState>,
) -> anyhow::Result<()> {
    let app_name = &app_state.app_name;
    let chat_id = msg.chat.id;
    let chat_id_as_integer = chat_id.0;
    let chat_id_as_str = chat_id_as_integer.to_string();
    let bot_data = bot.get_me().await?;
    let user_raw_request = msg.text().unwrap_or("Empty request").to_string();
    let temp_dir = app_name.temp_dir();
    info!("TEMP log: temp dir: {:?}", temp_dir);

    if msg.chat.is_private() {
        info!(
            "Got message: {} from: @{}",
            user_raw_request,
            msg.chat.username().unwrap_or("Anonymous User")
        );

        if let Some(voice) = msg.voice() {
            info!(
                "Message received from @{} is voice message. Let's process it...",
                msg.chat.username().unwrap_or("Anonymous User")
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
                    match process_user_raw_request(
                        &chat_id_as_str,
                        &user_voice_transcribed,
                        app_state.clone(),
                        app_name.clone(),
                    )
                    .await
                    {
                        Ok(llm_response) => {
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
                                "Successfully processed voice message from @{}",
                                msg.chat.username().unwrap_or("Anonymous User")
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
                "Message received from @{} is a text message. Let's process it...",
                msg.chat.username().unwrap_or("Anonymous User")
            );

            let typing_flag = Arc::new(Mutex::new(true));
            start_bots_chat_action(
                bot.clone(),
                chat_id,
                ChatAction::Typing,
                Arc::clone(&typing_flag),
            )
            .await;

            match process_user_raw_request(
                &chat_id_as_str,
                text,
                app_state.clone(),
                app_name.clone(),
            )
            .await
            {
                Ok(llm_response) => {
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

                    let message_id = Uuid::new_v4().to_string();

                    save_tts_payload(app_state.clone(), chat_id, &message_id, &llm_response).await;

                    stop_bots_chat_action(typing_flag).await;

                    bot.send_message(chat_id, &converted_to_markdown_v2_full_response)
                        .reply_markup(create_tts_button(chat_id, &message_id))
                        .parse_mode(ParseMode::MarkdownV2)
                        .await?;

                    info!(
                        "Successfully processed text message from @{}",
                        msg.chat.username().unwrap_or("Anonymous User")
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
                "Message received from @{} is neither voice nor text. No need to process it...",
                msg.chat.username().unwrap_or("Anonymous User")
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
                "Got message from @{} in public chat. User invited for private messaging",
                msg.chat.username().unwrap_or("Anonymous User")
            );

            if let Some(message_enum) = match app_name {
                AppName::ProbiotBot => Some(AppsSystemMessages::Probiot(
                    ProbiotBotMessages::PrivateChatInvitation,
                )),
                AppName::W3ABot => Some(AppsSystemMessages::W3ABot(
                    W3ABotMessages::PrivateChatInvitation,
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

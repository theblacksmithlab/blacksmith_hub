use crate::probiot_bot::probiot_bot_utils::get_and_remove_tts_payload;
use anyhow::Result;
use core::ai::common::voice_processing::simple_tts;
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::ProbiotBotMessages;
use core::models::tg_bot::probiot_bot::probiot_bot_commands::ProbiotBotCommands;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::{start_bots_chat_action, stop_bots_chat_action};
use std::fs::remove_file;
use std::sync::Arc;
use teloxide::prelude::{CallbackQuery, ChatId, Message, Requester};
use teloxide::types::{ChatAction, InputFile};
use teloxide::Bot;
use tokio::sync::Mutex;
use tracing::error;
use tracing::log::info;

// pub(crate) async fn probiot_message_handler(
//     bot: Bot,
//     msg: Message,
//     app_state: Arc<BotAppState>,
// ) -> Result<()> {
//     let chat_id = msg.chat.id;
//     let bot_data = bot.get_me().await?;
//     let user_raw_request = msg.text().unwrap_or("Empty request").to_string();
//     info!(
//         "Got message: {} from: @{}",
//         user_raw_request,
//         msg.chat.username().unwrap_or("Anonymous User")
//     );
//
//     if msg.chat.is_private() {
//         if let Some(voice) = msg.voice() {
//             info!(
//                 "Message received from @{} is voice message. Let's process it...",
//                 msg.chat.username().unwrap_or("Anonymous User")
//             );
//
//             let typing_flag = Arc::new(Mutex::new(true));
//             start_bots_chat_action(
//                 bot.clone(),
//                 chat_id,
//                 ChatAction::Typing,
//                 Arc::clone(&typing_flag),
//             )
//             .await;
//
//             let file_path =
//                 match download_voice(&bot, &voice.file.id, &format!("tmp/{}.ogg", voice.file.id))
//                     .await
//                 {
//                     Ok(path) => path,
//                     Err(err) => {
//                         error!("Failed to download voice message: {}", err);
//                         let bot_msg = get_message(AppsSystemMessages::Common(CommonMessages::ErrorDownloadingVoiceMessageFile)).await?;
//                         stop_bots_chat_action(typing_flag).await;
//                         bot.send_message(chat_id, bot_msg).await?;
//                         return Ok(());
//                     }
//                 };
//
//             match transcribe_voice_message(&file_path).await {
//                 Ok(Some(user_voice_transcribed)) => {
//                     info!("Voice message transcribed successfully...");
//                     match process_user_raw_request(
//                         chat_id,
//                         user_voice_transcribed,
//                         app_state.clone(),
//                         app_name.clone()
//                     )
//                     .await
//                     {
//                         Ok(llm_response) => {
//                             let full_response = append_footer_if_needed(
//                                 llm_response.clone(),
//                                 app_state.clone(),
//                                 chat_id,
//                             )
//                             .await
//                             .unwrap_or_else(|_| llm_response.clone());
//
//                             let message_id = Uuid::new_v4().to_string();
//
//                             save_tts_payload(
//                                 app_state.clone(),
//                                 chat_id,
//                                 message_id.clone(),
//                                 llm_response.clone(),
//                             )
//                             .await;
//
//                             stop_bots_chat_action(typing_flag).await;
//
//                             bot.send_message(chat_id, full_response.clone())
//                                 .reply_markup(create_tts_button(chat_id, message_id))
//                                 .await?;
//
//                             info!(
//                                 "Successfully processed voice message from @{}",
//                                 msg.chat.username().unwrap_or("Anonymous User")
//                             );
//
//                             add_llm_response_to_cache(
//                                 app_state.clone(),
//                                 chat_id,
//                                 full_response.clone(),
//                             )
//                             .await;
//                         }
//                         Err(err) => {
//                             error!("Error in process_user_raw_request: {}", err);
//                             let bot_msg = get_message(AppsSystemMessages::Common(CommonMessages::ErrorProcessingRequest)).await?;
//                             stop_bots_chat_action(typing_flag).await;
//                             bot.send_message(chat_id, bot_msg).await?;
//                         }
//                     }
//                 }
//                 Ok(None) => {
//                     let bot_msg = get_message(AppsSystemMessages::Common(CommonMessages::ErrorProcessingVoiceMessage)).await?;
//                     stop_bots_chat_action(typing_flag).await;
//                     bot.send_message(chat_id, bot_msg).await?;
//                 }
//                 Err(err) => {
//                     error!("Error in handle_voice_message: {}", err);
//                     let bot_msg = get_message(AppsSystemMessages::Common(CommonMessages::GlobalErrorProcessingVoiceMessage)).await?;
//                     stop_bots_chat_action(typing_flag).await;
//                     bot.send_message(chat_id, bot_msg).await?;
//                 }
//             }
//         } else if let Some(text) = msg.text() {
//             info!(
//                 "Message received from @{} is text message. Let's process it...",
//                 msg.chat.username().unwrap_or("Anonymous User")
//             );
//
//             let typing_flag = Arc::new(Mutex::new(true));
//             start_bots_chat_action(
//                 bot.clone(),
//                 chat_id,
//                 ChatAction::Typing,
//                 Arc::clone(&typing_flag),
//             )
//             .await;
//
//             match process_user_raw_request(chat_id, text.to_string(), app_state.clone(), app_name.clone()).await {
//                 Ok(llm_response) => {
//                     let full_response = append_footer_if_needed(
//                         llm_response.clone(),
//                         app_state.clone(),
//                         chat_id,
//                     )
//                     .await
//                     .unwrap_or_else(|_| llm_response.clone());
//
//                     // let htmled_full_response = markdown_to_html(&full_response);
//
//                     let message_id = Uuid::new_v4().to_string();
//
//                     save_tts_payload(
//                         app_state.clone(),
//                         chat_id,
//                         message_id.clone(),
//                         llm_response.clone(),
//                     )
//                     .await;
//
//                     stop_bots_chat_action(typing_flag).await;
//
//                     bot.send_message(chat_id, full_response.clone())
//                         .reply_markup(create_tts_button(chat_id, message_id))
//                         .parse_mode(ParseMode::Html)
//                         .await?;
//
//                     info!(
//                         "Successfully processed text message from @{}",
//                         msg.chat.username().unwrap_or("Anonymous User")
//                     );
//
//                     add_llm_response_to_cache(app_state.clone(), chat_id, full_response.clone())
//                         .await;
//                 }
//                 Err(err) => {
//                     error!("Error in process_user_raw_request: {}", err);
//                     let bot_msg = get_message(AppsSystemMessages::Common(CommonMessages::ErrorProcessingRequest)).await?;
//                     stop_bots_chat_action(typing_flag).await;
//                     bot.send_message(chat_id, bot_msg).await?;
//                 }
//             }
//         } else {
//             info!(
//                 "Message received from @{} is neither voice nor text. No need to process it...",
//                 msg.chat.username().unwrap_or("Anonymous User")
//             );
//             let bot_msg = get_message(AppsSystemMessages::Common(CommonMessages::InvalidRequestContent)).await?;
//             bot.send_message(chat_id, bot_msg).await?;
//         }
//     } else {
//         info!(
//             "Got message from @{} in public chat. User invited for private messaging",
//             msg.chat.username().unwrap_or("Anonymous User")
//         );
//         if user_raw_request.contains(&format!(
//             "@{}",
//             bot_data.user.clone().username.unwrap_or_default()
//         )) || (msg.reply_to_message().is_some()
//             && msg
//                 .reply_to_message()
//                 .and_then(|reply| reply.from.as_ref())
//                 .map(|user| user.id == bot_data.id)
//                 .unwrap_or(false))
//         {
//             let bot_msg = get_message(AppsSystemMessages::Probiot(ProbiotBotMessages::PrivateChatInvitation)).await?;
//             bot.send_message(chat_id, bot_msg)
//                 .reply_parameters(ReplyParameters::new(msg.id))
//                 .await?;
//         }
//     }
//
//     Ok(())
// }

pub(crate) async fn probiot_command_handler(
    bot: Bot,
    msg: Message,
    cmd: ProbiotBotCommands,
    _app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = msg.chat.id;

    match cmd {
        ProbiotBotCommands::Start => {
            let bot_msg = get_message(AppsSystemMessages::Probiot(
                ProbiotBotMessages::StartMessage,
            ))
            .await?;
            bot.send_message(user_id, bot_msg).await?;
        }
    }

    Ok(())
}

pub(crate) async fn probiot_callback_query_handler(
    bot: Bot,
    query: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    if let Some(data) = query.data {
        if let Some(data) = data.strip_prefix("tts:") {
            let parts: Vec<&str> = data.split(':').collect();
            if parts.len() == 2 {
                if let Ok(chat_id_i64) = parts[0].parse::<i64>() {
                    let chat_id = ChatId(chat_id_i64);
                    let message_id = parts[1].to_string();

                    let action_flag = Arc::new(Mutex::new(true));
                    start_bots_chat_action(
                        bot.clone(),
                        chat_id,
                        ChatAction::RecordVoice,
                        Arc::clone(&action_flag),
                    )
                    .await;

                    let tts_payload =
                        get_and_remove_tts_payload(app_state.clone(), chat_id, message_id.clone())
                            .await;

                    if let Some(tts_payload) = tts_payload {
                        match simple_tts(tts_payload, app_state.clone()).await {
                            Ok(audio_response) => {
                                let audio_file_path = format!("tmp/{}.mp3", message_id);
                                audio_response.save(&audio_file_path).await?;

                                stop_bots_chat_action(action_flag).await;

                                bot.send_voice(
                                    query.message.unwrap().chat().id,
                                    InputFile::file(audio_file_path.clone()),
                                )
                                .await?;

                                if let Err(e) = remove_file(audio_file_path.clone()) {
                                    info!(
                                        "Could not delete tmp tts file {}: {}",
                                        audio_file_path, e
                                    );
                                }
                            }
                            Err(err) => {
                                error!("TTS generation failed: {}", err);

                                stop_bots_chat_action(action_flag).await;

                                bot.send_message(
                                    query.message.unwrap().chat().id,
                                    "Не удалось озвучить сообщение. Попробуйте позже.",
                                )
                                .await?;
                            }
                        }
                    } else {
                        stop_bots_chat_action(action_flag).await;
                        bot.send_message(
                            query.message.unwrap().chat().id,
                            "Не удалось найти текст для озвучивания. Попробуйте позже.",
                        )
                        .await?;
                    }
                }
            }
        }
    }

    Ok(())
}

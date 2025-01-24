use std::fs::remove_file;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use std::sync::Arc;
use teloxide::macros::BotCommands;
use teloxide::prelude::{CallbackQuery, Message, Requester};
use teloxide::Bot;
use anyhow::Result;
use teloxide::payloads::SendMessageSetters;
use teloxide::types::{InputFile, ReplyParameters};
use tracing::error;
use tracing::log::info;
use crate::user_message_processing::process_user_raw_request;
use core::utils::tg_bot::tg_bot::download_voice;
use core::utils::common::handle_voice_message;
use core::utils::tg_bot::tg_bot::{add_llm_response_to_cache, get_cache_as_string};
use crate::probiot_utils::create_tts_button;
use core::ai::ai::simple_tts;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum ProbiotBotCommands {
    Start,
}

pub(crate) async fn message_handler(bot: Bot, msg: Message, app_state: Arc<BotAppState>) -> Result<()> {
    let chat_id = msg.chat.id;
    let bot_data = bot.get_me().await?;
    let user_raw_request = msg.text().unwrap_or("Empty request").to_string();
    let initiator_app_name = "probiot".to_string();

    // TODO: Удалить прямые отправки сообщений ботом, использовать get_message;
    if msg.chat.is_private() {
        if let Some(voice) = msg.voice() {
            let file_path = match download_voice(&bot, &voice.file.id, &format!("tmp/{}.ogg", voice.file.id)).await {
                Ok(path) => path,
                Err(err) => {
                    error!("Failed to download voice message: {}", err);
                    bot.send_message(chat_id, "Не удалось скачать голосовое сообщение. Попробуйте позже.").await?;
                    return Ok(());
                }
            };

            match handle_voice_message(&file_path).await {
                Ok(Some(user_voice_transcribed)) => {
                    match process_user_raw_request(chat_id, user_voice_transcribed, app_state.clone(), initiator_app_name.clone()).await {
                        Ok(llm_response) => {
                            add_llm_response_to_cache(app_state.clone(), chat_id, llm_response.clone()).await;

                            bot.send_message(chat_id, llm_response)
                                .reply_markup(create_tts_button())
                                .await?;
                            
                            let current_cache = get_cache_as_string(app_state.clone(), chat_id).await;
                            bot.send_message(chat_id, current_cache).await?;
                        }
                        Err(err) => {
                            error!("Error in process_user_raw_request: {}", err);
                            bot.send_message(chat_id, "Произошла ошибка при обработке вашего запроса. Попробуйте ещё раз.").await?;
                        }
                    }
                }
                Ok(None) => {
                    bot.send_message(chat_id, "Не удалось обработать голосовое сообщение. Попробуйте ещё раз.").await?;
                }
                Err(err) => {
                    error!("Error in handle_voice_message: {}", err);
                    bot.send_message(chat_id, "Произошла ошибка при обработке голосового сообщения. Попробуйте ещё раз.").await?;
                }
            }
        } else if let Some(text) = msg.text() {
            match process_user_raw_request(chat_id, text.to_string(), app_state.clone(), initiator_app_name.clone()).await {
                Ok(llm_response) => {
                    add_llm_response_to_cache(app_state.clone(), chat_id, llm_response.clone()).await;

                    bot.send_message(chat_id, llm_response)
                        .reply_markup(create_tts_button())
                        .await?;
                }
                Err(err) => {
                    error!("Error in process_user_raw_request: {}", err);
                    bot.send_message(chat_id, "Произошла ошибка при обработке вашего запроса. Попробуйте ещё раз.").await?;
                }
            }
        } else {
            bot.send_message(chat_id, "Извините, я могу работать только с текстом или голосовыми сообщениями.")
                .await?;
        }
    } else {
        if user_raw_request.contains(&format!("@{}", bot_data.user.clone().username.unwrap_or_default()))
            || (msg.reply_to_message().is_some()
            && msg
            .reply_to_message()
            .and_then(|reply| reply.from.as_ref())
            .map(|user| user.id == bot_data.id)
            .unwrap_or(false))
        {
            bot.send_message(chat_id, "Пожалуйста, напишите мне в приватный чат, так наше общение не помешает другим участникам чата, а я смогу ответить на интересующие вас вопросы.")
                .reply_parameters(ReplyParameters::new(msg.id))
                .await?;
        }
    }

    Ok(())
}

pub(crate) async fn command_handler(
    bot: Bot,
    msg: Message,
    cmd: ProbiotBotCommands,
    _app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = msg.chat.id;

    match cmd {
        ProbiotBotCommands::Start => {
            let bot_msg = get_message(Some("probiot"), "start_message", false).await?;
            bot.send_message(user_id, bot_msg).await?;
        }
    }

    Ok(())
}

pub(crate) async fn callback_query_handler(
    bot: Bot,
    query: CallbackQuery,
    app_state: Arc<BotAppState>
) -> Result<()> {
    if let Some(data) = query.data {
        if data == "tts" {
            if let Some(message) = query.message {
                if let Some(text) = message.regular_message().and_then(|m| m.text()) {
                    let message_id = message.id().to_string();
                    match simple_tts(text.to_string(), app_state).await {
                        Ok(audio_response) => {
                            let audio_file_path = format!("tmp/{}.mp3", message_id);
                            audio_response.save(&audio_file_path).await?;
                            
                            bot.send_voice(message.chat().id, InputFile::file(audio_file_path.clone())).await?;

                            match remove_file(audio_file_path.clone()) {
                                Ok(_) => info!("Tmp tts fn file {} deleted", audio_file_path),
                                Err(e) => info!("Could not delete tmp tts file {}: {}", audio_file_path, e),
                            }
                        }
                        Err(err) => {
                            error!("TTS generation failed: {}", err);
                            bot.send_message(message.chat().id, "Не удалось озвучить сообщение. Попробуйте позже.").await?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

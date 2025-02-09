use anyhow::Result;
use core::ai::common::voice_processing::simple_tts;
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::ProbiotBotMessages;
use core::models::tg_bot::probiot_bot::probiot_bot_commands::ProbiotBotCommands;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::get_and_remove_tts_payload;
use core::utils::tg_bot::tg_bot::{start_bots_chat_action, stop_bots_chat_action};
use std::fs::remove_file;
use std::sync::Arc;
use teloxide::prelude::{CallbackQuery, ChatId, Message, Requester};
use teloxide::types::{ChatAction, InputFile};
use teloxide::Bot;
use tokio::sync::Mutex;
use tracing::error;
use tracing::log::info;

pub(crate) async fn probiot_command_handler(
    bot: Bot,
    msg: Message,
    cmd: ProbiotBotCommands,
    _app_state: Arc<BotAppState>,
) -> Result<()> {
    let chat_id = msg.chat.id;

    match cmd {
        ProbiotBotCommands::Start => {
            let bot_msg = get_message(AppsSystemMessages::Probiot(
                ProbiotBotMessages::StartMessage,
            ))
            .await?;
            bot.send_message(chat_id, bot_msg).await?;
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
                        match simple_tts(&tts_payload, app_state.clone()).await {
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

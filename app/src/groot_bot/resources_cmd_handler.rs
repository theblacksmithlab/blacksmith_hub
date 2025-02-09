use std::fs;
use std::fs::File;
use std::sync::Arc;
use serde_json::Value;
use teloxide::Bot;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Message, Requester};
use teloxide::types::{KeyboardButton, KeyboardMarkup, ReplyMarkup};
use tracing::{error, info};
use core::models::tg_bot::groot_bot::groot_bot::ResourcesDialogState;
use core::models::tg_bot::groot_bot::groot_bot::{ShowType, EditType};
use crate::groot_bot::groot_bot_utils::{load_black_listed_users, load_restricted_words, load_white_listed_users};
use core::state::tg_bot::app_state::BotAppState;
use core::utils::tg_bot::groot_bot::build_resource_file_path;

pub async fn resources_cmd_handler(
    bot: Bot,
    msg: Message,
    state: &mut ResourcesDialogState,
    app_state: Arc<BotAppState>,
    username: &str,
    user_id: u64,
) -> anyhow::Result<()> {
    if state.awaiting_option_choice {
        let keyboard = KeyboardMarkup::new(vec![
            vec![
                KeyboardButton::new("'Белый список' пользователей"),
                KeyboardButton::new("'Чёрный список' пользователей"),
            ],
            vec![KeyboardButton::new("Запрещённые слова/фразы")],
            vec![KeyboardButton::new("Cancel")],
        ]);

        match msg.text().unwrap_or("") {
            "ПОКАЗАТЬ resources" => {
                state.awaiting_option_choice = false;
                state.awaiting_show_type = true;
                bot.send_message(msg.chat.id, "Выберите, что хотите ПРОСМОТРЕТЬ:")
                    .reply_markup(keyboard.clone())
                    .await?;
            }
            "ДОБАВИТЬ resources" => {
                state.awaiting_option_choice = false;
                state.awaiting_edit_type = true;
                bot.send_message(msg.chat.id, "Выберите, что хотите ДОБАВИТЬ:")
                    .reply_markup(keyboard.clone())
                    .await?;
            }
            "Cancel" => {
                state.awaiting_option_choice = false;
                bot.send_message(msg.chat.id, "Операция отменена.")
                    .reply_markup(ReplyMarkup::kb_remove())
                    .await?;
            }
            _ => {
                state.awaiting_option_choice = false;
                bot.send_message(msg.chat.id, "Неверный выбор. Пожалуйста выберите 'ПОКАЗАТЬ', 'ДОБАВИТЬ' или отмените операцию кнопкой 'Cancel'.")
                    .reply_markup(ReplyMarkup::kb_remove())
                    .await?;
            }
        }
    } else if state.awaiting_show_type {
        match msg.text().unwrap_or("") {
            "'Белый список' пользователей" => {
                state.awaiting_show_type = false;
                state.show_type = ShowType::UsersFromWhiteList;
                let white_listed_users_ids = load_white_listed_users(&app_state.app_name);
                
                let data = if white_listed_users_ids.is_empty() {
                    "No data available".to_string()
                } else {
                    white_listed_users_ids
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<String>>()
                        .join("\n")
                };

                if data.len() > 4095 {
                    bot.send_message(msg.chat.id, "Список слишком длинный и не может быть отправлен из-за ограничений Telegram API.")
                        .reply_markup(ReplyMarkup::kb_remove())
                        .await?;
                } else {
                    bot.send_message(
                        msg.chat.id,
                        format!("Текущий 'белый список' пользователей:\n\n{}", data),
                    )
                        .reply_markup(ReplyMarkup::kb_remove())
                        .await?;
                }

                state.show_type = ShowType::None;
            }
            "'Чёрный список' пользователей" => {
                state.awaiting_show_type = false;
                state.show_type = ShowType::UsersFromBlackList;

                let black_listed_users_ids = load_black_listed_users(&app_state.app_name);
                
                let data = if black_listed_users_ids.is_empty() {
                    "No data available".to_string()
                } else {
                    black_listed_users_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
                };

                if data.len() > 4095 {
                    bot.send_message(msg.chat.id, "Список слишком длинный и не может быть отправлен из-за ограничений Telegram API.")
                        .reply_markup(ReplyMarkup::kb_remove())
                        .await?;
                } else {
                    bot.send_message(
                        msg.chat.id,
                        format!("Текущий 'чёрный список' пользователей:\n\n{}", data),
                    )
                        .reply_markup(ReplyMarkup::kb_remove())
                        .await?;
                }

                state.show_type = ShowType::None;
            }
            "Запрещённые слова/фразы" => {
                state.awaiting_show_type = false;
                state.show_type = ShowType::Words;
                
                let restricted_words = load_restricted_words(&app_state.app_name);

                let data = if restricted_words.is_empty() {
                    "No data available".to_string()
                } else {
                    restricted_words.join("\n")
                };

                if data.len() > 4095 {
                    bot.send_message(msg.chat.id, "Список слишком длинный и не может быть отправлен из-за ограничений Telegram API.")
                        .reply_markup(ReplyMarkup::kb_remove())
                        .await?;
                } else {
                    bot.send_message(
                        msg.chat.id,
                        format!("Текущий список запрещённых слов/фраз:\n\n{}", data),
                    )
                        .reply_markup(ReplyMarkup::kb_remove())
                        .await?;
                }

                state.show_type = ShowType::None;
            }
            "Cancel" => {
                state.awaiting_show_type = false;
                bot.send_message(msg.chat.id, "Операция отменена.")
                    .reply_markup(ReplyMarkup::kb_remove())
                    .await?;
            }
            _ => {
                state.awaiting_show_type = false;
                bot.send_message(msg.chat.id, "Неверный выбор. Пожалуйста выберите 'Белый список' пользователей, 'Чёрный список' пользователей, 'Запрещённые слова/фразы' или отмените операцию кнопкой 'Cancel'")
                    .reply_markup(ReplyMarkup::kb_remove())
                    .await?;
            }
        }
    } else if state.awaiting_edit_type {
        match msg.text().unwrap_or("") {
            "'Белый список' пользователей" => {
                let keyboard = KeyboardMarkup::new(vec![vec![KeyboardButton::new("Cancel")]]);
                bot.send_message(
                    msg.chat.id,
                    "Пожалуйста, предоставьте id пользователя для добавления в 'белый список'\n\n\
                ВНИМАНИЕ! id пользователя можно получить с помощью бота: @username_to_id_bot, @userdatailsbot и др.",
                )
                    .reply_markup(keyboard)
                    .await?;

                state.awaiting_edit_type = false;
                state.awaiting_data_entry = true;
                state.edit_type = EditType::UsersToWhiteList;
            }
            "'Чёрный список' пользователей" => {
                let keyboard = KeyboardMarkup::new(vec![vec![KeyboardButton::new("Cancel")]]);
                bot.send_message(
                    msg.chat.id,
                    "Пожалуйста, предоставьте id пользователя для добавления в 'чёрный список'\n\n\
                ВНИМАНИЕ! id пользователя можно получить с помощью бота: @username_to_id_bot",
                )
                    .reply_markup(keyboard)
                    .await?;

                state.awaiting_edit_type = false;
                state.awaiting_data_entry = true;
                state.edit_type = EditType::UsersToBlackList;
            }
            "Запрещённые слова/фразы" => {
                let keyboard = KeyboardMarkup::new(vec![vec![KeyboardButton::new("Cancel")]]);
                bot.send_message(
                    msg.chat.id,
                    "Пожалуйста предоставьте слово/фразу для добавления в список спам-триггеров",
                )
                    .reply_markup(keyboard)
                    .await?;

                state.awaiting_edit_type = false;
                state.awaiting_data_entry = true;
                state.edit_type = EditType::Words;
            }
            "Cancel" => {
                bot.send_message(msg.chat.id, "Операция отменена.")
                    .reply_markup(ReplyMarkup::kb_remove())
                    .await?;

                state.awaiting_edit_type = false;
            }
            _ => {
                bot.send_message(msg.chat.id, "Неверный выбор. Пожалуйста выберите \"'Белый список' пользователей\", \"'Чёрный список' пользователей\",\"Запрещённые слова/фразы\" или отмените операцию кнопкой \"Cancel\"")
                    .await?;

                state.awaiting_edit_type = false;
            }
        }
    } else if state.awaiting_data_entry {
        return match msg.text().unwrap_or("") {
            "Cancel" => {
                state.awaiting_data_entry = false;
                bot.send_message(msg.chat.id, "Операция отменена.")
                    .reply_markup(ReplyMarkup::kb_remove())
                    .await?;
                Ok(())
            }
            _ => {
                state.awaiting_data_entry = false;
                let file_path = match state.edit_type {
                    EditType::UsersToWhiteList => {
                        build_resource_file_path(&app_state.app_name, "white_listed_users.json")
                    }
                    EditType::UsersToBlackList => {
                        build_resource_file_path(&app_state.app_name, "black_listed_users.json")
                    }
                    EditType::Words => build_resource_file_path(&app_state.app_name, "restricted_words.json"),
                    _ => return Ok(()),
                };
                
                let mut data: Vec<Value> = match fs::read_to_string(&file_path) {
                    Ok(content) => serde_json::from_str::<Vec<Value>>(&content).unwrap_or_default(),
                    Err(_) => Vec::new(),
                };

                let data_to_store: Value;

                let input_text = msg.text().unwrap_or("Empty text").to_string();
                
                match state.edit_type {
                    EditType::UsersToWhiteList | EditType::UsersToBlackList => {
                        let cleaned_id = input_text.trim();
                        if let Ok(user_id) = cleaned_id.parse::<i64>() {
                            data_to_store = Value::Number(user_id.into());
                            if !data.contains(&data_to_store) {
                                data.push(data_to_store.clone());
                            } else {
                                bot.send_message(msg.chat.id, "Этот ID уже есть в списке.")
                                    .reply_markup(ReplyMarkup::kb_remove())
                                    .await?;
                                return Ok(());
                            }
                        } else {
                            bot.send_message(msg.chat.id, "Неверный формат ID. Используйте ID, НЕ username.")
                                .reply_markup(ReplyMarkup::kb_remove())
                                .await?;
                            return Ok(());
                        }
                    }
                    EditType::Words => {
                        let lowercase_text = input_text.to_lowercase();
                        data_to_store = Value::String(lowercase_text.clone());
                        if !data.contains(&data_to_store) {
                            data.push(data_to_store.clone());
                        } else {
                            bot.send_message(msg.chat.id, "Эта фраза/слово уже есть в списке.")
                                .reply_markup(ReplyMarkup::kb_remove())
                                .await?;
                            return Ok(());
                        }
                    }
                    _ => return Ok(()),
                }
                
                if let Ok(mut file) = File::create(&file_path) {
                    if let Err(err) = serde_json::to_writer(&mut file, &data) {
                        error!("Error recording data to file: {}: {}", file_path.display(), err);
                        return Err(anyhow::anyhow!("Error saving data."));
                    }
                } else {
                    error!("Error opening file: {}", file_path.display());
                    return Err(anyhow::anyhow!("Ошибка при открытии файла."));
                }
                
                info!(
                "New restriction: | \'{}\' | added to file: | \'{}\' | by user: | \'{}\' | with id: | \'{}\' |",
                input_text, file_path.display(), username, user_id
            );

                bot.send_message(
                    msg.chat.id,
                    "Данные успешно сохранены!",
                )
                    .reply_markup(ReplyMarkup::kb_remove())
                    .await?;
                
                state.edit_type = EditType::None;
                Ok(())
            }
        };
    }
    Ok(())
}
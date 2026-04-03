use anyhow::Result;
use blacksmith_core::models::common::app_name::AppName;
use blacksmith_core::models::common::system_messages::{AppsSystemMessages, StatBotMessages};
use blacksmith_core::state::tg_bot::StatBotState;
use blacksmith_core::utils::common::get_message;
use std::env;
use std::sync::Arc;
use teloxide::prelude::Requester;
use teloxide::types::ChatId;
use teloxide::Bot;
use teloxide_core::payloads::SendMessageSetters;
use teloxide_core::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

pub fn check_admin_access(user_id: u64) -> Vec<AppName> {
    let mut accessible_apps = Vec::new();

    if let Ok(w3a_admins) = env::var("W3A_ADMIN_IDS") {
        let w3a_admin_list: Vec<u64> = w3a_admins
            .split(',')
            .filter_map(|id| id.trim().parse::<u64>().ok())
            .collect();

        if w3a_admin_list.contains(&user_id) {
            accessible_apps.push(AppName::W3AWeb);
        }
    }

    if let Ok(bls_admins) = env::var("BLS_ADMIN_IDS") {
        let bls_admin_list: Vec<u64> = bls_admins
            .split(',')
            .filter_map(|id| id.trim().parse::<u64>().ok())
            .collect();

        if bls_admin_list.contains(&user_id) {
            accessible_apps.push(AppName::BlacksmithWeb);
        }
    }

    accessible_apps
}

pub fn create_stats_keyboard(accessible_apps: &[AppName]) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    for app in accessible_apps {
        let app_label = match app {
            AppName::BlacksmithWeb => "BLS",
            AppName::W3AWeb => "W3A",
            _ => continue,
        };

        let app_code = app.as_str();

        if !keyboard.is_empty() {
            keyboard.push(vec![]);
        }

        keyboard.push(vec![InlineKeyboardButton::callback(
            format!("📊 {}: Неделя", app_label),
            format!("stats:{}:week", app_code),
        )]);
        keyboard.push(vec![InlineKeyboardButton::callback(
            format!("📊 {}: Месяц", app_label),
            format!("stats:{}:month", app_code),
        )]);
        keyboard.push(vec![InlineKeyboardButton::callback(
            format!("📊 {}: Всё время", app_label),
            format!("stats:{}:all", app_code),
        )]);
        keyboard.push(vec![InlineKeyboardButton::callback(
            format!("📅 {}: Кастомный период", app_label),
            format!("custom:{}", app_code),
        )]);
        keyboard.push(vec![InlineKeyboardButton::callback(
            format!("📥 {}: Экспорт запросов", app_label),
            format!("export:{}:requests", app_code),
        )]);
    }

    InlineKeyboardMarkup::new(keyboard)
}

pub async fn send_main_menu(
    bot: &Bot,
    chat_id: ChatId,
    user_id: u64,
    app_state: &Arc<StatBotState>,
    accessible_apps: &[AppName],
) -> Result<()> {
    let mut date_selection = app_state.date_selection.lock().await;
    date_selection.remove(&user_id);
    drop(date_selection);

    let app_names: Vec<String> = accessible_apps
        .iter()
        .map(|app| match app {
            AppName::BlacksmithWeb => "Blacksmith Web".to_string(),
            AppName::W3AWeb => "W3A Web".to_string(),
            _ => String::new(),
        })
        .filter(|s| !s.is_empty())
        .collect();

    let apps = app_names
        .iter()
        .map(|name| format!("• {}", name))
        .collect::<Vec<_>>()
        .join("\n");

    let welcome_message_template =
        get_message(AppsSystemMessages::StatBot(StatBotMessages::MainMenu)).await?;
    let welcome_message = welcome_message_template.replace("{apps}", &apps);

    let keyboard = create_stats_keyboard(accessible_apps);

    bot.send_message(chat_id, welcome_message)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

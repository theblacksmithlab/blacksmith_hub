use core::models::common::app_name::AppName;
use std::env;
use teloxide_core::types::{InlineKeyboardButton, InlineKeyboardMarkup};

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
            format!("📥 {}: Экспорт запросов", app_label),
            format!("export:{}:requests", app_code),
        )]);
    }

    InlineKeyboardMarkup::new(keyboard)
}

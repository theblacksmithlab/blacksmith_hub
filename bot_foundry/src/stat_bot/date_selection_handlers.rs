use crate::stat_bot::calendar_utils::{create_day_selection_keyboard, create_month_selection_keyboard};
use crate::stat_bot::date_validation::{format_validation_error, validate_date_range};
use crate::stat_bot::stat_bot_utils::{check_admin_access, create_stats_keyboard, send_main_menu};
use anyhow::Result;
use chrono::NaiveDate;
use core::local_db::blacksmith_web::statistics::StatisticsPeriod;
use core::models::common::app_name::AppName;
use core::models::tg_bot::stat_bot::{DateSelectionState, SelectionStep};
use core::state::tg_bot::StatBotState;
use std::str::FromStr;
use std::sync::Arc;
use teloxide::payloads::SendMessageSetters;
use teloxide_core::prelude::Requester;
use teloxide_core::types::CallbackQuery;
use teloxide_core::Bot;
use tracing::{error, info};

// Handle: ["custom", app_code]
pub async fn handle_custom_period_start(
    bot: &Bot,
    q: &CallbackQuery,
    app_state: Arc<StatBotState>,
    app_code: &str,
    accessible_apps: &[AppName],
) -> Result<()> {
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let user_id = q.from.id.0;

    let app_name = match AppName::from_str(app_code) {
        Ok(name) => name,
        Err(_) => {
            bot.answer_callback_query(&q.id).await?;
            bot.send_message(chat_id, "❌ Неизвестное приложение")
                .await?;
            return Ok(());
        }
    };

    if !accessible_apps.contains(&app_name) {
        bot.answer_callback_query(&q.id).await?;
        bot.send_message(chat_id, "❌ Нет доступа к этому приложению")
            .await?;
        return Ok(());
    }

    // Initialize date selection state
    let mut date_selection = app_state.date_selection.lock().await;
    date_selection.insert(
        user_id,
        DateSelectionState {
            app_name: app_name.clone(),
            start_date: None,
            step: SelectionStep::SelectingStartMonth,
        },
    );
    drop(date_selection);

    bot.answer_callback_query(&q.id).await?;

    let keyboard = create_month_selection_keyboard(app_code, false);
    bot.send_message(
        chat_id,
        "📅 <b>Выбор кастомного периода</b>\n\n\
         Шаг 1/4: Выберите месяц начальной даты:",
    )
    .parse_mode(teloxide_core::types::ParseMode::Html)
    .reply_markup(keyboard)
    .await?;

    Ok(())
}

// Handle: ["sel_month", selection_type, year_month]
pub async fn handle_month_selection(
    bot: &Bot,
    q: &CallbackQuery,
    app_state: Arc<StatBotState>,
    selection_type: &str,
    year_month: &str,
    app_code: &str,
) -> Result<()> {
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let user_id = q.from.id.0;

    // Parse year-month
    let parts: Vec<&str> = year_month.split('-').collect();
    if parts.len() != 2 {
        bot.answer_callback_query(&q.id).await?;
        bot.send_message(chat_id, "❌ Неверный формат даты").await?;
        return Ok(());
    }

    let year: i32 = parts[0].parse().unwrap_or(0);
    let month: u32 = parts[1].parse().unwrap_or(0);

    if year == 0 || month == 0 || month > 12 {
        bot.answer_callback_query(&q.id).await?;
        bot.send_message(chat_id, "❌ Неверная дата").await?;
        return Ok(());
    }

    // Update state
    let mut date_selection = app_state.date_selection.lock().await;
    if let Some(state) = date_selection.get_mut(&user_id) {
        if selection_type == "start" {
            state.step = SelectionStep::SelectingStartDay { year, month };
        } else {
            state.step = SelectionStep::SelectingEndDay { year, month };
        }
    } else {
        drop(date_selection);
        bot.answer_callback_query(&q.id).await?;
        bot.send_message(
            chat_id,
            "❌ Сессия выбора даты истекла. Начните заново с команды /start",
        )
        .await?;
        return Ok(());
    }
    let start_date = date_selection.get(&user_id).and_then(|s| s.start_date);
    drop(date_selection);

    bot.answer_callback_query(&q.id).await?;

    // Show day selection keyboard
    let for_end_date = selection_type == "end";
    let keyboard = create_day_selection_keyboard(app_code, year, month, for_end_date, start_date);

    let step_text = if for_end_date {
        "Шаг 3/4: Выберите день конечной даты:"
    } else {
        "Шаг 2/4: Выберите день начальной даты:"
    };

    bot.send_message(
        chat_id,
        format!("📅 <b>Выбор кастомного периода</b>\n\n{}", step_text),
    )
    .parse_mode(teloxide_core::types::ParseMode::Html)
    .reply_markup(keyboard)
    .await?;

    Ok(())
}

// Handle: ["sel_day", selection_type, date]
pub async fn handle_day_selection(
    bot: &Bot,
    q: &CallbackQuery,
    app_state: Arc<StatBotState>,
    selection_type: &str,
    date_str: &str,
    app_code: &str,
) -> Result<()> {
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let user_id = q.from.id.0;

    // Parse date (format: YYYY-MM-DD)
    let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to parse date {}: {}", date_str, e);
            bot.answer_callback_query(&q.id).await?;
            bot.send_message(chat_id, "❌ Неверный формат даты").await?;
            return Ok(());
        }
    };

    if selection_type == "start" {
        // Save start_date and proceed to end_date selection
        let mut date_selection = app_state.date_selection.lock().await;
        if let Some(state) = date_selection.get_mut(&user_id) {
            state.start_date = Some(date);
            state.step = SelectionStep::SelectingEndMonth;
        } else {
            drop(date_selection);
            bot.answer_callback_query(&q.id).await?;
            bot.send_message(
                chat_id,
                "❌ Сессия выбора даты истекла. Начните заново с команды /start",
            )
            .await?;
            return Ok(());
        }
        drop(date_selection);

        bot.answer_callback_query(&q.id).await?;

        // Show month selection for end_date
        let keyboard = create_month_selection_keyboard(app_code, true);
        bot.send_message(
            chat_id,
            format!(
                "✅ Начальная дата: <b>{}</b>\n\n\
                 📅 <b>Выбор кастомного периода</b>\n\n\
                 Шаг 3/4: Выберите месяц конечной даты:",
                date.format("%d.%m.%Y")
            ),
        )
        .parse_mode(teloxide_core::types::ParseMode::Html)
        .reply_markup(keyboard)
        .await?;
    } else {
        // end_date selected - validate and show statistics
        let mut date_selection = app_state.date_selection.lock().await;
        let state = match date_selection.get(&user_id) {
            Some(s) => s.clone(),
            None => {
                drop(date_selection);
                bot.answer_callback_query(&q.id).await?;
                bot.send_message(
                    chat_id,
                    "❌ Сессия выбора даты истекла. Начните заново с команды /start",
                )
                .await?;
                return Ok(());
            }
        };

        let start_date = match state.start_date {
            Some(d) => d,
            None => {
                drop(date_selection);
                bot.answer_callback_query(&q.id).await?;
                bot.send_message(
                    chat_id,
                    "❌ Начальная дата не выбрана. Начните заново с команды /start",
                )
                .await?;
                return Ok(());
            }
        };

        // Validate date range
        if let Err(e) = validate_date_range(start_date, date) {
            drop(date_selection);
            bot.answer_callback_query(&q.id).await?;
            let error_msg = format_validation_error(&e.to_string());
            bot.send_message(chat_id, error_msg).await?;

            // Clear state and show main menu
            let mut date_selection = app_state.date_selection.lock().await;
            date_selection.remove(&user_id);
            drop(date_selection);

            let accessible_apps = check_admin_access(user_id);
            let keyboard = create_stats_keyboard(&accessible_apps);
            bot.send_message(chat_id, "Выберите другой период или используйте фиксированные периоды:")
                .reply_markup(keyboard)
                .await?;
            return Ok(());
        }

        // Clear selection state before processing stats
        date_selection.remove(&user_id);
        drop(date_selection);

        bot.answer_callback_query(&q.id).await?;

        info!(
            "User {} selected custom period: {} to {}",
            user_id,
            start_date.format("%Y-%m-%d"),
            date.format("%Y-%m-%d")
        );

        // Create CustomRange period and call stats handler
        let custom_period = StatisticsPeriod::CustomRange {
            start: start_date.format("%Y-%m-%d").to_string(),
            end: date.format("%Y-%m-%d").to_string(),
        };

        // Reuse existing handle_stats_request but pass custom period
        handle_stats_request_with_period(bot, q, app_state, app_code, custom_period).await?;
    }

    Ok(())
}

// Handle: ["back_to_main"]
pub async fn handle_back_to_main(
    bot: &Bot,
    q: &CallbackQuery,
    app_state: Arc<StatBotState>,
    accessible_apps: &[AppName],
) -> Result<()> {
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let user_id = q.from.id.0;

    bot.answer_callback_query(&q.id).await?;

    // Use shared function to show main menu and clear state
    send_main_menu(bot, chat_id, user_id, &app_state, accessible_apps).await?;

    Ok(())
}

// Handle: ["back_to_months", selection_type]
pub async fn handle_back_to_months(
    bot: &Bot,
    q: &CallbackQuery,
    app_state: Arc<StatBotState>,
    selection_type: &str,
) -> Result<()> {
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let user_id = q.from.id.0;

    // Get app_code from state
    let date_selection = app_state.date_selection.lock().await;
    let app_code = match date_selection.get(&user_id) {
        Some(state) => state.app_name.as_str().to_string(),
        None => {
            drop(date_selection);
            bot.answer_callback_query(&q.id).await?;
            bot.send_message(
                chat_id,
                "❌ Сессия выбора даты истекла. Начните заново с команды /start",
            )
            .await?;
            return Ok(());
        }
    };
    drop(date_selection);

    // Update state back to month selection
    let mut date_selection = app_state.date_selection.lock().await;
    if let Some(state) = date_selection.get_mut(&user_id) {
        if selection_type == "start" {
            state.step = SelectionStep::SelectingStartMonth;
        } else {
            state.step = SelectionStep::SelectingEndMonth;
        }
    }
    drop(date_selection);

    bot.answer_callback_query(&q.id).await?;

    // Show month selection keyboard
    let for_end_date = selection_type == "end";
    let keyboard = create_month_selection_keyboard(&app_code, for_end_date);

    let step_text = if for_end_date {
        "Шаг 3/4: Выберите месяц конечной даты:"
    } else {
        "Шаг 1/4: Выберите месяц начальной даты:"
    };

    bot.send_message(
        chat_id,
        format!("📅 <b>Выбор кастомного периода</b>\n\n{}", step_text),
    )
    .parse_mode(teloxide_core::types::ParseMode::Html)
    .reply_markup(keyboard)
    .await?;

    Ok(())
}

// Helper to call handle_stats_request with custom period
async fn handle_stats_request_with_period(
    bot: &Bot,
    q: &CallbackQuery,
    app_state: Arc<StatBotState>,
    app_code: &str,
    period: StatisticsPeriod,
) -> Result<()> {
    let chat_id = q.message.as_ref().unwrap().chat().id;
    let user_id = q.from.id.0;

    let app_name = match AppName::from_str(app_code) {
        Ok(name) => name,
        Err(_) => {
            bot.send_message(chat_id, "❌ Неизвестное приложение")
                .await?;
            return Ok(());
        }
    };

    let accessible_apps = check_admin_access(user_id);
    if !accessible_apps.contains(&app_name) {
        bot.send_message(chat_id, "❌ Нет доступа к этому приложению")
            .await?;
        return Ok(());
    }

    bot.send_message(chat_id, "⏳ Загружаю статистику...")
        .await?;

    let db_pool = app_state
        .core
        .db_pool
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Database pool not initialized"))?;

    match core::local_db::blacksmith_web::statistics::get_statistics_for_period(
        db_pool,
        app_name.as_str(),
        &period,
    )
    .await
    {
        Ok((user_stats, request_stats)) => {
            let period_name = match &period {
                StatisticsPeriod::CustomRange { start, end } => {
                    format!("с {} по {}", start, end)
                }
                _ => "кастомный период".to_string(),
            };

            let app_display_name = match app_name {
                AppName::BlacksmithWeb => "Blacksmith Web",
                AppName::W3AWeb => "W3A Web",
                _ => app_name.as_str(),
            };

            let upper_divider = "============================================\n\n".to_string();
            let lower_divider = "\n\n============================================".to_string();
            let result_footer = "\n\n<i>Для получения обновлённой статистики перейдите в основное меню, выполнив команду /start</i>".to_string();

            let response_template = format!(
                "📊 <b>Статистика {}</b>\n\
                 📅 Период: {}\n\n\
                 👥 Уникальные пользователи: {}\n\
                 💬 Всего запросов: {}\n\n\
                 <i>Данные обновляются в реальном времени</i>",
                app_display_name, period_name, user_stats.unique_users, request_stats.requests
            );

            let response = format!("{}{}{}{}", upper_divider, response_template, lower_divider, result_footer);

            bot.send_message(chat_id, response)
                .parse_mode(teloxide_core::types::ParseMode::Html)
                .await?;
        }
        Err(e) => {
            error!("Failed to get statistics: {}", e);
            bot.send_message(
                chat_id,
                "❌ Ошибка при получении статистики. Попробуйте позже.",
            )
            .await?;
        }
    }

    Ok(())
}

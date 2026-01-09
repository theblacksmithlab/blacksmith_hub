use chrono::Utc;
use core::local_db::blacksmith_web::statistics::{
    export_user_requests_to_csv, get_statistics_for_period, StatisticsPeriod,
};
use core::models::common::app_name::AppName;
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::CommonMessages;
use core::state::tg_bot::StatBotState;
use core::utils::common::get_message;
use std::fs;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use teloxide_core::payloads::SendMessageSetters;
use teloxide_core::requests::Requester;
use teloxide_core::types::ParseMode::Html;
use teloxide_core::types::{CallbackQuery, InputFile};
use teloxide_core::Bot;
use tokio::time::sleep;
use tracing::{error, info};

pub async fn handle_stats_request(
    bot: &Bot,
    q: &CallbackQuery,
    app_state: Arc<StatBotState>,
    app_code: &str,
    period: &str,
    accessible_apps: &[AppName],
) -> anyhow::Result<()> {
    let chat_id = q.message.as_ref().unwrap().chat().id;

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

        let bot_system_message = get_message(AppsSystemMessages::Common(
            CommonMessages::NoRightsForAppUse,
        ))
        .await?;

        bot.send_message(chat_id, bot_system_message).await?;

        return Ok(());
    }

    let stats_period = match period {
        "week" => StatisticsPeriod::LastWeek,
        "month" => StatisticsPeriod::LastMonth,
        "all" => StatisticsPeriod::AllTime,
        _ => {
            bot.answer_callback_query(&q.id).await?;
            bot.send_message(chat_id, "❌ Неизвестный период").await?;
            return Ok(());
        }
    };

    bot.send_message(chat_id, "⏳ Загружаю статистику...")
        .await?;

    sleep(Duration::from_secs(2)).await;

    bot.answer_callback_query(&q.id).await?;

    let db_pool = app_state
        .core
        .db_pool
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Database pool not initialized"))?;

    match get_statistics_for_period(db_pool, app_name.as_str(), stats_period).await {
        Ok((user_stats, request_stats)) => {
            let period_name = match stats_period {
                StatisticsPeriod::LastWeek => "за последнюю неделю",
                StatisticsPeriod::LastMonth => "за последний месяц",
                StatisticsPeriod::AllTime => "за всё время (90 дней)",
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

            bot.send_message(chat_id, response).parse_mode(Html).await?;
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

pub async fn handle_export_requests(
    bot: &Bot,
    q: &CallbackQuery,
    app_state: Arc<StatBotState>,
    app_code: &str,
    accessible_apps: &[AppName],
) -> anyhow::Result<()> {
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

        let bot_system_message = get_message(AppsSystemMessages::Common(
            CommonMessages::NoRightsForAppUse,
        ))
        .await?;

        bot.send_message(chat_id, bot_system_message).await?;

        return Ok(());
    }

    bot.send_message(chat_id, "⏳ Формирую CSV файл...").await?;

    sleep(Duration::from_secs(3)).await;

    bot.answer_callback_query(&q.id).await?;

    let db_pool = app_state
        .core
        .db_pool
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Database pool not initialized"))?;

    let tmp_dir = "tmp/stat_bot/exports";
    if let Err(e) = fs::create_dir_all(tmp_dir) {
        error!("Failed to create tmp directory: {}", e);
        bot.send_message(chat_id, "❌ Ошибка создания временной директории")
            .await?;
        return Ok(());
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let file_name = format!("{}_{}_requests_{}.csv", app_code, user_id, timestamp);
    let file_path = format!("{}/{}", tmp_dir, file_name);

    match export_user_requests_to_csv(db_pool, app_name.as_str(), &file_path).await {
        Ok(_) => {
            let app_display_name = match app_name {
                AppName::BlacksmithWeb => "Blacksmith Web",
                AppName::W3AWeb => "W3A Web",
                _ => app_name.as_str(),
            };

            let upper_divider = "============================================\n\n".to_string();
            let lower_divider = "\n\n============================================".to_string();
            let description_footer = "\n\n<i>Для получения обновлённой выгрузки перейдите в основное меню, выполнив команду /start</i>".to_string();

            let description_template = format!(
                "📥 Экспорт запросов пользователей\n\n\
                📊 Приложение: {}\n\n\
                📅 Дата: {}",
                app_display_name,
                Utc::now().format("%d.%m.%Y %H:%M")
            );

            let description = format!("{}{}{}{}", upper_divider, description_template, lower_divider, description_footer);

            bot.send_message(chat_id, description).parse_mode(Html).await?;

            match bot
                .send_document(chat_id, InputFile::file(&file_path))
                .await
            {
                Ok(_) => {
                    info!("Successfully sent CSV export to user {}", user_id);
                }
                Err(e) => {
                    error!("Failed to send document: {}", e);
                    bot.send_message(chat_id, "❌ Ошибка при отправке файла")
                        .await?;
                }
            }

            if let Err(e) = fs::remove_file(&file_path) {
                error!("Could not delete tmp CSV file {}: {}", file_path, e);
            } else {
                info!("Cleaned up tmp CSV file: {}", file_path);
            }
        }
        Err(e) => {
            error!("Failed to export CSV: {}", e);
            bot.send_message(chat_id, "❌ Ошибка при формировании CSV. Попробуйте позже.")
                .await?;
        }
    }

    Ok(())
}

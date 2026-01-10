use chrono::{Datelike, Months, NaiveDate, Utc};
use teloxide_core::types::{InlineKeyboardButton, InlineKeyboardMarkup};

const MAX_DATA_AGE_DAYS: i64 = 90;

// Format month name in Russian
pub fn format_month_name(month: u32) -> &'static str {
    match month {
        1 => "Январь",
        2 => "Февраль",
        3 => "Март",
        4 => "Апрель",
        5 => "Май",
        6 => "Июнь",
        7 => "Июль",
        8 => "Август",
        9 => "Сентябрь",
        10 => "Октябрь",
        11 => "Ноябрь",
        12 => "Декабрь",
        _ => "???",
    }
}

// Check if a date is available (not in future, not older than 90 days, and not before optional minimum date)
pub fn is_date_available(date: NaiveDate, unavailable_before: Option<NaiveDate>) -> bool {
    let now = Utc::now().naive_utc().date();
    let min_date = now - chrono::Duration::days(MAX_DATA_AGE_DAYS);

    date <= now
        && date >= min_date
        && unavailable_before.map_or(true, |before| date >= before)
}

// Generate keyboard with available months (current + 3 months back to cover 90 days)
pub fn create_month_selection_keyboard(
    app_code: &str,
    for_end_date: bool,
) -> InlineKeyboardMarkup {
    let now = Utc::now().naive_utc().date();
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    // Show current month + 3 months back (total 4 months) to fully cover 90 days
    for months_ago in 0..4 {
        let target_date = if months_ago == 0 {
            now
        } else {
            now.checked_sub_months(Months::new(months_ago as u32))
                .unwrap_or(now)
        };
        let year = target_date.year();
        let month = target_date.month();

        let month_name = format_month_name(month);
        let label = format!("{} {}", month_name, year);

        let callback_type = if for_end_date { "end" } else { "start" };
        let callback_data = format!("sel_month:{}:{}:{}-{:02}", callback_type, app_code, year, month);

        keyboard.push(vec![InlineKeyboardButton::callback(label, callback_data)]);
    }

    // Add "Back to main menu" button
    keyboard.push(vec![InlineKeyboardButton::callback(
        "◀️ Назад в Главное меню",
        "back_to_main".to_string(),
    )]);

    InlineKeyboardMarkup::new(keyboard)
}

// Generate calendar grid for a specific month
pub fn create_day_selection_keyboard(
    app_code: &str,
    year: i32,
    month: u32,
    for_end_date: bool,
    unavailable_before: Option<NaiveDate>,
) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    // Header: "Month Year"
    let month_name = format_month_name(month);
    let header = format!("📅 {} {}", month_name, year);
    keyboard.push(vec![InlineKeyboardButton::callback(header, "ignore".to_string())]);

    // Weekday headers
    let weekdays = vec!["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"];
    let weekday_row: Vec<InlineKeyboardButton> = weekdays
        .iter()
        .map(|day| InlineKeyboardButton::callback(*day, "ignore".to_string()))
        .collect();
    keyboard.push(weekday_row);

    // Get first day of the month
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid date");
    let first_weekday = first_day.weekday().num_days_from_monday() as usize;

    // Get number of days in month
    let days_in_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .map(|d| d - chrono::Duration::days(1))
    .and_then(|d| Some(d.day()))
    .unwrap_or(30);

    // Build calendar grid
    let mut week_row: Vec<InlineKeyboardButton> = vec![];

    // Empty cells before first day
    for _ in 0..first_weekday {
        week_row.push(InlineKeyboardButton::callback(" ", "ignore".to_string()));
    }

    // Days of the month
    for day in 1..=days_in_month {
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("Invalid date");

        if is_date_available(date, unavailable_before) {
            let callback_type = if for_end_date { "end" } else { "start" };
            let callback_data = format!(
                "sel_day:{}:{}:{}-{:02}-{:02}",
                callback_type, app_code, year, month, day
            );
            week_row.push(InlineKeyboardButton::callback(
                day.to_string(),
                callback_data,
            ));
        } else {
            // Unavailable date - show empty cell to maintain grid structure
            week_row.push(InlineKeyboardButton::callback(" ", "ignore".to_string()));
        }

        // End of week - push row and start new one
        if week_row.len() == 7 {
            keyboard.push(week_row.clone());
            week_row.clear();
        }
    }

    // Fill remaining cells in last week
    while !week_row.is_empty() && week_row.len() < 7 {
        week_row.push(InlineKeyboardButton::callback(" ", "ignore".to_string()));
    }
    if !week_row.is_empty() {
        keyboard.push(week_row);
    }

    // Add "Back to month selection" button
    let callback_type = if for_end_date { "end" } else { "start" };
    keyboard.push(vec![InlineKeyboardButton::callback(
        "◀️ К выбору месяца",
        format!("back_to_months:{}", callback_type),
    )]);

    InlineKeyboardMarkup::new(keyboard)
}

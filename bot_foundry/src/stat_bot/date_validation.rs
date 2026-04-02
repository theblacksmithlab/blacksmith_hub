use anyhow::{bail, Result};
use chrono::NaiveDate;
use blacksmith_core::utils::moscow_time::moscow_today;

const MAX_PERIOD_DAYS: i64 = 90;

// Validate date range for custom period
pub fn validate_date_range(start: NaiveDate, end: NaiveDate) -> Result<()> {
    // 1. end >= start
    if end < start {
        bail!("Конечная дата не может быть раньше начальной");
    }

    // 2. Minimum 1 day (already covered by end >= start)

    // 3. Maximum 90 days
    let diff = (end - start).num_days();
    if diff > MAX_PERIOD_DAYS {
        bail!("Период не может превышать 90 дней");
    }

    // 4. end not in future
    let today = moscow_today();
    if end > today {
        bail!("Конечная дата не может быть в будущем");
    }

    Ok(())
}

// Format validation error message for user
pub fn format_validation_error(error: &str) -> String {
    format!(
        "❌ Ошибка: {}\n\nПожалуйста, выберите другой период.",
        error
    )
}

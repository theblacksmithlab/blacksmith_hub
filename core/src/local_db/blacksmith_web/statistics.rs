use chrono::{Duration, Utc};
use csv::WriterBuilder;
use sqlx::{Error, SqlitePool};
use std::fs::File;
use std::io::Write;
use tracing::{error, info};

const DAYS_IN_WEEK: i64 = 7;
const DAYS_IN_MONTH: i64 = 30;

#[derive(Debug)]
pub struct UserStatistics {
    pub period: StatisticsPeriod,
    pub unique_users: i64,
    pub app_name: String,
}

#[derive(Debug)]
pub struct RequestStatistics {
    pub period: StatisticsPeriod,
    pub requests: i64,
    pub app_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatisticsPeriod {
    LastWeek,
    LastMonth,
    AllTime,
    CustomRange { start: String, end: String },
}

impl StatisticsPeriod {
    fn get_date_range(&self) -> (Option<String>, Option<String>) {
        match self {
            Self::LastWeek => {
                let start = Utc::now() - Duration::days(DAYS_IN_WEEK);
                (Some(start.to_rfc3339()), None)
            }
            Self::LastMonth => {
                let start = Utc::now() - Duration::days(DAYS_IN_MONTH);
                (Some(start.to_rfc3339()), None)
            }
            Self::AllTime => (None, None),
            Self::CustomRange { start, end } => (Some(start.clone()), Some(end.clone())),
        }
    }
}

pub async fn get_unique_users_count(
    pool: &SqlitePool,
    app_name: &str,
    period: &StatisticsPeriod,
) -> Result<UserStatistics, Error> {
    let (start_date, end_date) = period.get_date_range();

    let count = match (start_date, end_date) {
        (Some(start), Some(end)) => {
            let result: (i64,) = sqlx::query_as(
                "SELECT COUNT(DISTINCT user_id)
                 FROM chat_messages
                 WHERE app_name = ?
                 AND datetime(created_at) >= datetime(?)
                 AND datetime(created_at) <= datetime(?)",
            )
            .bind(app_name)
            .bind(start)
            .bind(end)
            .fetch_one(pool)
            .await?;
            result.0
        }
        (Some(start), None) => {
            let result: (i64,) = sqlx::query_as(
                "SELECT COUNT(DISTINCT user_id)
                 FROM chat_messages
                 WHERE app_name = ?
                 AND datetime(created_at) >= datetime(?)",
            )
            .bind(app_name)
            .bind(start)
            .fetch_one(pool)
            .await?;
            result.0
        }
        (None, None) => {
            let result: (i64,) = sqlx::query_as(
                "SELECT COUNT(DISTINCT user_id)
                 FROM chat_messages
                 WHERE app_name = ?",
            )
            .bind(app_name)
            .fetch_one(pool)
            .await?;
            result.0
        }
        (None, Some(_)) => {
            unreachable!("Invalid date range: end without start")
        }
    };

    Ok(UserStatistics {
        period: period.clone(),
        unique_users: count,
        app_name: app_name.to_string(),
    })
}

pub async fn get_request_count(
    pool: &SqlitePool,
    app_name: &str,
    period: &StatisticsPeriod,
) -> Result<RequestStatistics, Error> {
    let (start_date, end_date) = period.get_date_range();

    let count = match (start_date, end_date) {
        (Some(start), Some(end)) => {
            let result: (i64,) = sqlx::query_as(
                "SELECT COUNT(*)
                 FROM chat_messages
                 WHERE app_name = ?
                 AND sender = 'user'
                 AND datetime(created_at) >= datetime(?)
                 AND datetime(created_at) <= datetime(?)",
            )
            .bind(app_name)
            .bind(start)
            .bind(end)
            .fetch_one(pool)
            .await?;
            result.0
        }
        (Some(start), None) => {
            let result: (i64,) = sqlx::query_as(
                "SELECT COUNT(*)
                 FROM chat_messages
                 WHERE app_name = ?
                 AND sender = 'user'
                 AND datetime(created_at) >= datetime(?)",
            )
            .bind(app_name)
            .bind(start)
            .fetch_one(pool)
            .await?;
            result.0
        }
        (None, None) => {
            let result: (i64,) = sqlx::query_as(
                "SELECT COUNT(*)
                 FROM chat_messages
                 WHERE app_name = ?
                 AND sender = 'user'",
            )
            .bind(app_name)
            .fetch_one(pool)
            .await?;
            result.0
        }
        (None, Some(_)) => {
            unreachable!("Invalid date range: end without start")
        }
    };

    Ok(RequestStatistics {
        period: period.clone(),
        requests: count,
        app_name: app_name.to_string(),
    })
}

pub async fn get_statistics_for_period(
    pool: &SqlitePool,
    app_name: &str,
    period: &StatisticsPeriod,
) -> Result<(UserStatistics, RequestStatistics), Error> {
    let user_stats = get_unique_users_count(pool, app_name, period).await?;
    let request_stats = get_request_count(pool, app_name, period).await?;

    Ok((user_stats, request_stats))
}

pub async fn export_user_requests_to_csv(
    pool: &SqlitePool,
    app_name: &str,
    output_path: &str,
) -> Result<(), anyhow::Error> {
    info!("Starting CSV export for app: {}", app_name);

    let messages = sqlx::query_as::<_, (i64, String, String, String)>(
        "SELECT id, user_id, message, created_at
         FROM chat_messages
         WHERE app_name = ? AND sender = 'user'
         ORDER BY created_at ASC",
    )
    .bind(app_name)
    .fetch_all(pool)
    .await?;

    info!("Found {} user requests for export", messages.len());

    let mut file = match File::create(output_path) {
        Ok(f) => {
            info!("Created CSV file: {}", output_path);
            f
        }
        Err(e) => {
            error!("Failed to create CSV file {}: {}", output_path, e);
            return Err(e.into());
        }
    };

    if let Err(e) = file.write_all(b"\xEF\xBB\xBF") {
        error!("Failed to write BOM to CSV: {}", e);
        return Err(e.into());
    }

    let mut wtr = WriterBuilder::new().from_writer(file);

    if let Err(e) = wtr.write_record(&["message_id", "user_id", "message_text", "created_at"]) {
        error!("Failed to write CSV headers: {}", e);
        return Err(e.into());
    }

    for (id, user_id, message, created_at) in messages {
        if let Err(e) = wtr.write_record(&[&id.to_string(), &user_id, &message, &created_at]) {
            error!(
                "Failed to write CSV record: {}. Continuing with next record.",
                e
            );
            continue;
        }
    }

    if let Err(e) = wtr.flush() {
        error!("Failed to flush CSV file: {}. File may be incomplete.", e);
        return Err(e.into());
    }

    info!("CSV export completed: {}", output_path);
    Ok(())
}

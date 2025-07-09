use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite};
use tracing::{error, info};

pub async fn check_chat_payment(db_pool: &Pool<Sqlite>, chat_id: i64) -> anyhow::Result<bool> {
    let query = "
        SELECT end_date
        FROM subscriptions
        WHERE chat_id = ?
        ORDER BY end_date DESC
        LIMIT 1
    ";

    match sqlx::query_as::<_, (String,)>(query)
        .bind(chat_id)
        .fetch_optional(db_pool)
        .await
    {
        Ok(Some((end_date_str,))) => match DateTime::parse_from_rfc3339(&end_date_str) {
            Ok(end_date) => {
                let now = Utc::now();
                let is_valid = end_date.with_timezone(&Utc) > now;

                if is_valid {
                    info!("Chat {} has valid subscription until {}", chat_id, end_date);
                } else {
                    info!("Chat {} subscription expired on {}", chat_id, end_date);
                }

                Ok(is_valid)
            }
            Err(e) => {
                error!("Failed to parse end_date for chat {}: {}", chat_id, e);
                Ok(false)
            }
        },
        Ok(None) => {
            info!("No subscription found for chat {}", chat_id);
            Ok(false)
        }
        Err(e) => {
            error!(
                "Database error checking payment for chat {}: {}",
                chat_id, e
            );
            Ok(false)
        }
    }
}

pub async fn create_subscription(
    db_pool: &Pool<Sqlite>,
    chat_id: i64,
    chat_username: &str,
    paid_by_user_id: i64,
    paid_by_username: Option<&str>,
    plan_type: &str,
) -> anyhow::Result<()> {
    let now = Utc::now();
    let end_date = match plan_type {
        "monthly" => now + chrono::Duration::days(30),
        "yearly" => now + chrono::Duration::days(365),
        _ => return Err(anyhow::anyhow!("Invalid plan type: {}", plan_type)),
    };

    let upsert_query = "
        INSERT INTO subscriptions
        (chat_id, chat_username, paid_by_user_id, paid_by_username, start_date, end_date, plan_type)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(chat_id) DO UPDATE SET
            chat_username = excluded.chat_username,
            paid_by_user_id = excluded.paid_by_user_id,
            paid_by_username = excluded.paid_by_username,
            start_date = excluded.start_date,
            end_date = excluded.end_date,
            plan_type = excluded.plan_type,
            created_at = CURRENT_TIMESTAMP
    ";

    sqlx::query(upsert_query)
        .bind(chat_id)
        .bind(chat_username)
        .bind(paid_by_user_id)
        .bind(paid_by_username)
        .bind(now.to_rfc3339())
        .bind(end_date.to_rfc3339())
        .bind(plan_type)
        .execute(db_pool)
        .await?;

    info!(
        "Created/updated {} subscription for chat {} (paid by user {})",
        plan_type, chat_id, paid_by_user_id
    );

    Ok(())
}

pub async fn get_subscription_info(
    db_pool: &Pool<Sqlite>,
    chat_id: i64,
) -> anyhow::Result<Option<SubscriptionInfo>> {
    let query = "
        SELECT chat_username, paid_by_user_id, paid_by_username, start_date, end_date, plan_type
        FROM subscriptions
        WHERE chat_id = ?
        ORDER BY end_date DESC
        LIMIT 1
    ";

    match sqlx::query_as::<_, (String, i64, Option<String>, String, String, String)>(query)
        .bind(chat_id)
        .fetch_optional(db_pool)
        .await
    {
        Ok(Some((
                    chat_username,
                    paid_by_user_id,
                    paid_by_username,
                    start_date,
                    end_date,
                    plan_type,
                ))) => Ok(Some(SubscriptionInfo {
            chat_username,
            paid_by_user_id,
            paid_by_username,
            start_date,
            end_date,
            plan_type,
        })),
        Ok(None) => Ok(None),
        Err(e) => {
            error!(
                "Error getting subscription info for chat {}: {}",
                chat_id, e
            );
            Err(e.into())
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    pub chat_username: String,
    pub paid_by_user_id: i64,
    pub paid_by_username: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub plan_type: String,
}

pub async fn get_expiring_subscriptions(
    db_pool: &Pool<Sqlite>,
    days_before: i64,
) -> anyhow::Result<Vec<SubscriptionInfo>> {
    let target_date = Utc::now() + chrono::Duration::days(days_before);

    let query = "
        SELECT chat_id, chat_username, paid_by_user_id, paid_by_username, start_date, end_date, plan_type
        FROM subscriptions
        WHERE datetime(end_date) <= datetime(?)
        AND datetime(end_date) > datetime('now')
        ORDER BY end_date ASC
    ";

    let rows =
        sqlx::query_as::<_, (i64, String, i64, Option<String>, String, String, String)>(query)
            .bind(target_date.to_rfc3339())
            .fetch_all(db_pool)
            .await?;

    let subscriptions = rows
        .into_iter()
        .map(
            |(
                 _,
                 chat_username,
                 paid_by_user_id,
                 paid_by_username,
                 start_date,
                 end_date,
                 plan_type,
             )| {
                SubscriptionInfo {
                    chat_username,
                    paid_by_user_id,
                    paid_by_username,
                    start_date,
                    end_date,
                    plan_type,
                }
            },
        )
        .collect();

    Ok(subscriptions)
}
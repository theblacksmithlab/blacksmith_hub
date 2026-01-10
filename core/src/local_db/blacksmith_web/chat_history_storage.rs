use crate::models::blacksmith_web::blacksmith_web::ChatMessage;
use crate::utils::moscow_time::moscow_now;
use sqlx::{Error, SqlitePool};
use tracing::info;

pub async fn fetch_chat_history_from_db(
    pool: &SqlitePool,
    user_id: &str,
    app_name: &str,
    limit: Option<usize>,
) -> Result<Vec<ChatMessage>, Error> {
    let messages = if let Some(limit_value) = limit {
        sqlx::query_as::<_, ChatMessage>(
            "SELECT * FROM (
                SELECT id, user_id, sender, message, app_name, created_at
                FROM chat_messages
                WHERE user_id = ? AND app_name = ?
                ORDER BY id DESC
                LIMIT ?
            ) ORDER BY id ASC",
        )
        .bind(user_id)
        .bind(app_name)
        .bind(limit_value as i64)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ChatMessage>(
            "SELECT id, user_id, sender, message, app_name, created_at
             FROM chat_messages
             WHERE user_id = ? AND app_name = ?
             ORDER BY id ASC",
        )
        .bind(user_id)
        .bind(app_name)
        .fetch_all(pool)
        .await?
    };

    Ok(messages)
}

pub async fn save_message_to_db(
    pool: &SqlitePool,
    user_id: &str,
    sender: &str,
    message: &str,
    app_name: &str,
) -> Result<(), Error> {
    let now = moscow_now().to_rfc3339();

    sqlx::query(
        "INSERT INTO chat_messages (user_id, sender, message, app_name, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(sender)
    .bind(message)
    .bind(app_name)
    .bind(now)
    .execute(pool)
    .await?;

    cleanup_old_messages(pool, app_name).await?;

    Ok(())
}

pub async fn cleanup_old_messages(pool: &SqlitePool, app_name: &str) -> Result<(), Error> {
    let result = sqlx::query(
        "DELETE FROM chat_messages
         WHERE app_name = ?
         AND datetime(created_at) < datetime('now', '-90 days')",
    )
    .bind(app_name)
    .execute(pool)
    .await?;

    let deleted_count = result.rows_affected();
    if deleted_count > 0 {
        info!(
            "Cleaned up {} messages older than 90 days for app_name={}",
            deleted_count, app_name
        );
    }

    Ok(())
}

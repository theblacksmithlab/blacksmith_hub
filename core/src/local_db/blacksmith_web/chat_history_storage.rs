use sqlx::{Error, SqlitePool};
use tracing::info;
use crate::models::blacksmith_web::blacksmith_web::ChatMessage;

pub async fn fetch_chat_history_from_db(
    pool: &SqlitePool,
    user_id: &str,
    app_name: &str,
    limit: Option<usize>,
) -> Result<Vec<ChatMessage>, Error> {
    let messages = if let Some(limit_value) = limit {
        sqlx::query_as::<_, ChatMessage>(
            "SELECT * FROM (
                SELECT id, user_id, sender, message, app_name
                FROM chat_messages
                WHERE user_id = ? AND app_name = ?
                ORDER BY id DESC
                LIMIT ?
            ) ORDER BY id ASC"
        )
        .bind(user_id)
        .bind(app_name)
        .bind(limit_value as i64)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ChatMessage>(
            "SELECT id, user_id, sender, message, app_name
             FROM chat_messages
             WHERE user_id = ? AND app_name = ?
             ORDER BY id ASC"
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
    sqlx::query(
        "INSERT INTO chat_messages (user_id, sender, message, app_name)
         VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(sender)
    .bind(message)
    .bind(app_name)
    .execute(pool)
    .await?;

    delete_old_messages(pool, user_id, app_name, 100).await?;

    Ok(())
}

pub async fn delete_old_messages(
    pool: &SqlitePool,
    user_id: &str,
    app_name: &str,
    max_messages: i64,
) -> Result<(), Error> {
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM chat_messages WHERE user_id = ? AND app_name = ?")
            .bind(user_id)
            .bind(app_name)
            .fetch_one(pool)
            .await?;

    if count.0 > max_messages {
        let excess = count.0 - max_messages;

        info!(
            "Deleting {} oldest messages for user_id={} AND app_name={}",
            excess, user_id, app_name
        );

        sqlx::query(
            "DELETE FROM chat_messages WHERE id IN (
                SELECT id FROM chat_messages WHERE user_id = ? AND app_name = ?
                ORDER BY id ASC LIMIT ?
            )",
        )
        .bind(user_id)
        .bind(app_name)
        .bind(excess)
        .execute(pool)
        .await?;
    }

    Ok(())
}
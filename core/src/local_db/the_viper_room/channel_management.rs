use crate::models::the_viper_room::db_models::UserChannel;
use sqlx::{Pool, Sqlite};
use tracing::info;

pub async fn add_channel(
    db_pool: &Pool<Sqlite>,
    user_id: i64,
    channel_id: i64,
    channel_title: &str,
    channel_username: &str,
) -> anyhow::Result<()> {
    let query = "
        INSERT INTO user_channels (user_id, channel_id, channel_title, channel_username)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(user_id, channel_id) DO UPDATE SET
            channel_title = excluded.channel_title,
            channel_username = excluded.channel_username
    ";

    sqlx::query(query)
        .bind(user_id)
        .bind(channel_id)
        .bind(channel_title)
        .bind(channel_username)
        .execute(db_pool)
        .await?;

    info!(
        "Channel '{}' ({}) [@{}] added for user {}",
        channel_title, channel_id, channel_username, user_id
    );

    Ok(())
}

pub async fn remove_channel(
    db_pool: &Pool<Sqlite>,
    user_id: i64,
    channel_id: i64,
) -> anyhow::Result<()> {
    let query = "DELETE FROM user_channels WHERE user_id = ? AND channel_id = ?";

    let result = sqlx::query(query)
        .bind(user_id)
        .bind(channel_id)
        .execute(db_pool)
        .await?;

    if result.rows_affected() > 0 {
        info!("Channel {} removed for user {}", channel_id, user_id);
    } else {
        info!(
            "Channel {} not found for user {} (nothing to remove)",
            channel_id, user_id
        );
    }

    Ok(())
}

pub async fn get_user_channels(
    db_pool: &Pool<Sqlite>,
    user_id: i64,
) -> anyhow::Result<Vec<UserChannel>> {
    let query = "SELECT id, user_id, channel_id, channel_title, channel_username FROM user_channels WHERE user_id = ? ORDER BY channel_title";

    let channels = sqlx::query_as::<_, UserChannel>(query)
        .bind(user_id)
        .fetch_all(db_pool)
        .await?;

    Ok(channels)
}

pub async fn get_channel(
    db_pool: &Pool<Sqlite>,
    user_id: i64,
    channel_id: i64,
) -> anyhow::Result<Option<UserChannel>> {
    let query = "SELECT id, user_id, channel_id, channel_title, channel_username FROM user_channels WHERE user_id = ? AND channel_id = ?";

    let channel = sqlx::query_as::<_, UserChannel>(query)
        .bind(user_id)
        .bind(channel_id)
        .fetch_optional(db_pool)
        .await?;

    Ok(channel)
}

pub async fn clear_user_channels(db_pool: &Pool<Sqlite>, user_id: i64) -> anyhow::Result<()> {
    let query = "DELETE FROM user_channels WHERE user_id = ?";

    let result = sqlx::query(query).bind(user_id).execute(db_pool).await?;

    info!(
        "Cleared {} channels for user {}",
        result.rows_affected(),
        user_id
    );

    Ok(())
}

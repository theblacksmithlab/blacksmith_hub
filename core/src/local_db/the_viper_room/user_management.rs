use crate::models::the_viper_room::db_models::User;
use sqlx::{Pool, Sqlite};
use tracing::info;

pub async fn create_or_update_user(
    db_pool: &Pool<Sqlite>,
    user_id: i64,
    telegram_username: Option<&str>,
) -> anyhow::Result<()> {
    let query = "
        INSERT INTO users (user_id, telegram_username)
        VALUES (?, ?)
        ON CONFLICT(user_id) DO UPDATE SET
            telegram_username = excluded.telegram_username
    ";

    sqlx::query(query)
        .bind(user_id)
        .bind(telegram_username)
        .execute(db_pool)
        .await?;

    info!(
        "User {} (username: @{}) created/updated",
        user_id,
        telegram_username.unwrap_or("no_username")
    );

    Ok(())
}

pub async fn get_user(db_pool: &Pool<Sqlite>, user_id: i64) -> anyhow::Result<Option<User>> {
    let query = "SELECT user_id, telegram_username FROM users WHERE user_id = ?";

    let user = sqlx::query_as::<_, User>(query)
        .bind(user_id)
        .fetch_optional(db_pool)
        .await?;

    Ok(user)
}

pub async fn delete_user(db_pool: &Pool<Sqlite>, user_id: i64) -> anyhow::Result<()> {
    let query = "DELETE FROM users WHERE user_id = ?";

    sqlx::query(query).bind(user_id).execute(db_pool).await?;

    info!("User {} deleted", user_id);

    Ok(())
}

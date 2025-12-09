use crate::models::the_viper_room::db_models::User;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::tg_bot::the_viper_room_bot::the_viper_room_bot_utils::generate_user_nickname;
use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tracing::{info, warn};

pub async fn create_or_update_user<T>(
    db_pool: &Pool<Sqlite>,
    user_id: u64,
    telegram_username: Option<&str>,
    first_name: Option<&str>,
    last_name: Option<&str>,
    app_state: Arc<T>,
) -> Result<()>
where
    T: OpenAIClientInit + Send + Sync + 'static,
{
    let username_str = telegram_username.unwrap_or("mommy's_anon");
    let first_str = first_name.unwrap_or("null");
    let last_str = last_name.unwrap_or("null");

    let existing_user = get_user(db_pool, user_id).await?;

    let nickname = if let Some(user) = existing_user {
        info!(
            "User {} already exists with nickname '{}', keeping it",
            user_id,
            user.nickname.as_deref().unwrap_or("Unknown")
        );
        user.nickname.unwrap_or_else(|| {
            warn!("User {} has no nickname, using fallback", user_id);
            format!("{}_{}", first_str, user_id % 1000)
        })
    } else {
        info!("New user {}, generating nickname via LLM", user_id);
        match generate_user_nickname(
            app_state,
            username_str.to_string(),
            first_str.to_string(),
            last_str.to_string(),
        )
        .await
        {
            Ok(nick) => nick,
            Err(e) => {
                warn!("Failed to generate nickname: {}. Using fallback", e);
                format!("{}_{}", first_str, user_id % 1000)
            }
        }
    };

    let query = "
        INSERT INTO users (user_id, telegram_username, first_name, last_name, nickname)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(user_id) DO UPDATE SET
            telegram_username = excluded.telegram_username,
            first_name = excluded.first_name,
            last_name = excluded.last_name,
            nickname = excluded.nickname
    ";

    sqlx::query(query)
        .bind(user_id as i64)
        .bind(telegram_username)
        .bind(first_name)
        .bind(last_name)
        .bind(&nickname)
        .execute(db_pool)
        .await?;

    info!(
        "User {} (username: @{}, nickname: {}) created/updated",
        user_id, username_str, nickname
    );

    Ok(())
}

pub async fn get_user(db_pool: &Pool<Sqlite>, user_id: u64) -> Result<Option<User>> {
    let query = "SELECT user_id, telegram_username, first_name, last_name, nickname FROM users WHERE user_id = ?";

    let user = sqlx::query_as::<_, User>(query)
        .bind(user_id as i64)
        .fetch_optional(db_pool)
        .await?;

    Ok(user)
}

pub async fn get_user_nickname(db_pool: &Pool<Sqlite>, user_id: u64) -> Result<Option<String>> {
    let user = get_user(db_pool, user_id).await?;
    Ok(user.and_then(|u| u.nickname))
}

pub async fn delete_user(db_pool: &Pool<Sqlite>, user_id: i64) -> anyhow::Result<()> {
    let query = "DELETE FROM users WHERE user_id = ?";

    sqlx::query(query).bind(user_id).execute(db_pool).await?;

    info!("User {} deleted", user_id);

    Ok(())
}

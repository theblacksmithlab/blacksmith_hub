use crate::state::request_app::app_state::{AdditionalInfo, RegistrationInfo, UserProfile};
use sqlx::{Error, SqlitePool};
use sqlx::{FromRow, Pool, Sqlite};
use std::path::Path;
use teloxide::prelude::ChatId;
use tokio::sync::Mutex;
use tracing::info;
use crate::models::blacksmith_web::blacksmith_web::ChatMessage;

#[derive(Debug, FromRow)]
struct DbUserProfile {
    user_id: i64,
    first_name: Option<String>,
    last_name: Option<String>,
    age: Option<u8>,
    gender: Option<String>,
    city_of_residence: Option<String>,
    interests: Option<String>,
}

pub(crate) async fn initialize_database() -> Result<(), Error> {
    if !Path::new("user_profiles.db").exists() {
        std::fs::File::create("user_profiles.db")?;
    }
    Ok(())
}

pub async fn create_db_pool() -> Result<Pool<Sqlite>, Error> {
    initialize_database().await?;
    let pool = sqlx::SqlitePool::connect("sqlite:user_profiles.db").await?;
    Ok(pool)
}

pub async fn create_table(pool: &Mutex<Option<Pool<Sqlite>>>) -> Result<(), Error> {
    let pool = pool.lock().await;
    if let Some(pool) = &*pool {
        let query = r#"
            CREATE TABLE IF NOT EXISTS user_profiles (
                user_id INTEGER PRIMARY KEY,
                first_name TEXT,
                last_name TEXT,
                age INTEGER,
                gender TEXT,
                city_of_residence TEXT,
                interests TEXT
            );
        "#;

        sqlx::query(query).execute(pool).await?;

        Ok(())
    } else {
        Err(Error::Configuration("DB pool is not initialized".into()))
    }
}

pub(crate) async fn save_user_profile(
    pool: &Mutex<Option<Pool<Sqlite>>>,
    user_id: i64,
    user_profile: &UserProfile,
) -> Result<(), Error> {
    let pool = pool.lock().await;
    if let Some(pool) = &*pool {
        let interests = user_profile
            .additional_info
            .interests
            .as_ref()
            .map(|ints| ints.join(", "))
            .unwrap_or_default();

        let query = r#"
            INSERT INTO user_profiles (
                user_id, first_name, last_name, age, gender, city_of_residence, interests
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET
                first_name = excluded.first_name,
                last_name = excluded.last_name,
                age = excluded.age,
                gender = excluded.gender,
                city_of_residence = excluded.city_of_residence,
                interests = excluded.interests;
        "#;

        sqlx::query(query)
            .bind(user_id)
            .bind(user_profile.registration_info.first_name.as_deref())
            .bind(user_profile.registration_info.last_name.as_deref())
            .bind(user_profile.registration_info.age)
            .bind(user_profile.registration_info.gender.as_deref())
            .bind(user_profile.registration_info.city_of_residence.as_deref())
            .bind(interests)
            .execute(pool)
            .await?;

        info!("TEMP: Fn: save_user_profile | User_profile saved to local_bd");

        Ok(())
    } else {
        Err(Error::Configuration("DB pool is not initialized".into()))
    }
}

pub async fn get_user_profile_from_db(
    pool: &Pool<Sqlite>,
    chat_id: ChatId,
) -> Result<Option<UserProfile>, Error> {
    let query = r#"
        SELECT user_id, first_name, last_name, age, gender, city_of_residence, interests
        FROM user_profiles
        WHERE user_id = ?
    "#;

    let db_profile = sqlx::query_as::<_, DbUserProfile>(query)
        .bind(chat_id.0)
        .fetch_optional(pool)
        .await?;

    if let Some(db_profile) = db_profile {
        let user_profile = UserProfile {
            registration_info: RegistrationInfo {
                first_name: db_profile.first_name,
                last_name: db_profile.last_name,
                age: db_profile.age,
                gender: db_profile.gender,
                city_of_residence: db_profile.city_of_residence,
            },
            additional_info: AdditionalInfo {
                interests: db_profile.interests.map(|ints| {
                    if ints.trim().is_empty() {
                        vec![]
                    } else {
                        ints.split(',').map(|s| s.trim().to_string()).collect()
                    }
                }),
            },
        };

        Ok(Some(user_profile))
    } else {
        Ok(None)
    }
}

pub async fn create_blacksmith_labs_db_table(pool: &SqlitePool) -> Result<(), Error> {
    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            sender TEXT NOT NULL,
            message TEXT NOT NULL,
            app_name TEXT NOT NULL
        )"
    )
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn fetch_chat_history_from_db(
    pool: &SqlitePool,
    user_id: &str,
    app_name: &str,
) -> Result<Vec<ChatMessage>, Error> {
    let messages = sqlx::query_as!(
        ChatMessage,
        "SELECT id, user_id, sender, message, app_name FROM chat_messages WHERE user_id = ? AND app_name = ? ORDER BY id ASC",
        user_id,
        app_name
    )
        .fetch_all(pool)
        .await?;

    Ok(messages)
}

pub async fn save_message_to_db(
    pool: &SqlitePool,
    user_id: &str,
    sender: &str,
    message: &str,
    app_name: &str,
) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO chat_messages (user_id, sender, message, app_name) VALUES (?, ?, ?, ?)",
        user_id,
        sender,
        message,
        app_name
    )
        .execute(pool)
        .await?;

    Ok(())
}

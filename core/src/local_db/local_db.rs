use std::{env, fs};
use std::path::Path;
use std::str::FromStr;
use anyhow::Context;
use crate::state::request_app::app_state::{AdditionalInfo, RegistrationInfo, UserProfile};
use sqlx::{Error, Executor, SqlitePool};
use sqlx::{FromRow, Pool, Sqlite};
use sqlx::sqlite::SqliteConnectOptions;
use teloxide::prelude::ChatId;
use tokio::sync::Mutex;
use tracing::{info, warn};
use crate::models::blacksmith_web::blacksmith_web::ChatMessage;

pub async fn setup_blacksmith_lab_db() -> anyhow::Result<SqlitePool> {
    let blacksmith_lab_database_url = env::var("BLACKSMITH_LAB_DATABASE_URL")
        .context("Error: BLACKSMITH_LAB_DATABASE_URL must be set")?;

    let db_path = "blacksmith_lab.db";

    if !Path::new(db_path).exists() {
        fs::File::create(db_path)
            .context("Error creating local db file for Blacksmith Lab")?;
        warn!("Blacksmith Lab local_db file {} created.", db_path);
    }

    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::from_str(&blacksmith_lab_database_url)
            .context("Error: invalid DATABASE_URL format")?
            .create_if_missing(true)
    ).await.context("Error connecting to db pool")?;

    info!("Blacksmith Lab local_db pool initialized successfully");

    create_blacksmith_lab_db_table(&pool)
        .await
        .context("Error creating table in Blacksmith Lab local db")?;

    info!("Blacksmith Lab table created successfully");

    Ok(pool)
}

pub async fn setup_request_app_db() -> anyhow::Result<SqlitePool> {
    let request_app_database_url = env::var("REQUEST_APP_DATABASE_URL")
        .context("Error: REQUEST_APP_DATABASE_URL must be set")?;

    let db_path = "request_app.db";

    if !Path::new(db_path).exists() {
        fs::File::create(db_path)
            .context("Error creating local db file for Request App")?;
        warn!("Request App local_db file {} created.", db_path);
    }

    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::from_str(&request_app_database_url)
            .context("Error: invalid DATABASE_URL format")?
            .create_if_missing(true)
    ).await.context("Error connecting to db pool")?;

    info!("Request App local_db pool initialized successfully");

    create_request_app_db_table(&pool)
        .await
        .context("Error creating table in Request App local db")?;

    info!("Request App table created successfully");

    Ok(pool)
}

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

pub async fn create_request_app_db_table(pool: &SqlitePool) -> Result<(), Error> {
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

pub async fn create_blacksmith_lab_db_table(pool: &SqlitePool) -> Result<(), Error> {
    let query = "
        CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            sender TEXT NOT NULL,
            message TEXT NOT NULL,
            app_name TEXT NOT NULL
        );
    ";

    pool.execute(query).await?;

    Ok(())
}

pub async fn fetch_chat_history_from_db(
    pool: &SqlitePool,
    user_id: &str,
    app_name: &str,
) -> Result<Vec<ChatMessage>, Error> {
    info!("Executing query: SELECT id, user_id, sender, message, app_name FROM chat_messages WHERE user_id = '{}' AND app_name = '{}'", user_id, app_name);

    let messages = sqlx::query_as::<_, ChatMessage>(
        "SELECT id, user_id, sender, message, app_name FROM chat_messages
         WHERE user_id = ? AND app_name = ? ORDER BY id ASC"
    )
        .bind(user_id)
        .bind(app_name)
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
    sqlx::query(
        "INSERT INTO chat_messages (user_id, sender, message, app_name)
         VALUES (?, ?, ?, ?)"
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
    max_messages: i64
) -> Result<(), Error> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM chat_messages WHERE user_id = ? AND app_name = ?"
    )
        .bind(user_id)
        .bind(app_name)
        .fetch_one(pool)
        .await?;

    if count.0 > max_messages {
        let excess = count.0 - max_messages;

        info!("Deleting {} oldest messages for user_id={} AND app_name={}", excess, user_id, app_name);

        sqlx::query(
            "DELETE FROM chat_messages WHERE id IN (
                SELECT id FROM chat_messages WHERE user_id = ? AND app_name = ?
                ORDER BY id ASC LIMIT ?
            )"
        )
            .bind(user_id)
            .bind(app_name)
            .bind(excess)
            .execute(pool)
            .await?;
    }

    Ok(())
}

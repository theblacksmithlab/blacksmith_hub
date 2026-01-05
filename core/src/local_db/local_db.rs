use crate::models::common::app_name::AppName;
use anyhow::Context;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Error, Executor, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use std::{env, fs};
use tracing::{info, warn};

pub async fn setup_app_db_pool(app_name: &AppName) -> anyhow::Result<SqlitePool> {
    let env_var_name = match app_name {
        AppName::BlacksmithWeb | AppName::W3AWeb => "BLACKSMITH_LAB_DATABASE_URL",
        AppName::GrootBot => "GROOT_BOT_DATABASE_URL",
        AppName::AgentDavon => "AGENT_DAVON_DATABASE_URL",
        AppName::TheViperRoom | AppName::TheViperRoomBot => "THE_VIPER_ROOM_DATABASE_URL",
        _ => {
            return Err(anyhow::anyhow!(
                "Database not supported for app: {}",
                app_name.as_str()
            ))
        }
    };

    let database_url =
        env::var(env_var_name).with_context(|| format!("Error: {} must be set", env_var_name))?;

    let db_path = database_url
        .strip_prefix("sqlite://")
        .unwrap_or(&database_url);

    if !Path::new(db_path).exists() {
        if let Some(parent) = Path::new(db_path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Error creating directory for {} DB", app_name.as_str())
                })?;
            }
        }
        fs::File::create(db_path)
            .with_context(|| format!("Error creating db file for {}", app_name.as_str()))?;
        warn!("{} db file {} created", app_name.as_str(), db_path);
    }

    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::from_str(&database_url)
            .context("Error: invalid DATABASE_URL format")?
            .create_if_missing(true),
    )
    .await
    .with_context(|| format!("Error connecting to {} db pool", app_name.as_str()))?;

    info!("{} db pool initialized successfully", app_name.as_str());

    create_app_db_tables(&pool, app_name)
        .await
        .with_context(|| format!("Error creating tables in {} db", app_name.as_str()))?;

    info!("{} tables are ready", app_name.as_str());

    Ok(pool)
}

async fn create_app_db_tables(pool: &SqlitePool, app_name: &AppName) -> Result<(), Error> {
    match app_name {
        AppName::BlacksmithWeb | AppName::W3AWeb => {
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
        }
        AppName::GrootBot => {
            let query = "
                CREATE TABLE IF NOT EXISTS subscriptions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    chat_id INTEGER NOT NULL UNIQUE,
                    chat_username TEXT NOT NULL,
                    paid_by_user_id INTEGER NOT NULL,
                    paid_by_username TEXT,
                    start_date TEXT NOT NULL,
                    end_date TEXT NOT NULL,
                    plan_type TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );

                CREATE INDEX IF NOT EXISTS idx_chat_id ON subscriptions(chat_id);
                CREATE INDEX IF NOT EXISTS idx_end_date ON subscriptions(end_date);
            ";
            sqlx::query(query).execute(pool).await?;
        }
        AppName::AgentDavon => {
            let query = "
                CREATE TABLE IF NOT EXISTS chat_monitoring (
                    chat_id INTEGER PRIMARY KEY,
                    chat_title TEXT NOT NULL,
                    chat_username TEXT,
                    first_message_time TEXT,
                    last_report_sent TEXT,
                    spam_count INTEGER DEFAULT 0,
                    total_messages INTEGER DEFAULT 0,
                    status TEXT NOT NULL DEFAULT 'collecting',
                    report_response TEXT
                );

                CREATE TABLE IF NOT EXISTS chat_admins (
                    chat_id INTEGER,
                    user_id INTEGER,
                    role TEXT,
                    fetched_at TEXT,
                    PRIMARY KEY (chat_id, user_id)
                );

                CREATE TABLE IF NOT EXISTS spam_messages (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    chat_id INTEGER NOT NULL,
                    user_id INTEGER NOT NULL,
                    username TEXT,
                    message_text TEXT NOT NULL,
                    detected_at TEXT NOT NULL,
                    FOREIGN KEY (chat_id) REFERENCES chat_monitoring(chat_id)
                );

                CREATE INDEX IF NOT EXISTS idx_chat_monitoring_status ON chat_monitoring(status);
                CREATE INDEX IF NOT EXISTS idx_chat_monitoring_first_time ON chat_monitoring(first_message_time);
                CREATE INDEX IF NOT EXISTS idx_spam_messages_chat_id ON spam_messages(chat_id);
                CREATE INDEX IF NOT EXISTS idx_spam_messages_detected_at ON spam_messages(detected_at);
            ";
            sqlx::query(query).execute(pool).await?;
        }
        AppName::TheViperRoom | AppName::TheViperRoomBot => {
            let query = "
                CREATE TABLE IF NOT EXISTS users (
                    user_id INTEGER PRIMARY KEY,
                    telegram_username TEXT,
                    first_name TEXT,
                    last_name TEXT,
                    nickname TEXT
                );

                CREATE TABLE IF NOT EXISTS user_channels (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id INTEGER NOT NULL,
                    channel_id INTEGER NOT NULL,
                    channel_title TEXT NOT NULL,
                    channel_username TEXT NOT NULL,
                    UNIQUE(user_id, channel_id)
                );
            ";
            sqlx::query(query).execute(pool).await?;
        }
        _ => {
            info!("No tables to create for {}", app_name.as_str());
        }
    }
    Ok(())
}

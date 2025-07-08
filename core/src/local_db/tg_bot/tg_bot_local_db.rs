use std::fs;
use std::path::Path;
use anyhow::Context;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use tracing::{info, warn};
use crate::models::common::app_name::AppName;


pub async fn setup_bot_localdb_pool(app_name: &AppName) -> anyhow::Result<SqlitePool> {
    let db_path = match app_name {
        AppName::GrootBot => "common_res/local_db/groot_bot.db",
        _ => return Err(anyhow::anyhow!("Database not supported for app: {}", app_name.as_str())),
    };

    if !Path::new(db_path).exists() {
        if let Some(parent) = Path::new(db_path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .context("Error creating directory for bot DB")?;
            }
        }
        fs::File::create(db_path).context("Error creating db file for bot")?;
        warn!("New db file for {} created at: {}", app_name.as_str(), db_path);
    }

    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true),
    )
        .await
        .context("Error connecting to bot db pool")?;

    info!("{} db pool initialized successfully", app_name.as_str());

    create_bot_localdb_tables(&pool, app_name)
        .await
        .context("Error creating tables in bot db")?;

    info!("{} tables are ready", app_name.as_str());

    Ok(pool)
}


async fn create_bot_localdb_tables(pool: &SqlitePool, app_name: &AppName) -> Result<(), sqlx::Error> {
    match app_name {
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
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                );

                CREATE INDEX IF NOT EXISTS idx_chat_id ON subscriptions(chat_id);
                CREATE INDEX IF NOT EXISTS idx_end_date ON subscriptions(end_date);
            ";
            sqlx::query(query).execute(pool).await?;
        },
        _ => {
            info!("No tables to create for {}", app_name.as_str());
        }
    }
    Ok(())
}
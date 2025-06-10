use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Executor, SqlitePool};
use std::{env, fs};
use anyhow::Context;
use std::path::Path;
use std::str::FromStr;
use tracing::{info, warn};

pub async fn setup_uniframe_studio_db() -> anyhow::Result<SqlitePool> {
    let auth_database_url = env::var("UNIFRAME_STUDIO_DATABASE_URL")
        .context("Error: UNIFRAME_STUDIO_DATABASE_URL must be set")?;

    let db_path = "common_res/local_db/uniframe_studio.db";

    if !Path::new(db_path).exists() {
        if let Some(parent) = Path::new(db_path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .context("Error creating directory for Uniframe Studio DB")?;
            }
        }

        fs::File::create(db_path).context("Error creating db file for Uniframe Studio")?;
        warn!("Uniframe Studio db file {} created.", db_path);
    }

    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::from_str(&auth_database_url)
            .context("Error: invalid DATABASE_URL format")?
            .create_if_missing(true),
    )
        .await
        .context("Error connecting to db pool")?;

    info!("Uniframe Studio db pool initialized successfully");

    create_uniframe_studio_tables(&pool)
        .await
        .context("Error creating tables in Uniframe Studio db")?;

    info!("Uniframe Studio tables created successfully");

    Ok(pool)
}

async fn create_uniframe_studio_tables(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let query = "
        CREATE TABLE IF NOT EXISTS auth_users (
            id TEXT PRIMARY KEY,
            email TEXT UNIQUE NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS auth_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            token TEXT UNIQUE NOT NULL,
            expires_at DATETIME NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS auth_magic_links (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL,
            token TEXT UNIQUE NOT NULL,
            expires_at DATETIME NOT NULL,
            used BOOLEAN DEFAULT FALSE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
    ";

    pool.execute(query).await?;
    Ok(())
}
use anyhow::Context;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Executor, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use std::{env, fs};
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
        warn!("New db file for Uniframe Studio created at: {}", db_path);
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
            created_at TEXT DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS auth_sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            token TEXT UNIQUE NOT NULL,
            expires_at TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS auth_magic_links (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL,
            token TEXT UNIQUE NOT NULL,
            expires_at TEXT NOT NULL,
            used BOOLEAN DEFAULT FALSE,
            created_at TEXT DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS dubbing_pipelines (
            job_id TEXT PRIMARY KEY,
            user_id TEXT,
            status TEXT NOT NULL DEFAULT 'preparing',
            step INTEGER,
            step_description TEXT NOT NULL DEFAULT 'Preparing pipeline...',
            progress_percentage INTEGER,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now')),
            completed_at TEXT,
            result_urls TEXT,
            error_message TEXT,
            processing_steps TEXT,
            original_video_s3_url TEXT,
            system_file_name TEXT,
            original_file_name TEXT,
            review_required_url TEXT,
            video_duration_seconds INTEGER,
            estimated_cost_usd REAL,
            FOREIGN KEY (user_id) REFERENCES auth_users(id)
        );

        CREATE TABLE IF NOT EXISTS lipsync_pipelines (
            job_id TEXT PRIMARY KEY,
            user_id TEXT,
            status TEXT NOT NULL DEFAULT 'preparing',
            step INTEGER,
            step_description TEXT NOT NULL DEFAULT 'Preparing pipeline...',
            progress_percentage INTEGER,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now')),
            completed_at TEXT,
            result_urls TEXT,
            error_message TEXT,
            processing_steps TEXT,
            original_video_s3_url TEXT,
            system_file_name TEXT,
            original_file_name TEXT,
            review_required_url TEXT,
            FOREIGN KEY (user_id) REFERENCES auth_users(id)
        );
        
        CREATE TABLE IF NOT EXISTS user_balances (
            user_id TEXT PRIMARY KEY,
            balance_usd REAL NOT NULL DEFAULT 100.0,
            updated_at TEXT DEFAULT (datetime('now')),
            active_dubbing_jobs INTEGER NOT NULL DEFAULT 0,
            active_lipsync_jobs INTEGER NOT NULL DEFAULT 0,
            max_concurrent_dubbing_jobs INTEGER NOT NULL DEFAULT 1,
            max_concurrent_lipsync_jobs INTEGER NOT NULL DEFAULT 1,
            FOREIGN KEY (user_id) REFERENCES auth_users(id)
        );

        CREATE TABLE IF NOT EXISTS transactions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            type TEXT NOT NULL CHECK (type IN ('deposit', 'charge')),
            amount_usd REAL NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'completed', 'failed')),
            description TEXT,
            heleket_invoice_id TEXT,
            crypto_tx_hash TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            completed_at TEXT,
            FOREIGN KEY (user_id) REFERENCES auth_users(id)
        );

        CREATE TABLE IF NOT EXISTS pricing_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            duration_min INTEGER NOT NULL,
            duration_max INTEGER,
            price_per_minute REAL NOT NULL,
            created_at TEXT DEFAULT (datetime('now'))
        );

    ";

    pool.execute(query).await?;
    Ok(())
}

use anyhow::Result;
use core::local_db::tg_bot::tg_bot_local_db::setup_localdb_pool;
use core::models::common::app_name::AppName;
use core::models::tg_agent::bot_alias::GrootBotAlias;
use core::telegram_client::telegram_client::TelegramAgent;
use core::utils::tg_bot::tg_bot::create_app_tmp_dir;
use dotenv::dotenv;
use rustls::crypto::{aws_lc_rs, CryptoProvider};
use std::env;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = CryptoProvider::install_default(aws_lc_rs::default_provider()) {
        error!("Failed to install CryptoProvider: {:?}", e);
    }

    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    info!("Determining AppName of the Telegram agent being launched...");

    let app_name_str = env::var("APP_NAME").unwrap_or_else(|_| "tester_bot".to_string());
    let app_name = match app_name_str.as_str() {
        "agent_davon" => AppName::AgentDavon,
        "the_viper_room" | "w3a_web" | "blacksmith_web" | "probiot_bot" | "the_viper_room_bot"
        | "tester_bot" | "w3a_bot" | "groot_bot" => {
            info!(
                "No Telegram agent system implementation for {}",
                app_name_str
            );
            return Ok(());
        }
        _ => return Err(anyhow::anyhow!("Unknown APP_NAME: {}", app_name_str)),
    };

    if let Err(e) = create_app_tmp_dir(&app_name) {
        error!("Failed to create app tmp directory: {}", e);
    }

    let db_pool = setup_localdb_pool(&app_name).await?;

    //TODO: Place *.session file to the ./common_res/agent_davon/ folder
    let telegram_agent = TelegramAgent::new(&app_name, "current.session").await?;

    let groot_bot_alias = GrootBotAlias::new(
        env::var("GROOT_BOT_ID")?.parse()?,
        env::var("GROOT_BOT_USERNAME")?,
    );

    info!("Starting | {} | Telegram agent...", app_name_str);

    telegram_agent
        .start_monitoring(groot_bot_alias, db_pool)
        .await?;

    Ok(())
}

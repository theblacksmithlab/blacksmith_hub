use crate::blacksmith_web::server::start_blacksmith_web_server;
use crate::the_viper_room::server::start_the_viper_room_server;
use crate::uniframe_studio::server::start_uniframe_studio_server;
use anyhow::{anyhow, Result};
use config::{Config, File};
use core::models::common::app_name::AppName;
use core::server_config::server_config::get_config_path;
use core::server_config::server_config::AppConfig;
use core::state::server_common::app_state::ServerAppState;
use core::utils::tg_bot::tg_bot::create_app_tmp_dir;
use dotenv::dotenv;
use rustls::crypto::{aws_lc_rs, CryptoProvider};
use std::env;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod blacksmith_web;
mod the_viper_room;
mod uniframe_studio;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = CryptoProvider::install_default(aws_lc_rs::default_provider()) {
        error!("Failed to install CryptoProvider: {:?}", e);
    }

    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    info!("Determining AppName of the service being launched...");

    let app_name_str = env::var("APP_NAME").map_err(|_| {
        error!("APP_NAME environment variable is not set");
        anyhow!("APP_NAME environment variable must be set")
    })?;

    let app_name = match app_name_str.as_str() {
        "blacksmith_web" => AppName::BlacksmithWeb,
        "the_viper_room" => AppName::TheViperRoom,
        "uniframe_studio" => AppName::UniframeStudio,
        "probiot_bot" | "the_viper_room_bot" | "groot_bot" => {
            info!("No server implementation for {}", app_name_str);
            return Ok(());
        }
        "w3a_web" => {
            info!("W3A Web is serving by 'Blacksmith Web' App...");
            return Ok(());
        }
        _ => return Err(anyhow!("Unknown APP_NAME: {}", app_name_str)),
    };

    if let Err(e) = create_app_tmp_dir(&app_name) {
        error!("Failed to create app tmp directory: {}", e);
    }

    let config_path = get_config_path(&app_name);

    let config_builder = Config::builder().add_source(File::with_name(&config_path));
    let config: AppConfig = config_builder.build()?.try_deserialize()?;
    let config = Arc::new(config);

    let server_app_state = Arc::new(ServerAppState::new(config.clone()));

    match app_name {
        AppName::BlacksmithWeb => {
            start_blacksmith_web_server(server_app_state).await?;
        }
        AppName::TheViperRoom => {
            start_the_viper_room_server(server_app_state).await?;
        }
        AppName::UniframeStudio => {
            start_uniframe_studio_server(server_app_state).await?;
        }
        _ => {
            return Err(anyhow!(
                "No server implementation for app: {}",
                app_name.as_str()
            ));
        }
    }

    Ok(())
}

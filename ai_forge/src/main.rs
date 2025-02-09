use anyhow::Result;
use async_openai::Client as LLM_Client;
use config::{Config, File};
use core::config::server_config::AppConfig;
use core::local_db::local_db::setup_blacksmith_lab_db;
use core::local_db::local_db::setup_request_app_db;
use core::state::blacksmith_web::app_state::BlacksmithWebAppState;
use core::state::request_app::app_state::RequestAppState;
use core::state::server::app_state::ServerAppState;
use core::state::the_viper_room::app_state::TheViperRoomAppState;
use dotenv::dotenv;
use qdrant_client::Qdrant;
use server::start_server;
use std::env;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    info!("Initializing system components...");

    dotenv().ok();

    let config_builder = Config::builder().add_source(File::with_name("config.yaml"));
    let config: AppConfig = config_builder.build()?.try_deserialize()?;
    let config = Arc::new(config);

    let qdrant_client = Arc::new(
        Qdrant::from_url(&env::var("QDRANT_URL")?)
            .api_key(env::var("QDRANT_API_KEY")?)
            .build()?,
    );

    let llm_client = LLM_Client::new();

    let blacksmith_lab_db_pool = setup_blacksmith_lab_db().await?;
    let _request_app_db_pool = setup_request_app_db().await?;

    let server_app_state = Arc::new(ServerAppState::new(config.clone()));
    let request_app_state = Arc::new(RequestAppState::new(
        qdrant_client.clone(),
        llm_client.clone(),
    ));
    let the_viper_room_app_state = Arc::new(TheViperRoomAppState::new(llm_client.clone()));
    let blacksmith_web_app_state = Arc::new(BlacksmithWebAppState::new(
        llm_client.clone(),
        qdrant_client.clone(),
        blacksmith_lab_db_pool.clone(),
    ));

    info!("Starting server...");
    tokio::spawn(async move {
        if let Err(e) = start_server(
            server_app_state,
            request_app_state,
            the_viper_room_app_state,
            blacksmith_web_app_state,
        )
        .await
        {
            tracing::error!("Server error: {:?}", e);
        }
    });
    info!("Server started successfully");

    info!("All system components initialized successfully");

    tokio::signal::ctrl_c().await?;

    Ok(())
}

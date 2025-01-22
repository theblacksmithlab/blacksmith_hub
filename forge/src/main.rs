use anyhow::Result;
use async_openai::Client as LLM_Client;
use config::{Config, File};
use core::config::server_config::AppConfig;
use core::local_db::local_db::{create_db_pool, create_table};
use core::state::request_app::app_state::RequestAppState;
use core::state::server::app_state::ServerAppState;
use core::state::tg_bot::app_state::BotAppState;
use core::state::the_viper_room::app_state::TheViperRoomAppState;
use dotenv::dotenv;
use qdrant_client::Qdrant;
use request_app_bot::start_request_app_bot;
use server::start_server;
use std::env;
use std::sync::Arc;
use the_viper_room_bot::start_the_viper_room_bot;
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

    let qdrant_client = Qdrant::from_url(&env::var("QDRANT_URL")?)
        .api_key(env::var("QDRANT_API_KEY")?)
        .build()?;

    let llm_client = LLM_Client::new();

    let server_app_state = Arc::new(ServerAppState::new(config.clone()));
    let request_app_state = Arc::new(RequestAppState::new(qdrant_client, llm_client.clone()));
    let the_viper_room_app_state = Arc::new(TheViperRoomAppState::new(llm_client.clone()));
    let bot_app_state = Arc::new(BotAppState::new(llm_client));

    info!("Initializing local_db pool...");
    let local_db_pool = create_db_pool().await?;
    info!("Local_db pool initialized successfully");

    {
        let mut db_pool = request_app_state.local_db_pool.lock().await;
        *db_pool = Some(local_db_pool);
    }

    info!("Trying to create local_db table...");
    create_table(&request_app_state.local_db_pool).await?;
    info!("Local_db table created successfully");

    info!("Starting server...");
    tokio::spawn(async move {
        if let Err(e) = start_server(
            server_app_state,
            request_app_state,
            the_viper_room_app_state,
        )
        .await
        {
            tracing::error!("Server error: {:?}", e);
        }
    });
    info!("Server started successfully");

    let request_app_bot_state = bot_app_state.clone();
    let viper_room_bot_state = bot_app_state.clone();

    info!("Starting Request App bot...");
    tokio::spawn(async move {
        if let Err(e) = start_request_app_bot(request_app_bot_state).await {
            tracing::error!("Request App Bot error: {:?}", e);
        }
        info!("Request App bot started successfully");
    });

    info!("Starting The Viper Room bot...");
    tokio::spawn(async move {
        if let Err(e) = start_the_viper_room_bot(viper_room_bot_state).await {
            tracing::error!("Bot error: {:?}", e);
        }
        info!("The Viper Room bot started successfully");
    });

    info!("All system components initialized successfully");

    tokio::signal::ctrl_c().await?;

    Ok(())
}

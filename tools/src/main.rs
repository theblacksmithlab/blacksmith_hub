mod qdrant_data_pre_processing;
mod video_to_audio_converter;
mod w3a_qdrant_data_processing;

use crate::qdrant_data_pre_processing::validate_input_data;
use crate::video_to_audio_converter::convert_videos_to_wav;
use crate::w3a_qdrant_data_processing::{upsert_data_to_qdrant, upsert_w3a_data_to_qdrant};
use async_openai::Client as LLM_Client;
use clap::Parser;
use clap_derive::{Parser, Subcommand};
use dotenv::dotenv;
use qdrant_client::Qdrant;
use std::env;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "tools")]
#[command(about = "Blacksmith lab's tools")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    ConvertVideosToWav,
    ValidateInputDataForQdrant,
    UpsertW3ADataToQdrant,
    UpsertDataToQdrant,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    dotenv().ok();

    let cli = Cli::parse();
    info!("Blacksmith tools launched with command: {:?}", cli.command);

    let qdrant_client = Arc::new(
        Qdrant::from_url(&env::var("QDRANT_URL").expect("QDRANT_URL not set"))
            .api_key(env::var("QDRANT_API_KEY").expect("QDRANT_API_KEY not set"))
            .build()
            .unwrap(),
    );

    let llm_client = LLM_Client::new();

    match cli.command {
        Commands::ConvertVideosToWav => {
            if let Err(e) = convert_videos_to_wav() {
                error!("Error during conversion: {}", e);
                std::process::exit(1);
            }
        }
        Commands::ValidateInputDataForQdrant => {
            if let Err(e) = validate_input_data().await {
                error!("Error during validation: {}", e);
                std::process::exit(1);
            }
        }
        Commands::UpsertW3ADataToQdrant => {
            match upsert_w3a_data_to_qdrant(qdrant_client, llm_client).await {
                Ok(_) => {
                    info!("Data successfully uploaded to Qdrant.");
                }
                Err(e) => {
                    error!("Error during upsert data: {}", e);
                }
            }
            std::process::exit(1);
        }
        Commands::UpsertDataToQdrant => {
            match upsert_data_to_qdrant(qdrant_client, llm_client).await {
                Ok(_) => {
                    info!("Data successfully uploaded to Qdrant.");
                }
                Err(e) => {
                    error!("Error during upsert data: {}", e);
                }
            }
        }
    }
}

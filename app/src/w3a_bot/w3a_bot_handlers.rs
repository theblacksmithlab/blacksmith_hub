use anyhow::Result;
use tracing::info;

pub async fn w3a_bot_command_handler() -> Result<()> {
    info!("w3a_bot_command_handler >>>");
    Ok(())
}

pub async fn w3a_bot_message_handler() -> Result<()> {
    info!("w3a_bot_message_handler >>>");
    Ok(())
}

pub async fn w3a_bot_callback_query_handler() -> Result<()> {
    info!("w3a_bot_callback_query_handler >>>");
    Ok(())
}
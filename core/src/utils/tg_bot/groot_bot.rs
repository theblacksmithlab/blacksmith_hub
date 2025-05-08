use crate::models::common::app_name::AppName;
use crate::models::tg_bot::groot_bot::groot_bot::ChatObject;
use anyhow::{Context, Result};
use std::fs;
use tracing::info;
pub use crate::utils::common::build_resource_file_path;


pub fn load_chats_objects_from_file(app_name: &AppName) -> Result<Vec<ChatObject>> {
    let chats_path = build_resource_file_path(app_name, "chats_list.json");

    if !chats_path.exists() {
        return Err(anyhow::anyhow!(
            "Chats list file not found: {}",
            chats_path.display()
        ));
    }

    let data = fs::read_to_string(&chats_path)
        .with_context(|| format!("Failed to read chats list file: {}", chats_path.display()))?;

    let chats: Vec<ChatObject> = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse JSON in: {}", chats_path.display()))?;

    Ok(chats)
}

pub fn add_chat_to_file(app_name: &AppName, chat_object: ChatObject) -> Result<()> {
    let chats_path = build_resource_file_path(app_name, "chats_list.json");

    let mut chats: Vec<ChatObject> = if chats_path.exists() {
        let data = fs::read_to_string(&chats_path)
            .with_context(|| format!("Failed to read chats list file: {}", chats_path.display()))?;
        serde_json::from_str(&data).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };

    if chats.iter().any(|c| c.chat_id == chat_object.chat_id) {
        info!(
            "Chat: {} with id: {} already in the chats list. Continue",
            chat_object.username, chat_object.chat_id
        );
        return Ok(());
    }

    chats.push(chat_object.clone());

    let new_data = serde_json::to_string_pretty(&chats)
        .with_context(|| "Failed to serialize updated chat list")?;
    fs::write(&chats_path, new_data).with_context(|| {
        format!(
            "Failed to write updated chat list to file: {}",
            chats_path.display()
        )
    })?;

    info!(
        "New chat: {} with id: {} added to chats list file {}.",
        chat_object.username,
        chat_object.chat_id,
        chats_path.display()
    );

    Ok(())
}

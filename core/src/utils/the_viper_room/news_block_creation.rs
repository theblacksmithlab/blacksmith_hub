use crate::ai::common::common::raw_llm_processing;
use crate::ai::common::voice_processing::podcast_tts;
use crate::models::common::app_name::AppName;
use crate::models::common::system_messages::AppsSystemMessages;
use crate::models::common::system_messages::TheViperRoomBotMessages;
use crate::models::common::system_roles::TheViperRoomRoleType;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::get_system_role_or_fallback;
use crate::utils::common::get_message;
use crate::utils::the_viper_room::news_block_creation_utils::{
    get_dialogs, mix_podcast_with_music, processing_dialogs, summarize_updates,
    updates_file_creation,
};
use grammers_client::Client as g_Client;
use std::fs;
use std::fs::{create_dir_all, read_dir, remove_file, rename};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};
use crate::models::common::ai::LlmModel;

pub async fn news_block_creation<T: OpenAIClientInit + Send + Sync>(
    client: &g_Client,
    user_id: &str,
    app_state: Arc<T>,
    nickname: String,
    need_caption: bool,
) -> anyhow::Result<PathBuf> {
    let user_tmp_dir = format!("common_res/the_viper_room/tmp/{}", user_id);
    create_dir_all(&user_tmp_dir)?;

    let channels = get_dialogs(&client).await?;

    processing_dialogs(&client, channels, app_state.clone(), user_tmp_dir.clone()).await?;

    updates_file_creation(user_tmp_dir.clone(), app_state.clone()).await?;

    let podcast_text = summarize_updates(user_tmp_dir.clone(), app_state.clone(), nickname).await?;

    let audio_path = podcast_tts(
        podcast_text.clone(),
        user_tmp_dir.clone(),
        app_state.clone(),
    )
    .await?;

    info!("Starting to add background music to the podcast...");
    let background_music_path = "common_res/the_viper_room/background_music.mp3";
    let mixed_audio_path = audio_path.with_file_name(format!(
        "{}_mixed.mp3",
        audio_path.file_stem().unwrap().to_string_lossy()
    ));

    match mix_podcast_with_music(
        audio_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid audio path"))?,
        background_music_path,
        mixed_audio_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid mixed audio path"))?,
    )
    .await
    {
        Ok(_) => {
            remove_file(&audio_path)?;
            rename(&mixed_audio_path, &audio_path)?;
            info!("Background music added successfully");
        }
        Err(e) => {
            error!(
                "Failed to add background music: {}. Continuing with original audio",
                e
            );
            if mixed_audio_path.exists() {
                let _ = remove_file(&mixed_audio_path);
            }
        }
    }

    let txt_files: Vec<_> = read_dir(&user_tmp_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "txt"))
        .map(|entry| entry.path())
        .collect();

    for file_path in &txt_files {
        remove_file(file_path)?;
        info!("File {} has been deleted.", file_path.display());
    }

    if need_caption {
        let system_role = get_system_role_or_fallback(
            &AppName::TheViperRoom,
            TheViperRoomRoleType::CaptionGeneration,
            None,
        );

        let mut caption = raw_llm_processing(
            &system_role,
            &podcast_text,
            app_state.clone(),
            LlmModel::Light,
        )
        .await?;
        caption.push_str(
            &get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::DonationFooter,
            ))
            .await?,
        );

        let caption_path = audio_path.with_extension("txt");
        fs::write(caption_path, caption)?;
    }

    Ok(audio_path)
}

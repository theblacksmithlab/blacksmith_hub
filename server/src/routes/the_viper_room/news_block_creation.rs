use crate::routes::the_viper_room::news_block_creation_utils::{
    get_dialogs, mix_podcast_with_music, processing_dialogs, summarize_updates,
    updates_file_creation,
};
use core::ai::utils::{raw_llm_processing, text_to_speech};
use core::state::llm_processing_trait::LlmProcessing;
use core::utils::common::{get_message, LlmModel};
use grammers_client::Client as g_Client;
use std::fs;
use std::fs::{create_dir_all, read_dir, read_to_string, remove_file, rename};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info};

pub async fn news_block_creation<T: LlmProcessing + Send + Sync>(
    client: &g_Client,
    user_id: u64,
    app_state: Arc<T>,
    nickname: String,
    need_caption: bool,
) -> anyhow::Result<PathBuf> {
    let user_tmp_dir = format!("common_res/the_viper_room/tmp/{}", user_id);
    create_dir_all(&user_tmp_dir)?;
    let ai_utils_dir = Path::new("common_res/the_viper_room/ai_utils/");

    let channels = get_dialogs(&client).await?;

    processing_dialogs(
        &client,
        channels,
        app_state.clone(),
        user_tmp_dir.clone(),
        ai_utils_dir,
    )
    .await?;

    updates_file_creation(user_tmp_dir.clone(), app_state.clone(), ai_utils_dir).await?;

    let podcast_text = summarize_updates(
        user_tmp_dir.clone(),
        app_state.clone(),
        nickname,
        ai_utils_dir,
    )
    .await?;

    let audio_path = text_to_speech(
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
        let system_role_path = ai_utils_dir.join("system_role_caption_generation.txt");
        let system_role = read_to_string(&system_role_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read 'system role for podcast caption generation': {}",
                e
            )
        })?;

        let mut caption = raw_llm_processing(
            system_role,
            podcast_text,
            app_state.clone(),
            LlmModel::Light,
        )
        .await?;
        caption.push_str(&get_message("the_viper_room", "donation_footer").await?);

        let caption_path = audio_path.with_extension("txt");
        fs::write(caption_path, caption)?;
    }

    Ok(audio_path)
}

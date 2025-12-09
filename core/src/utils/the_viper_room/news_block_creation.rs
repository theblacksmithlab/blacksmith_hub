use crate::ai::common::common::raw_llm_processing;
use crate::ai::common::voice_processing::{
    podcast_tts_via_elevenlabs, podcast_tts_via_google, podcast_tts_via_openai,
};
use crate::local_db::the_viper_room::user_management::get_user_nickname;
use crate::models::common::ai::LlmModel;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::TheViperRoomRoleType;
use crate::models::the_viper_room::common::TTSProvider;
use crate::models::the_viper_room::db_models::Recipient;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::get_system_role_or_fallback;
use crate::utils::the_viper_room::news_block_creation_utils::{
    get_dialogs, get_user_dialogs_from_db, mix_podcast_with_music, processing_chats,
    processing_dialogs, summarize_updates, updates_file_creation,
};
use grammers_client::Client as g_Client;
use sqlx::{Pool, Sqlite};
use std::fs;
use std::fs::{create_dir_all, read_dir, remove_file, rename};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::log::warn;
use tracing::{error, info};

pub async fn news_block_creation<T: OpenAIClientInit + Send + Sync>(
    client: &g_Client,
    user_id: &str,
    app_state: Arc<T>,
    recipient: Recipient,
    need_caption: bool,
    db_pool: Option<&Pool<Sqlite>>,
) -> anyhow::Result<PathBuf> {
    let user_tmp_dir = format!("common_res/the_viper_room/tmp/{}", user_id);
    create_dir_all(&user_tmp_dir)?;

    match &recipient {
        Recipient::Public => {
            info!("Fetching channels for public podcast from agent subscriptions");
            let channels = get_dialogs(&client).await?;

            info!(
                "Processing {} channels for podcast generation",
                channels.len()
            );
            processing_dialogs(&client, channels, app_state.clone(), user_tmp_dir.clone()).await?;
        }
        Recipient::Private(user_id) => {
            info!("Fetching channels for user {} from database", user_id);
            if let Some(pool) = db_pool {
                let chats = get_user_dialogs_from_db(&client, *user_id, pool).await?;

                info!("Processing {} channels for podcast generation", chats.len());
                processing_chats(&client, chats, app_state.clone(), user_tmp_dir.clone()).await?;
            } else {
                return Err(anyhow::anyhow!(
                    "Database pool is required for private podcast generation"
                ));
            }
        }
    };

    let addressee = match recipient {
        Recipient::Public => "Public".to_string(),
        Recipient::Private(user_id) => {
            if let Some(pool) = db_pool {
                match get_user_nickname(pool, user_id).await {
                    Ok(Some(nickname)) => {
                        info!("Using nickname for user {}: {}", user_id, nickname);
                        nickname
                    }
                    Ok(None) => {
                        warn!("No nickname found for user {}, using 'Друг'", user_id);
                        "Друг".to_string()
                    }
                    Err(e) => {
                        warn!(
                            "Error fetching nickname for user {}: {}. Using 'Друг'",
                            user_id, e
                        );
                        "Друг".to_string()
                    }
                }
            } else {
                warn!("No database pool provided, using 'Друг' as default");
                "Друг".to_string()
            }
        }
    };

    updates_file_creation(user_tmp_dir.clone(), app_state.clone()).await?;

    let podcast_text =
        summarize_updates(user_tmp_dir.clone(), app_state.clone(), &addressee).await?;

    let tts_provider = TTSProvider::Google;

    let audio_path = match tts_provider {
        TTSProvider::OpenAI => {
            let audio_path = podcast_tts_via_openai(
                podcast_text.clone(),
                user_tmp_dir.clone(),
                app_state.clone(),
            )
            .await?;
            audio_path
        }
        TTSProvider::ElevenLabs => {
            let audio_path =
                podcast_tts_via_elevenlabs(podcast_text.clone(), user_tmp_dir.clone()).await?;
            audio_path
        }
        TTSProvider::Google => {
            let audio_path =
                podcast_tts_via_google(podcast_text.clone(), user_tmp_dir.clone()).await?;
            audio_path
        }
    };

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

        let data_for_caption = format!(
            "Адресат: {}\nТекст эпизода подкаста: {}",
            addressee, podcast_text
        );

        let caption = raw_llm_processing(
            &system_role,
            &data_for_caption,
            app_state.clone(),
            LlmModel::Light,
        )
        .await?;

        // caption.push_str(
        //     &get_message(AppsSystemMessages::TheViperRoomBot(
        //         TheViperRoomBotMessages::DonationFooter,
        //     ))
        //     .await?
        // );

        let caption_path = audio_path.with_extension("txt");
        fs::write(caption_path, caption)?;
    }

    Ok(audio_path)
}

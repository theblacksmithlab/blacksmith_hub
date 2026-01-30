use crate::ai::common::openai::raw_openai_processing;
use crate::ai::common::voice_processing::{
    generate_single_part_via_elevenlabs, generate_single_part_via_openai,
};
use crate::local_db::the_viper_room::user_management::get_user_nickname;
use crate::models::common::ai::OpenAIModel;
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
use crate::utils::the_viper_room::podcast_voiceover::{
    generate_parts_batched_google, merge_audio_parts,
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
            processing_dialogs(
                &client,
                channels,
                app_state.clone(),
                user_tmp_dir.clone(),
                recipient,
            )
            .await?;
        }
        Recipient::Private(user_id) => {
            info!("Fetching channels for user {} from database", user_id);
            if let Some(pool) = db_pool {
                let chats = get_user_dialogs_from_db(&client, *user_id, pool).await?;

                info!("Processing {} channels for podcast generation", chats.len());
                processing_chats(
                    &client,
                    chats,
                    app_state.clone(),
                    user_tmp_dir.clone(),
                    recipient,
                )
                .await?;
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

    let podcast_structure =
        summarize_updates(user_tmp_dir.clone(), app_state.clone(), &addressee).await?;

    // Group body parts to reduce voice inconsistencies between TTS requests
    const BODY_GROUP_SIZE: usize = 3; // Group 3 news items per TTS request

    let mut parts_to_voice: Vec<String> = Vec::new();

    // Intro separately
    parts_to_voice.push(podcast_structure.intro.clone());

    // Group body parts (5 news per group, joined with double newline for natural pauses)
    for chunk in podcast_structure.body.chunks(BODY_GROUP_SIZE) {
        let grouped_text = chunk.join("\n\n");
        parts_to_voice.push(grouped_text);
    }

    // Outro separately
    parts_to_voice.push(podcast_structure.outro.clone());

    let body_groups = (podcast_structure.body.len() + BODY_GROUP_SIZE - 1) / BODY_GROUP_SIZE;
    info!(
        "Podcast structure: 1 intro + {} body groups (from {} news, {} per group) + 1 outro = {} total parts to voice",
        body_groups,
        podcast_structure.body.len(),
        BODY_GROUP_SIZE,
        parts_to_voice.len()
    );

    let tts_provider = TTSProvider::Google;

    let audio_parts = if tts_provider == TTSProvider::Google {
        info!("Using batched TTS generation for Google (9 parallel requests per batch)");
        generate_parts_batched_google(&parts_to_voice, &user_tmp_dir).await?
    } else {
        info!(
            "Using sequential TTS generation for {}",
            if tts_provider == TTSProvider::OpenAI {
                "OpenAI"
            } else {
                "ElevenLabs"
            }
        );
        let mut audio_parts: Vec<PathBuf> = Vec::new();
        for (i, part) in parts_to_voice.iter().enumerate() {
            info!(
                "Voicing part {}/{}: {} chars",
                i + 1,
                parts_to_voice.len(),
                part.chars().count()
            );

            let part_audio = match tts_provider {
                TTSProvider::OpenAI => {
                    generate_single_part_via_openai(part, &user_tmp_dir, i, app_state.clone())
                        .await?
                }
                TTSProvider::ElevenLabs => {
                    generate_single_part_via_elevenlabs(part, &user_tmp_dir, i).await?
                }
                // Google TTS resolved earlier
                TTSProvider::Google => unreachable!(),
            };

            audio_parts.push(part_audio);

            if i < parts_to_voice.len() - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }
        }
        audio_parts
    };

    let now = chrono::Utc::now();
    let utc_plus_3 = now + chrono::Duration::hours(3);
    let date_only = utc_plus_3.date_naive();
    let final_filename = format!("The_Viper_Podcast_({})", date_only);

    let audio_path = merge_audio_parts(audio_parts, &user_tmp_dir, &final_filename).await?;

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

        let full_podcast_text = format!(
            "{}\n\n{}\n\n{}",
            podcast_structure.intro,
            podcast_structure.body.join("\n\n"),
            podcast_structure.outro
        );

        let data_for_caption = format!(
            "Адресат: {}\nТекст эпизода подкаста: {}",
            addressee, full_podcast_text
        );

        let caption = raw_openai_processing(
            &system_role,
            &data_for_caption,
            app_state.clone(),
            OpenAIModel::GPT4o,
        )
        .await?;

        // // Donation footer disabled for now
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

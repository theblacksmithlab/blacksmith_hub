use crate::ai::common::common::{raw_llm_processing, raw_llm_processing_json};
use crate::local_db::the_viper_room::channel_management::get_user_channels;
use crate::models::common::ai::LlmModel;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::TheViperRoomRoleType;
use crate::models::the_viper_room::common::PodcastStructure;
use crate::models::the_viper_room::db_models::Recipient;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::get_system_role_or_fallback;
use chrono::Duration as ChronoDuration;
use chrono::Utc;
use grammers_client::types::Chat::{Channel, Group, User};
use grammers_client::{types, Client as g_Client};
use sqlx::{Pool, Sqlite};
use std::fs;
use std::fs::{copy, read_dir, remove_file, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tracing::info;
use tracing::log::warn;

pub(crate) async fn get_dialogs(client: &g_Client) -> Result<Vec<types::Dialog>, anyhow::Error> {
    info!("Getting list of updates sources...");

    let mut dialogs = client.iter_dialogs();

    // // In case there is a need to process updates from group chats
    // let mut groups = Vec::new();

    // // In case there is a need to process updates from private chats
    // let mut private_chats = Vec::new();

    // Processing only public channel's updates by default
    let mut channels = Vec::new();

    while let Some(dialog) = dialogs.next().await? {
        match dialog.chat() {
            // deactivated by default
            Group(_group) => {
                // // In case there is a need to process updates from group chats
                // groups.push(dialog.clone());
                // info!("Group: {} (ID: {})", group.title(), group.id());
            }
            // deactivated by default
            User(_user) => {
                // // In case there is a need to process updates from private chats
                // private_chats.push(dialog.clone());
                // info!("Private chat: {} (ID: {})", user.first_name(), user.id());
            }
            Channel(channel) => {
                channels.push(dialog.clone());
                info!(
                    "Channel: {} (ID: {}) pushed to channels list...",
                    channel.title(),
                    channel.id()
                );
            }
        }
    }
    Ok(channels)
}

pub(crate) async fn get_user_dialogs_from_db(
    client: &g_Client,
    user_id: u64,
    db_pool: &Pool<Sqlite>,
) -> Result<Vec<types::Chat>, anyhow::Error> {
    info!("Getting user {} channels from database...", user_id);

    let user_channels = get_user_channels(db_pool, user_id).await?;

    if user_channels.is_empty() {
        info!("User {} has no channels in database", user_id);
        return Ok(Vec::new());
    }

    info!(
        "Found {} channels for user {} in database",
        user_channels.len(),
        user_id
    );

    let mut chats = Vec::new();

    for user_channel in &user_channels {
        let username = &user_channel.channel_username;

        info!(
            "Resolving channel: {} (@{})",
            user_channel.channel_title, username
        );

        match client.resolve_username(username).await {
            Ok(Some(chat)) => {
                if let Channel(_channel) = &chat {
                    chats.push(chat);
                    info!(
                        "Channel {} (@{}) resolved successfully",
                        user_channel.channel_title, username
                    );
                } else {
                    warn!(
                        "Chat @{} is not a channel ({}), skipping",
                        username, user_channel.channel_title
                    );
                }
            }
            Ok(None) => {
                warn!(
                    "Channel @{} ({}) not found or not accessible",
                    username, user_channel.channel_title
                );
            }
            Err(e) => {
                warn!(
                    "Failed to resolve channel @{} ({}): {}",
                    username, user_channel.channel_title, e
                );
            }
        }
    }

    info!(
        "Successfully resolved {}/{} channels for user {}",
        chats.len(),
        user_channels.len(),
        user_id
    );

    Ok(chats)
}

pub(crate) async fn processing_dialogs<T: OpenAIClientInit + Send + Sync>(
    client: &g_Client,
    channels: Vec<types::Dialog>,
    app_state: Arc<T>,
    user_tmp_dir: String,
) -> Result<(), anyhow::Error> {
    // info!("\nReceiving updates from each group...\n");
    // for dialog in groups {
    //     if let types::Chat::Group(group) = dialog.chat() {
    //         let group_name = group.title();
    //         info!("\nGroup: {}\n", group_name);
    //         get_latest_messages(client, dialog.clone(), &group_name, msg.clone()).await?;
    //         sleep(Duration_2::from_secs(2)).await;
    //     }
    // }

    // info!("\nReceiving updates from each private chat...");
    // for dialog in private_chats {
    //     if let types::Chat::User(user) = dialog.chat() {
    //         let user_name = match (user.first_name(), user.last_name()) {
    //             (first, Some(last)) => format!("{} {}", first, last),
    //             (first, None) => first.to_string(),
    //         };
    //         info!("\nPrivate chat: {}\n", user_name);
    //         get_latest_messages(client, dialog.clone(), &user_name, msg.clone()).await?;
    //         sleep(Duration_2::from_secs(2)).await;
    //     }
    // }

    for dialog in channels {
        if let Channel(channel) = dialog.chat() {
            let channel_name = channel.title();
            info!("Receiving updates from channel: {}...", channel_name);
            get_latest_messages(
                client,
                dialog.chat(),
                &channel_name,
                app_state.clone(),
                user_tmp_dir.clone(),
            )
            .await?;
        }
    }

    Ok(())
}

pub(crate) async fn processing_chats<T: OpenAIClientInit + Send + Sync>(
    client: &g_Client,
    chats: Vec<types::Chat>,
    app_state: Arc<T>,
    user_tmp_dir: String,
) -> Result<(), anyhow::Error> {
    for chat in &chats {
        if let Channel(channel) = chat {
            let channel_name = channel.title();
            info!("Receiving updates from channel: {}...", channel_name);
            get_latest_messages(
                client,
                chat,
                &channel_name,
                app_state.clone(),
                user_tmp_dir.clone(),
            )
            .await?;
        }
    }

    Ok(())
}

pub(crate) async fn updates_file_creation<T: OpenAIClientInit + Send + Sync>(
    user_tmp_dir: String,
    app_state: Arc<T>,
) -> Result<(), anyhow::Error> {
    info!("Writing updates from information sources in updates.txt...");

    let updates_file_path = format!("{}/updates.txt", user_tmp_dir);
    let mut updates_file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(updates_file_path.clone())?;

    remove_empty_txt_files(user_tmp_dir.clone()).await?;

    let txt_files: Vec<_> = read_dir(&user_tmp_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "txt"))
        .filter(|entry| entry.path() != Path::new(&updates_file_path))
        .map(|entry| entry.path())
        .collect();

    let now = Utc::now();
    let utc_plus_3_now = now + ChronoDuration::hours(3);
    let utc_plus_3_start = utc_plus_3_now - ChronoDuration::hours(9);

    writeln!(
        updates_file,
        "Список обновлений за период: с {} по {} (UTC+3)\n\
        Текущее время: {}\n",
        utc_plus_3_start.format("%H:%M %d.%m.%Y"),
        utc_plus_3_now.format("%H:%M %d.%m.%Y"),
        utc_plus_3_now.format("%H:%M %d.%m.%Y")
    )?;

    writeln!(updates_file, "\n===НАЧАЛО ОБНОВЛЕНИЙ===\n\n")?;

    let system_role = get_system_role_or_fallback(
        &AppName::TheViperRoom,
        TheViperRoomRoleType::ExtractingNews,
        None,
    );

    for file_path in txt_files.clone() {
        let content = read_file_safe(&file_path)?;
        let (source, messages) = parse_channel_file(&content)?;

        if messages.is_empty() {
            info!("No messages found in {}, skipping", file_path.display());
            continue;
        }

        info!(
            "Processing {} messages from source: {}",
            messages.len(),
            source
        );

        for (idx, message_text) in messages.iter().enumerate() {
            let llm_input = format!(
                "Источник обновления: {}\nТекст обновления:\n{}",
                source, message_text
            );

            let response = raw_llm_processing(
                &system_role,
                &llm_input,
                app_state.clone(),
                LlmModel::ComplexFast,
            )
            .await?;

            let trimmed = response.trim();
            if !trimmed.is_empty() {
                writeln!(
                    updates_file,
                    "Источник обновления: {}\nОбзор обновления:\n{}\n",
                    source, trimmed,
                )?;
            } else {
                warn!("Empty response for message {} from {}", idx + 1, source);
            }
        }

        info!("{} processed successfully!", file_path.display());
    }

    writeln!(updates_file, "\n===КОНЕЦ ОБНОВЛЕНИЙ===")?;

    for file_path in &txt_files {
        remove_file(file_path)?;
        info!("{} file has been deleted.", file_path.display());
    }

    info!("Updates file created successfully! Temporary files deleted");

    Ok(())
}

pub(crate) async fn summarize_updates<T: OpenAIClientInit + Send + Sync>(
    user_tmp_dir: String,
    app_state: Arc<T>,
    addressee: &str,
) -> Result<PodcastStructure, anyhow::Error> {
    info!("Starting updates summarization...");

    let system_role = get_system_role_or_fallback(
        &AppName::TheViperRoom,
        TheViperRoomRoleType::CreatingPodcast,
        None,
    );

    let updates = read_file_safe(format!("{}/updates.txt", user_tmp_dir))
        .map_err(|e| format!("Failed to read 'updates': {}", e))
        .unwrap();

    let updates_with_nickname_provided = format!(
        "Адресат: {}\nТекст подкаста подготовленный твоим помощником: {}",
        addressee, updates
    );

    let updates_summarized_json = raw_llm_processing_json(
        &system_role,
        &updates_with_nickname_provided,
        app_state.clone(),
        LlmModel::ComplexFast,
    )
    .await?;

    info!("Received JSON response from LLM, parsing...");

    let podcast_structure: PodcastStructure = serde_json::from_str(&updates_summarized_json)
        .map_err(|e| anyhow::anyhow!("Failed to parse podcast structure from JSON: {}", e))?;

    // // Save for debugging
    // let updates_summarized_file_path = format!("{}/updates_summarized.json", user_tmp_dir);
    // let mut updates_summarized_file = OpenOptions::new()
    //     .create(true)
    //     .write(true)
    //     .truncate(true)
    //     .open(updates_summarized_file_path.clone())?;
    // writeln!(updates_summarized_file, "{}", updates_summarized_json)?;

    info!(
        "Podcast structure parsed successfully: {} intro, {} body parts, {} outro",
        podcast_structure.intro.len(),
        podcast_structure.body.len(),
        podcast_structure.outro.len()
    );

    Ok(podcast_structure)
}

pub(crate) async fn get_latest_messages<T: OpenAIClientInit + Send + Sync>(
    client: &g_Client,
    chat: &types::Chat,
    chat_name: &str,
    app_state: Arc<T>,
    user_tmp_dir: String,
) -> anyhow::Result<()> {
    let mut messages = client.iter_messages(chat);
    let now = Utc::now();
    let period = now - chrono::Duration::hours(12);

    let user_tmp_file = format!(
        "{}/{}.txt",
        user_tmp_dir,
        chat_name.replace(" ", "_").replace("/", "_")
    );

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(user_tmp_file)?;

    writeln!(file, "ИСТОЧНИК ОБНОВЛЕНИЙ: {}\n", chat_name)?;

    let system_role = get_system_role_or_fallback(
        &AppName::TheViperRoom,
        TheViperRoomRoleType::CheckUsefulness,
        None,
    );

    while let Some(message) = messages.next().await? {
        if message.date() < period {
            break;
        }
        if !message.text().is_empty() {
            let text = message.text();

            let llm_response =
                raw_llm_processing(&system_role, text, app_state.clone(), LlmModel::Light).await?;

            let decision = llm_response.trim().to_lowercase();

            info!("LLM usefulness decision: {}", decision);

            let contains_ok = decision.contains("ok");
            let contains_skip = decision.contains("skip");

            if !contains_ok && !contains_skip {
                warn!(
                    "Unexpected LLM response: '{}', defaulting to skip",
                    llm_response
                );
                continue;
            }

            if contains_skip {
                continue;
            }

            writeln!(file, "===ТЕКСТ ОБНОВЛЕНИЯ===\n{}\n===КОНЕЦ===\n", text)?;
        }
    }

    Ok(())
}

async fn remove_empty_txt_files(dir: String) -> anyhow::Result<()> {
    for entry in read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(file_name) = path.file_name() {
            if file_name == "updates.txt" {
                continue;
            }
        }

        if path.extension().and_then(|ext| ext.to_str()) == Some("txt") {
            let metadata = fs::metadata(&path)?;

            if metadata.len() == 0 {
                remove_file(&path)?;
                info!("Deleted empty file: {:?}", path);
            }
        }
    }
    Ok(())
}

pub fn read_file_safe(path: impl AsRef<Path>) -> anyhow::Result<String, anyhow::Error> {
    let bytes = fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

async fn get_duration(file_path: &str) -> anyhow::Result<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            file_path,
        ])
        .output()
        .await?;

    let duration_str = String::from_utf8(output.stdout)?;
    let duration = duration_str
        .trim()
        .parse::<f64>()
        .map_err(|e| anyhow::anyhow!("Failed to parse duration: {}", e))?;

    if duration <= 0.0 {
        return Err(anyhow::anyhow!("Invalid duration: {}", duration));
    }

    Ok(duration)
}

pub(crate) async fn equalize_voice_for_broadcast(input_path: &str) -> anyhow::Result<String> {
    if !Path::new(input_path).exists() {
        return Err(anyhow::anyhow!(
            "Input audio file not found: {}",
            input_path
        ));
    }

    info!("Applying professional voice equalization...");

    let input_path_buf = PathBuf::from(input_path);
    let parent = input_path_buf.parent().unwrap_or(Path::new("."));
    let file_stem = input_path_buf.file_stem().unwrap_or_default();
    let equalized_path = parent.join(format!("{}_eq.mp3", file_stem.to_string_lossy()));
    let equalized_path_str = equalized_path.to_str().unwrap();

    let audio_filter = "highpass=f=80,\
                        equalizer=f=200:width_type=o:width=2:g=-2,\
                        equalizer=f=3000:width_type=o:width=2:g=4,\
                        acompressor=threshold=-18dB:ratio=4:attack=5:release=50,\
                        alimiter=limit=0.9";

    let output = Command::new("ffmpeg")
        .args([
            "-i",
            input_path,
            "-filter:a",
            audio_filter,
            "-codec:a",
            "libmp3lame",
            "-q:a",
            "0",
            "-y",
            equalized_path_str,
        ])
        .output()
        .await?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("ffmpeg equalization error: {}", error));
    }

    info!("Voice equalization completed: {}", equalized_path_str);
    Ok(equalized_path_str.to_string())
}

pub(crate) async fn mix_podcast_with_music(
    podcast_path: &str,
    music_path: &str,
    output_path: &str,
) -> anyhow::Result<()> {
    if !Path::new(podcast_path).exists() {
        return Err(anyhow::anyhow!("Podcast file not found: {}", podcast_path));
    }
    if !Path::new(music_path).exists() {
        return Err(anyhow::anyhow!("Music file not found: {}", music_path));
    }

    info!("Step 1: Equalizing voice...");
    let equalized_podcast_path = equalize_voice_for_broadcast(podcast_path).await?;

    info!("Step 2: Getting podcast duration...");
    let podcast_duration = get_duration(&equalized_podcast_path).await?;
    info!("Podcast duration: {} seconds", podcast_duration);

    let fade_start = podcast_duration - 4.0;

    // Side-chain compression: Music ducks when voice is present
    // 1. Loop music and apply fade-out
    // 2. Apply side-chain compression (music compressed by voice level)
    // 3. Mix voice and compressed music
    let filter_complex = format!(
        "[1:a]aloop=loop=-1:size=2e+09,volume=0.08,afade=t=out:st={}:d=4[music_looped];\
         [music_looped][0:a]sidechaincompress=threshold=0.02:ratio=4:attack=10:release=500:makeup=2[music_ducked];\
         [0:a][music_ducked]amix=inputs=2:duration=first:weights=1.0 0.7",
        fade_start
    );

    info!("Step 3: Mixing equalized voice with music...");
    let output = Command::new("ffmpeg")
        .args([
            "-i",
            &equalized_podcast_path,
            "-i",
            music_path,
            "-filter_complex",
            &filter_complex,
            "-codec:a",
            "libmp3lame",
            "-q:a",
            "0",
            "-y",
            output_path,
        ])
        .output()
        .await?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        let _ = remove_file(&equalized_podcast_path);
        return Err(anyhow::anyhow!("ffmpeg mixing error: {}", error));
    }

    if let Err(e) = remove_file(&equalized_podcast_path) {
        warn!(
            "Could not delete temporary equalized file {}: {}",
            equalized_podcast_path, e
        );
    } else {
        info!(
            "Temporary equalized file deleted: {}",
            equalized_podcast_path
        );
    }

    info!("Mixing completed successfully");
    Ok(())
}

pub async fn generate_waveform(audio_path: &Path) -> anyhow::Result<Vec<u8>> {
    let output = Command::new("ffmpeg")
        .args([
            "-i",
            audio_path.to_str().unwrap(),
            "-filter:a",
            "aformat=channel_layouts=mono,compand=gain=-6",
            "-map",
            "0:a",
            "-c:a",
            "pcm_s16le",
            "-f",
            "data",
            "-",
        ])
        .output()
        .await?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("ffmpeg error: {}", error));
    }

    let samples: Vec<i16> = output
        .stdout
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    let chunk_size = samples.len() / 100;
    let mut waveform = Vec::with_capacity(100);

    for chunk in samples.chunks(chunk_size) {
        if chunk.is_empty() {
            continue;
        }

        let max_amplitude = chunk
            .iter()
            .map(|&s| s.abs() as f32 / i16::MAX as f32)
            .fold(0f32, f32::max);

        let value = (max_amplitude * 31.0) as u8;
        waveform.push(value.min(31));
    }

    Ok(waveform)
}

pub async fn save_daily_podcast(
    podcast_path: &PathBuf,
    caption_path: &PathBuf,
    recipient: Recipient,
) -> anyhow::Result<()> {
    let daily_podcast_dir = match recipient {
        Recipient::Public => "common_res/the_viper_room/daily_public_podcast".to_string(),
        Recipient::Private(user_id) => {
            format!(
                "common_res/the_viper_room/{}/daily_private_podcast",
                user_id
            )
        }
    };

    fs::create_dir_all(&daily_podcast_dir)?;

    info!(
        "Cleaning old daily podcast files in {}...",
        daily_podcast_dir
    );
    for entry in read_dir(&daily_podcast_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            remove_file(&path)?;
            info!("Removed old file: {:?}", path);
        }
    }

    info!("Saving new daily podcast...");

    let podcast_filename = podcast_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid podcast path"))?;
    let caption_filename = caption_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid caption path"))?;

    let dest_podcast = PathBuf::from(&daily_podcast_dir).join(podcast_filename);
    let dest_caption = PathBuf::from(&daily_podcast_dir).join(caption_filename);

    copy(podcast_path, &dest_podcast)?;
    copy(caption_path, &dest_caption)?;

    info!("Daily podcast saved to: {:?}", dest_podcast);
    info!("Daily caption saved to: {:?}", dest_caption);

    Ok(())
}

fn parse_channel_file(content: &str) -> Result<(String, Vec<String>), anyhow::Error> {
    let mut lines = content.lines();

    let source = lines
        .find(|line| line.starts_with("ИСТОЧНИК ОБНОВЛЕНИЙ:"))
        .map(|line| {
            line.trim_start_matches("ИСТОЧНИК ОБНОВЛЕНИЙ:")
                .trim()
                .to_string()
        })
        .unwrap_or_else(|| {
            warn!("Source not found in file, using default");
            "Не определено".to_string()
        });

    let remaining_content: String = lines.collect::<Vec<_>>().join("\n");

    let messages: Vec<String> = remaining_content
        .split("===ТЕКСТ ОБНОВЛЕНИЯ===")
        .filter_map(|part| {
            let text = part.replace("===КОНЕЦ===", "").trim().to_string();

            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        })
        .collect();

    Ok((source, messages))
}

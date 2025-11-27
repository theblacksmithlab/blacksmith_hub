use crate::ai::common::common::raw_llm_processing;
use crate::models::common::ai::LlmModel;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::TheViperRoomRoleType;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::get_system_role_or_fallback;
use chrono::Duration as ChronoDuration;
use chrono::Utc;
use grammers_client::types::Chat::{Channel, Group, User};
use grammers_client::{types, Client as g_Client};
use std::fs;
use std::fs::{read_dir, remove_file, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;
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
                dialog.clone(),
                &channel_name,
                app_state.clone(),
                user_tmp_dir.clone(),
            )
            .await?;
            // Check for FLOOD_WAIT
            // sleep(Duration::from_secs(1)).await;
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

        info!("Processing {} messages from source: {}", messages.len(), source);

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
            ).await?;

            let trimmed = response.trim();
            if !trimmed.is_empty() {
                writeln!(
                    updates_file,
                    "Источник обновления: {}\nОбзор обновления:\n{}\n",
                    source,
                    trimmed,
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
    nickname: String,
) -> Result<String, anyhow::Error> {
    info!("Starting updates summarization...");

    let system_role = get_system_role_or_fallback(
        &AppName::TheViperRoom,
        TheViperRoomRoleType::CreatingPodcast,
        None,
    );

    let updates = read_file_safe(format!("{}/updates.txt", user_tmp_dir))
        .map_err(|e| format!("Failed to read 'updates': {}", e))
        .unwrap();

    let updates_with_nickname_provided =
        format!("Адресат: {}\nТекст подкаста подготовленный твоим помощником: {}", nickname, updates);

    let updates_summarized = raw_llm_processing(
        &system_role,
        &updates_with_nickname_provided,
        app_state.clone(),
        LlmModel::ComplexFast,
    )
    .await?;

    let updates_summarized_file_path = format!("{}/updates_summarized.txt", user_tmp_dir);

    let mut updates_summarized_file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(updates_summarized_file_path.clone())?;

    writeln!(updates_summarized_file, "{}", updates_summarized)?;

    info!("Podcast text file created successfully!");

    Ok(updates_summarized)
}

pub(crate) async fn get_latest_messages<T: OpenAIClientInit + Send + Sync>(
    client: &g_Client,
    dialog: types::Dialog,
    chat_name: &str,
    app_state: Arc<T>,
    user_tmp_dir: String,
) -> anyhow::Result<()> {
    let mut messages = client.iter_messages(dialog.chat());
    let now = Utc::now();
    let period = now - chrono::Duration::hours(9); // TODO: Implement news parsing period setting from UI

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
                raw_llm_processing(
                    &system_role,
                    text,
                    app_state.clone(),
                    LlmModel::Light,
                ).await?;

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

            writeln!(
                file,
                "===ТЕКСТ ОБНОВЛЕНИЯ===\n{}\n===КОНЕЦ===\n",
                text
            )?;
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

    info!("Getting podcast duration...");
    let duration = get_duration(podcast_path).await?;
    info!("Podcast duration: {} seconds", duration);

    let fade_start = duration - 4.0;

    let filter_complex = format!(
        "[1:a]volume=0.36,afade=t=out:st={}:d=4[music];[0:a][music]amix=inputs=2:duration=first",
        fade_start
    );

    info!("Starting ffmpeg mixing process...");
    let output = Command::new("ffmpeg")
        .args([
            "-i",
            podcast_path,
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
        return Err(anyhow::anyhow!("ffmpeg error: {}", error));
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
            let text = part
                .replace("===КОНЕЦ===", "")
                .trim()
                .to_string();

            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        })
        .collect();

    Ok((source, messages))
}

use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::split_text_into_chunks;
use anyhow::anyhow;
use async_openai::types::{CreateSpeechRequestArgs, CreateSpeechResponse, SpeechModel, Voice};
use chrono::{Duration, Utc};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};
use reqwest::Client as ReqwestClient;
use serde_json::json;

pub async fn podcast_tts_via_openai<T: OpenAIClientInit + Send + Sync>(
    text: String,
    user_tmp_dir: String,
    app_state: Arc<T>,
) -> anyhow::Result<PathBuf> {
    info!("Starting recording podcast via OpenAI TTS model...");

    let now = Utc::now();
    let utc_plus_3 = now + Duration::hours(3);
    let date_only = utc_plus_3.date_naive();
    let file_name = format!("The_Viper_Podcast_({})", date_only);

    let llm_client = app_state.get_llm_client().clone();

    const MAX_TTS_CHARS: usize = 4095;

    let char_count = text.chars().count();

    if char_count <= MAX_TTS_CHARS {
        let char_count = text.chars().count();
        info!(
            "Podcast text length is: {} characters. There is not need to split text into chunks.",
            char_count
        );
        let request = CreateSpeechRequestArgs::default()
            .input(&text)
            .voice(Voice::Onyx)
            .model(SpeechModel::Tts1Hd)
            .speed(1.3)
            .build()?;

        info!("Starting OpenAI voice generation...");
        let response = llm_client.audio().speech(request).await?;
        let audio_file_path = format!("{}/{}.mp3", user_tmp_dir, file_name);
        response.save(&audio_file_path).await?;

        info!("Podcast generated as single file");
        return Ok(PathBuf::from(audio_file_path));
    }

    let chunks = split_text_into_chunks(&text, MAX_TTS_CHARS);
    info!("Text split into {} chunks", chunks.len());

    let mut audio_parts = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        info!(
            "Processing podcast chunk: {}/{}, podcast length: {} chars",
            i + 1,
            chunks.len(),
            chunk.chars().count()
        );

        let request = CreateSpeechRequestArgs::default()
            .input(chunk)
            .voice(Voice::Onyx)
            .model(SpeechModel::Tts1Hd)
            .speed(1.3)
            .build()?;

        let response = llm_client.audio().speech(request).await?;
        let part_path = format!("{}/part_{}.mp3", user_tmp_dir, i);
        response.save(&part_path).await?;
        audio_parts.push(part_path);
    }

    let final_path = format!("{}/{}.mp3", user_tmp_dir, file_name);

    let mut command = Command::new("ffmpeg");
    command
        .arg("-i")
        .arg(format!("concat:{}", audio_parts.join("|")))
        .arg("-acodec")
        .arg("copy")
        .arg(&final_path);

    let status = command.status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to merge audio files"));
    }

    for part in audio_parts {
        if let Err(e) = fs::remove_file(&part) {
            warn!("Could not delete temporary file {}: {}", part, e);
        }
    }

    info!("Complete podcast successfully generated via OpenAI API");

    Ok(PathBuf::from(final_path))
}

pub async fn podcast_tts_via_openai_new<T: OpenAIClientInit + Send + Sync>(
    text: String,
    user_tmp_dir: String,
    app_state: Arc<T>,
) -> anyhow::Result<PathBuf> {
    info!("tarting recording podcast via OpenAI TTS model...");

    let now = Utc::now();
    let utc_plus_3 = now + Duration::hours(3);
    let date_only = utc_plus_3.date_naive();
    let file_name = format!("The_Viper_Podcast_({})", date_only);

    let llm_client = app_state.get_llm_client().clone();

    const MAX_TTS_CHARS: usize = 4095;

    let char_count = text.chars().count();

    if char_count <= MAX_TTS_CHARS {
        let char_count = text.chars().count();
        info!(
            "Podcast text length is: {} characters. There is not need to split text into chunks.",
            char_count
        );
        let request = CreateSpeechRequestArgs::default()
            .input(&text)
            .voice(Voice::Onyx)
            .model(SpeechModel::Other("gpt-4o-mini-tts".to_string()))
            .speed(1.3)
            .build()?;

        info!("Starting OpenAI voice generation...");
        let response = llm_client.audio().speech(request).await?;
        let audio_file_path = format!("{}/{}.mp3", user_tmp_dir, file_name);
        response.save(&audio_file_path).await?;

        info!("Podcast generated as single file");
        return Ok(PathBuf::from(audio_file_path));
    }

    let chunks = split_text_into_chunks(&text, MAX_TTS_CHARS);
    info!("Text split into {} chunks", chunks.len());

    let mut audio_parts = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        info!(
            "Processing podcast chunk: {}/{}, podcast length: {} chars",
            i + 1,
            chunks.len(),
            chunk.chars().count()
        );

        let request = CreateSpeechRequestArgs::default()
            .input(chunk)
            .voice(Voice::Onyx)
            .model(SpeechModel::Other("gpt-4o-mini-tts".to_string()))
            .speed(1.3)
            .build()?;

        let response = llm_client.audio().speech(request).await?;
        let part_path = format!("{}/part_{}.mp3", user_tmp_dir, i);
        response.save(&part_path).await?;
        audio_parts.push(part_path);
    }

    let final_path = format!("{}/{}.mp3", user_tmp_dir, file_name);

    let mut command = Command::new("ffmpeg");
    command
        .arg("-i")
        .arg(format!("concat:{}", audio_parts.join("|")))
        .arg("-acodec")
        .arg("copy")
        .arg(&final_path);

    let status = command.status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to merge audio files"));
    }

    for part in audio_parts {
        if let Err(e) = fs::remove_file(&part) {
            warn!("Could not delete temporary file {}: {}", part, e);
        }
    }

    info!("Complete podcast successfully generated via OpenAI API");

    Ok(PathBuf::from(final_path))
}

pub async fn simple_tts<T: OpenAIClientInit + Send + Sync>(
    text: &str,
    app_state: Arc<T>,
) -> anyhow::Result<CreateSpeechResponse> {
    let llm_client = app_state.get_llm_client().clone();

    let request = CreateSpeechRequestArgs::default()
        .input(text)
        .voice(Voice::Onyx)
        .model(SpeechModel::Tts1Hd)
        .speed(1.3)
        .build()?;

    let response = llm_client.audio().speech(request).await?;

    Ok(response)
}

pub async fn speech_to_text(file_path: &Path) -> anyhow::Result<String> {
    let start = Instant::now();

    if !file_path.exists() {
        return Err(anyhow!(
            "Voice message file not found: {}",
            file_path.display()
        ));
    }

    let model_path = std::env::var("WHISPER_MODEL_PATH")
        .unwrap_or_else(|_| "/root/projects/whisper.cpp/models/ggml-base.bin".to_string());

    let output = Command::new("whisper-cli")
        .arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(file_path)
        .arg("-l")
        .arg("ru")
        .arg("--no-timestamps")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            info!("Transcription took: {:?}", start.elapsed());

            let stdout = String::from_utf8(output.stdout)?;

            if stdout.trim().is_empty() {
                Ok("Empty text".to_string())
            } else {
                Ok(stdout)
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Whisper CLI failed: {}", stderr);
            Err(anyhow!("Whisper CLI failed: {}", stderr))
        }
        Err(err) => {
            error!("Failed to execute Whisper CLI: {}", err);
            Err(anyhow!("Failed to execute Whisper CLI: {}", err))
        }
    }
}

pub async fn podcast_tts_via_elevenlabs(
    text: String,
    user_tmp_dir: String,
) -> anyhow::Result<PathBuf> {
    info!("Starting ElevenLabs podcast recording via ElevenLabs TTS model...");

    let api_key = std::env::var("ELEVEN_LABS_API_KEY")
        .map_err(|_| anyhow::anyhow!("ELEVEN_LABS_API_KEY not found"))?;

    let now = Utc::now();
    let utc_plus_3 = now + Duration::hours(3);
    let date_only = utc_plus_3.date_naive();
    let file_name = format!("The_Viper_Podcast_({})", date_only);
    const ELEVEN_LABS_MAX_CHARS: usize = 9900;

    let char_count = text.chars().count();

    if char_count <= ELEVEN_LABS_MAX_CHARS {
        info!(
            "Podcast text length: {} chars. Generating single file.",
            char_count
        );

        let audio_data = generate_elevenlabs_speech(&text, &api_key).await?;
        let audio_file_path = format!("{}/{}.mp3", user_tmp_dir, file_name);
        fs::write(&audio_file_path, audio_data)?;

        info!("Podcast generated as single file");
        return Ok(PathBuf::from(audio_file_path));
    }

    let chunks = split_text_into_chunks(&text, ELEVEN_LABS_MAX_CHARS);
    info!("Text split into {} chunks", chunks.len());

    let mut audio_parts = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        info!(
            "Processing chunk {}/{}, length: {} chars",
            i + 1,
            chunks.len(),
            chunk.chars().count()
        );

        let audio_data = generate_elevenlabs_speech(chunk, &api_key).await?;
        let part_path = format!("{}/part_{}.mp3", user_tmp_dir, i);
        fs::write(&part_path, audio_data)?;
        audio_parts.push(part_path);

        if i < chunks.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    let final_path = format!("{}/{}.mp3", user_tmp_dir, file_name);

    let mut command = Command::new("ffmpeg");
    command
        .arg("-i")
        .arg(format!("concat:{}", audio_parts.join("|")))
        .arg("-acodec")
        .arg("copy")
        .arg(&final_path);

    let status = command.status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to merge audio files"));
    }

    for part in audio_parts {
        if let Err(e) = fs::remove_file(&part) {
            warn!("Could not delete temporary file {}: {}", part, e);
        }
    }

    info!("Complete podcast successfully generated via ElevenLabs API");
    Ok(PathBuf::from(final_path))
}

async fn generate_elevenlabs_speech(text: &str, api_key: &str) -> anyhow::Result<Vec<u8>> {
    let client = ReqwestClient::new();

    let voice_id = "vpUqfpCIn34tjFW4KHjt";

    let response = client
        .post(format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}",
            voice_id
        ))
        .header("xi-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&json!({
            "text": text,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.5,
                "style": 0.5,
                "speed": 1.05,
                "use_speaker_boost": true
            }
        }))
        .send()
        .await?
        .bytes()
        .await?
        .to_vec();

    Ok(response)
}
